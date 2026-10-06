//! TBD queue over IPC: resolve ambiguous matches and record verdicts on 1G1R-rejected releases.
//! Both are persisted in the store and take effect on the next plan.
use crate::commands::{CmdResult, err, open_store};
use rombro_core::plan::{Verdict, trash};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum VerdictArg {
    Keep,
    Discard,
    Prefer,
}

/// Stores the user's pick (system, name) for the ambiguous file at `path`.
#[tauri::command]
pub async fn resolve_ambiguous(path: PathBuf, system: String, name: String) -> CmdResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let report = rombro_core::scan(&path);
        let sha1 = match report.discs.first() {
            Some(d) => d.tracks.first().map(|t| t.hashes.sha1),
            None => report.roms.first().map(|r| r.hashes.sha1),
        }
        .ok_or_else(|| format!("no ROM or disc found at {}", path.display()))?;
        let (store, _) = open_store()?;
        store.set_resolution(&sha1, &system, &name).map_err(err)
    })
    .await
    .map_err(err)?
}

/// Records (or with `None` clears) the verdict on a release 1G1R rejected.
#[tauri::command]
pub async fn set_verdict(
    system: String,
    name: String,
    verdict: Option<VerdictArg>,
    reason: Option<String>,
) -> CmdResult<()> {
    let v = verdict.map(|v| match v {
        VerdictArg::Keep => Verdict::Keep,
        VerdictArg::Discard => Verdict::Discard,
        VerdictArg::Prefer => Verdict::Prefer,
    });
    let (store, _) = open_store()?;
    store
        .set_verdict(&system, &name, v, reason.as_deref().unwrap_or_default())
        .map_err(err)
}

#[derive(Serialize)]
pub struct TrashFile {
    /// Relative to the trash folder.
    path: String,
    bytes: u64,
}

/// Contents of `<library>/_trash`.
#[tauri::command]
pub async fn trash_list(library: PathBuf) -> CmdResult<Vec<TrashFile>> {
    let base = library.join(rombro_core::plan::TRASH_DIR);
    let files = trash::list(&library).map_err(err)?;
    Ok(files
        .into_iter()
        .map(|(p, bytes)| TrashFile {
            path: p.strip_prefix(&base).unwrap_or(&p).display().to_string(),
            bytes,
        })
        .collect())
}

/// Permanently deletes `<library>/_trash`; returns the number of deleted files.
#[tauri::command]
pub async fn trash_empty(library: PathBuf) -> CmdResult<usize> {
    tauri::async_runtime::spawn_blocking(move || trash::empty(&library).map_err(err))
        .await
        .map_err(err)?
}
