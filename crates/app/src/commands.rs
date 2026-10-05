//! IPC commands. Errors cross the boundary as strings.
use rombro_store::Store;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

pub(crate) type CmdResult<T> = Result<T, String>;

pub(crate) fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn db_path() -> CmdResult<PathBuf> {
    rombro_store::default_path().ok_or_else(|| "HOME not set".to_owned())
}

pub(crate) fn open_store() -> CmdResult<(Store, PathBuf)> {
    let path = db_path()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    Ok((Store::open(&path).map_err(err)?, path))
}

/// Scans `root` incrementally via the persistent file index and updates the index.
pub(crate) fn indexed_scan(
    store: &Store,
    root: &std::path::Path,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> CmdResult<rombro_core::ScanReport> {
    let cache = store.hash_cache(root).map_err(err)?;
    let report = rombro_core::scan_cached(root, &cache, progress);
    store.save_scan(root, &report).map_err(err)?;
    Ok(report)
}

#[derive(Serialize)]
pub struct SystemCount {
    system: String,
    count: u64,
}

#[derive(Serialize)]
pub struct DbStats {
    db_path: String,
    entries: u64,
    systems: Vec<SystemCount>,
}

#[tauri::command]
pub async fn db_stats() -> CmdResult<DbStats> {
    tauri::async_runtime::spawn_blocking(|| {
        let (store, path) = open_store()?;
        let systems: Vec<_> = store
            .system_counts()
            .map_err(err)?
            .into_iter()
            .map(|(system, count)| SystemCount { system, count })
            .collect();
        Ok(DbStats {
            db_path: path.display().to_string(),
            entries: systems.iter().map(|s| s.count).sum(),
            systems,
        })
    })
    .await
    .map_err(err)?
}

#[derive(Serialize, Clone, Copy)]
pub(crate) struct Progress {
    pub done: usize,
    pub total: usize,
}

#[derive(Serialize)]
pub struct ScanSummary {
    roms: usize,
    discs: usize,
    playlists: usize,
    failures: usize,
    bytes: u64,
    millis: u128,
}

/// Minimum number of files between two progress events (keeps IPC cheap).
pub(crate) const PROGRESS_STEP: usize = 64;

#[tauri::command]
pub async fn scan(app: AppHandle, dir: PathBuf) -> CmdResult<ScanSummary> {
    if !dir.exists() {
        return Err(format!("{} does not exist", dir.display()));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let t = Instant::now();
        let report = rombro_core::scan_with_progress(&dir, &|done, total| {
            if done % PROGRESS_STEP == 0 || done == total {
                // Event delivery is best effort; a closed window is not an error.
                let _ = app.emit("scan://progress", Progress { done, total });
            }
        });
        let bytes = report
            .roms
            .iter()
            .chain(report.discs.iter().flat_map(|d| &d.tracks))
            .map(|r| r.hashes.size)
            .sum();
        ScanSummary {
            roms: report.roms.len(),
            discs: report.discs.len(),
            playlists: report.playlists.len(),
            failures: report.failures.len(),
            bytes,
            millis: t.elapsed().as_millis(),
        }
    })
    .await
    .map_err(err)
}
