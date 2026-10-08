//! Import flow over IPC: plan (dry run) → execute → undo. The last plan is cached in app state so
//! `execute` runs exactly what the user reviewed.
use crate::commands::{
    CmdResult, Progress, Throttle, err, indexed_scan, library_snapshot, open_store,
};
use rombro_core::plan::{self, Decision, Mode, Op, Options};
use rombro_core::rules::Why;
use rombro_store::Store;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
pub struct Pending(Mutex<Option<(PathBuf, Vec<Op>)>>);

/// Inbox files the last plan leaves alone: (inbox, library, files).
#[derive(Default)]
pub struct Leftovers(Mutex<Option<(PathBuf, PathBuf, Vec<PathBuf>)>>);

#[derive(Serialize)]
pub struct LeftoverView {
    /// Relative to the inbox.
    path: String,
    size: u64,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ModeArg {
    Move,
    Copy,
}

impl ModeArg {
    fn as_str(self) -> &'static str {
        match self {
            ModeArg::Move => "move",
            ModeArg::Copy => "copy",
        }
    }
}

impl From<ModeArg> for Mode {
    fn from(m: ModeArg) -> Self {
        match m {
            ModeArg::Move => Mode::Move,
            ModeArg::Copy => Mode::Copy,
        }
    }
}

#[derive(Serialize)]
pub struct OpView {
    kind: &'static str,
    from: Option<String>,
    to: String,
    rule: &'static str,
    why: String,
}

#[derive(Serialize)]
pub struct DecisionView {
    kind: &'static str,
    path: String,
    /// Rejected: the core reason in a few words; empty otherwise.
    headline: String,
    /// System the decision is about (empty if unknown).
    system: String,
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
    /// Inbox files no operation touches (stay in the inbox).
    leftovers: Vec<LeftoverView>,
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
    // Announce the phase right away: walking a large tree takes a while before `total` is known.
    let _ = app.emit("import://progress", (phase, Progress { done: 0, total: 0 }));
    let throttle = Throttle::new();
    // the inbox is new material and checked file by file
    indexed_scan(store, dir, false, &|p| {
        if throttle.ready(p.done, p.total) {
            let _ = app.emit("import://progress", (phase, p));
        }
    })
}

#[tauri::command]
pub async fn plan_import(
    app: AppHandle,
    pending: State<'_, Pending>,
    leftover: State<'_, Leftovers>,
    inbox: Option<PathBuf>,
    library: PathBuf,
    mode: ModeArg,
) -> CmdResult<PlanView> {
    let library = std::path::absolute(&library).map_err(err)?;
    let lib = library.clone();
    let (p, items, left) = tauri::async_runtime::spawn_blocking(move || -> CmdResult<_> {
        let (store, _) = open_store()?;
        store.set_library(&lib).map_err(err)?;
        store.set_setting("mode", mode.as_str()).map_err(err)?;
        let mut items = Vec::new();
        let mut known = Default::default();
        if lib.is_dir() {
            let _ = app.emit(
                "import://progress",
                ("library", Progress { done: 0, total: 0 }),
            );
            let throttle = Throttle::new();
            let snap = library_snapshot(&store, &lib, false, &|p| {
                if throttle.ready(p.done, p.total) {
                    let _ = app.emit("import://progress", ("library", p));
                }
            })?;
            known = snap.sets.into_iter().collect();
            items = snap.items;
        }
        let mut inbox_root: Option<PathBuf> = None;
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
                    .items_with(&emit_scan(&store, &app, "inbox", &inbox)?, false, &known)
                    .map_err(err)?,
            );
            inbox_root = Some(inbox);
        }
        let _ = app.emit(
            "import://progress",
            ("planning", Progress { done: 0, total: 0 }),
        );
        let opts = Options {
            mode: mode.into(),
            rules: crate::settings::load_rules(&store)?,
            verdicts: store.verdicts().map_err(err)?,
            inbox: inbox_root.clone(),
            ignore: store.ignored().map_err(err)?,
        };
        let p = plan::build(&items, &lib, &opts);
        store.set_rule_hits(&p.why).map_err(err)?;
        let left = match &inbox_root {
            Some(i) => Some((i.clone(), plan::inbox::leftovers(i, &p).map_err(err)?)),
            None => None,
        };
        Ok((p, items.len(), left))
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
        leftovers: left
            .iter()
            .flat_map(|(inbox, files)| {
                files.iter().map(move |(f, size)| LeftoverView {
                    path: f
                        .strip_prefix(inbox)
                        .unwrap_or(f)
                        .to_string_lossy()
                        .into_owned(),
                    size: *size,
                })
            })
            .collect(),
    };
    *leftover.0.lock().map_err(err)? = left.map(|(inbox, files)| {
        let files = files.into_iter().map(|(f, _)| f).collect();
        (inbox, library.clone(), files)
    });
    *pending.0.lock().map_err(err)? = Some((library, p.ops));
    Ok(view)
}

