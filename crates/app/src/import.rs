//! Import flow over IPC: plan (dry run) → execute → undo. The last plan is cached in app state so
//! `execute` runs exactly what the user reviewed.
use crate::commands::{CmdResult, PROGRESS_STEP, Progress, err, indexed_scan, open_store};
use rombro_core::plan::{self, Decision, Mode, Op, Options, PLAYLIST_DIR};
use rombro_store::Store;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
pub struct Pending(Mutex<Option<(PathBuf, Vec<Op>)>>);

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ModeArg {
    Move,
    Copy,
    Hardlink,
    Reflink,
}

impl ModeArg {
    fn as_str(self) -> &'static str {
        match self {
            ModeArg::Move => "move",
            ModeArg::Copy => "copy",
            ModeArg::Hardlink => "hardlink",
            ModeArg::Reflink => "reflink",
        }
    }
}

impl From<ModeArg> for Mode {
    fn from(m: ModeArg) -> Self {
        match m {
            ModeArg::Move => Mode::Move,
            ModeArg::Copy => Mode::Copy,
            ModeArg::Hardlink => Mode::Hardlink,
            ModeArg::Reflink => Mode::Reflink,
        }
    }
}

#[derive(Serialize)]
pub struct OpView {
    kind: &'static str,
    from: Option<String>,
    to: String,
    why: String,
}