#[tauri::command]
pub async fn execute_plan(app: AppHandle, pending: State<'_, Pending>) -> CmdResult<ExecResult> {
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
        let emit = |phase: &str, done, total| {
            let _ = app.emit("execute://progress", (phase, Progress { done, total }));
        };
        let throttle = Throttle::new();
        let ex = plan::execute_progress(&ops, &|done, total| {
            if throttle.ready(done, total) {
                emit("move", done, total);
            }
        });
        // indexing, pruning and the journal follow; tell the UI it is not stuck
        emit("finish", 0, 0);
        let journal = if ex.done.is_empty() {
            None
        } else {
            let (store, _) = open_store()?;
            store.index_executed(&ex.done).map_err(err)?;
            let inbox = store.setting("inbox").map_err(err)?.map(PathBuf::from);
            let roots: Vec<&std::path::Path> = [Some(library.as_path()), inbox.as_deref()]
                .into_iter()
                .flatten()
                .collect();
            plan::prune_emptied(&ex.done, &roots);
            if let Some(inbox) = &inbox {
                plan::inbox::prune_emptied_trees(&ex.done, inbox);
            }
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

/// Moves the inbox leftovers of the last plan to `<library>/_trash/inbox-<time>/`
/// (journaled: undo brings them back) and removes the emptied inbox folders.
#[tauri::command]
pub async fn inbox_clear(leftover: State<'_, Leftovers>) -> CmdResult<ExecResult> {
    let (inbox, library, files) = leftover
        .0
        .lock()
        .map_err(err)?
        .take()
        .ok_or("no inbox leftovers – plan an import first")?;
    tauri::async_runtime::spawn_blocking(move || {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(err)?
            .as_secs() as i64;
        let ops = plan::inbox::clear_ops(&inbox, &library, &ts.to_string(), &files);
        let ex = plan::execute(&ops);
        let journal = if ex.done.is_empty() {
            None
        } else {
            let (store, _) = open_store()?;
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
        plan::inbox::prune_empty_dirs(&inbox);
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

fn op_view(op: &Op, why: &Why) -> OpView {
    let kind = match op {
        Op::Move { .. } => "move",
        Op::Copy { .. } => "copy",
        Op::Extract { .. } => "extract",
        Op::Write { .. } => "write",
    };
    OpView {
        kind,
        from: op.source().map(s),
        to: s(op.target()),
        rule: why.rule.id(),
        why: why.to_string(),
    }
}

/// Plain-words 1G1R reason (`reason` is the Debug form of `g1r::Reason`).
fn reason_text(reason: &str, kept: bool) -> String {
    if let Some(what) = reason
        .strip_prefix("Excluded(\"")
        .and_then(|r| r.strip_suffix("\")"))
    {
        let none = if kept { "" } else { " (no other release)" };
        return format!("Filtered out as {what} – see Settings{none}");
    }
    match reason {
        "Duplicate" => "Same game as the kept file",
        "Region" => "Kept release has a preferred region",
        "Language" => "Kept release has a preferred language",
        "Variant" => "Alternative version (re-release, alt dump) – original kept",
        "Revision" => "Newer revision kept",
        "TieBreak" => "Equal releases – one kept",
        r => r,
    }
    .to_owned()
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
            ..
        } => (
            "rejected",
            s(path),
            match kept {
                Some(k) => format!("{name}  →  kept: {k}"),
                None => format!("{name}  →  no release kept"),
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
    let (headline, system) = match d {
        Decision::Rejected {
            system,
            reason,
            kept,
            ..
        } => (reason_text(reason, kept.is_some()), system.clone()),
        Decision::Tie { system, .. } => (String::new(), system.clone()),
        _ => (String::new(), String::new()),
    };
    DecisionView {
        kind,
        path,
        headline,
        system,
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