#[derive(Serialize)]
pub struct DecisionView {
    kind: &'static str,
    path: String,
    detail: String,
    /// Ambiguous: candidates; tie: releases; rejected: the release itself.
    options: Vec<Choice>,
    /// Rejected only: keeping is possible (not a duplicate of the pick).
    can_keep: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Choice {
    system: String,
    name: String,
}

fn choice(system: &str, name: &str) -> Choice {
    Choice {
        system: system.to_owned(),
        name: name.to_owned(),
    }
}

#[derive(Serialize)]
pub struct PlanView {
    items: usize,
    placed: usize,
    unchanged: usize,
    quarantined: usize,
    discarded: usize,
    ops: Vec<OpView>,
    decisions: Vec<DecisionView>,
}

#[derive(Serialize)]
pub struct ExecResult {
    done: usize,
    journal: Option<i64>,
    error: Option<String>,
}

fn emit_scan(
    store: &Store,
    app: &AppHandle,
    phase: &'static str,
    dir: &Path,
) -> CmdResult<rombro_core::ScanReport> {
    indexed_scan(store, dir, &|done, total| {
        if done % PROGRESS_STEP == 0 || done == total {
            let _ = app.emit("import://progress", (phase, Progress { done, total }));
        }
    })
}

#[tauri::command]
pub async fn plan_import(
    app: AppHandle,
    pending: State<'_, Pending>,
    inbox: Option<PathBuf>,
    library: PathBuf,
    mode: ModeArg,
) -> CmdResult<PlanView> {
    let library = std::path::absolute(&library).map_err(err)?;
    let lib = library.clone();
    let (p, items) = tauri::async_runtime::spawn_blocking(move || -> CmdResult<_> {
        let (store, _) = open_store()?;
        store.set_library(&lib).map_err(err)?;
        store.set_setting("mode", mode.as_str()).map_err(err)?;
        let mut items = Vec::new();
        if lib.is_dir() {
            items = store
                .items(&emit_scan(&store, &app, "library", &lib)?, true)
                .map_err(err)?;
        }
        if let Some(inbox) = inbox {
            let inbox = std::path::absolute(&inbox).map_err(err)?;
            if !inbox.is_dir() {
                return Err(format!("{} is not a directory", inbox.display()));
            }
            store
                .set_setting("inbox", &inbox.to_string_lossy())
                .map_err(err)?;
            items.extend(
                store
                    .items(&emit_scan(&store, &app, "inbox", &inbox)?, false)
                    .map_err(err)?,
            );
        }
        let opts = Options {
            mode: mode.into(),
            rules: crate::settings::load_rules(&store)?,
            playlists: Some(lib.join(PLAYLIST_DIR)),
            verdicts: store.verdicts().map_err(err)?,
        };
        Ok((plan::build(&items, &lib, &opts), items.len()))
    })
    .await
    .map_err(err)??;
    let view = PlanView {
        items,
        placed: p.placed,
        unchanged: p.unchanged,
        quarantined: p.quarantined,
        discarded: p.discarded,
        ops: p
            .ops
            .iter()
            .zip(&p.why)
            .map(|(o, w)| op_view(o, w))
            .collect(),
        decisions: p.decisions.iter().map(decision_view).collect(),
    };
    *pending.0.lock().map_err(err)? = Some((library, p.ops));
    Ok(view)
}

#[tauri::command]
pub async fn execute_plan(pending: State<'_, Pending>) -> CmdResult<ExecResult> {
    let (library, ops) = pending
        .0
        .lock()
        .map_err(err)?
        .take()
        .ok_or("no plan to execute")?;
    tauri::async_runtime::spawn_blocking(move || {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(err)?
            .as_secs() as i64;
        let ex = plan::execute(&ops);
        let journal = if ex.done.is_empty() {
            None
        } else {
            let (store, _) = open_store()?;
            store.index_executed(&ex.done).map_err(err)?;
            Some(
                store
                    .add_journal(
                        ts,
                        &library.to_string_lossy(),
                        &plan::journal_to_json(&ex.done),
                    )
                    .map_err(err)?,
            )
        };
        Ok(ExecResult {
            done: ex.done.len(),
            journal,
            error: ex
                .error
                .map(|(op, e)| format!("{}: {e}", op.target().display())),
        })
    })
    .await
    .map_err(err)?
}

/// Reverts the most recent execution; returns the number of reverted operations (0 = nothing to undo).
#[tauri::command]
pub async fn undo_last() -> CmdResult<usize> {
    tauri::async_runtime::spawn_blocking(|| {
        let (store, _) = open_store()?;
        let Some(j) = store.last_journal().map_err(err)? else {
            return Ok(0);
        };
        let done = plan::journal_from_json(&j.done).map_err(err)?;
        let errors = plan::undo(&done);
        if let Some((op, e)) = errors.first() {
            return Err(format!(
                "{} operations could not be reverted (first: {}: {e})",
                errors.len(),
                op.target().display()
            ));
        }
        store.index_undone(&done).map_err(err)?;
        store.mark_undone(j.id).map_err(err)?;
        Ok(done.len())
    })
    .await
    .map_err(err)?
}

fn s(p: &Path) -> String {
    p.display().to_string()
}

fn op_view(op: &Op, why: &str) -> OpView {
    let kind = match op {
        Op::Move { .. } => "move",
        Op::Copy { .. } => "copy",
        Op::Hardlink { .. } => "link",
        Op::Reflink { .. } => "clone",
        Op::Extract { .. } => "extract",
        Op::Write { .. } => "write",
    };
    OpView {
        kind,
        from: op.source().map(s),
        to: s(op.target()),
        why: why.to_owned(),
    }
}

fn decision_view(d: &Decision) -> DecisionView {
    let can_keep = matches!(d, Decision::Rejected { reason, .. } if reason != "Duplicate");
    let (kind, path, detail, options) = match d {
        Decision::Ambiguous { path, candidates } => (
            "ambiguous",
            s(path),
            String::new(),
            candidates
                .iter()
                .map(|g| choice(&g.system, &g.name))
                .collect(),
        ),
        Decision::Tie { system, releases } => (
            "tie",
            system.clone(),
            String::new(),
            releases.iter().map(|r| choice(system, r)).collect(),
        ),
        Decision::Rejected {
            path,
            system,
            name,
            kept,
            reason,
        } => (
            "rejected",
            s(path),
            match kept {
                Some(k) => format!("{name}: {reason}; kept {k}"),
                None => format!("{name}: excluded ({reason})"),
            },
            vec![choice(system, name)],
        ),
        Decision::Skipped { path, reason } => ("skipped", s(path), reason.clone(), Vec::new()),
        Decision::Conflict { path, target } => (
            "conflict",
            s(path),
            format!("target exists: {}", target.display()),
            Vec::new(),
        ),
    };
    DecisionView {
        kind,
        path,
        detail,
        options,
        can_keep,
    }
}

#[derive(Serialize)]
pub struct JournalView {
    id: i64,
    ts: i64,
    library: String,
    state: String,
    ops: usize,
}

/// Recent executions for the Dashboard, newest first.
#[tauri::command]
pub async fn journal_list() -> CmdResult<Vec<JournalView>> {
    tauri::async_runtime::spawn_blocking(|| {
        let (store, _) = open_store()?;
        let list = store.journals(20).map_err(err)?;
        Ok(list
            .into_iter()
            .map(|(j, state)| JournalView {
                ops: plan::journal_from_json(&j.done).map_or(0, |d| d.len()),
                id: j.id,
                ts: j.ts,
                library: j.library,
                state,
            })
            .collect())
    })
    .await
    .map_err(err)?
}
