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
    rombro_store::default_path().ok_or_else(|| "no data directory".to_owned())
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
    progress: &(dyn Fn(rombro_core::ScanTick<'_>) + Sync),
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

/// Minimum time between two progress events (keeps IPC cheap, stays live on big files).
const PROGRESS_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

/// Rate-limits progress callbacks; the first and last tick always pass.
pub(crate) struct Throttle(std::sync::Mutex<Option<Instant>>);

impl Throttle {
    pub(crate) fn new() -> Self {
        Self(std::sync::Mutex::new(None))
    }

    pub(crate) fn ready(&self, done: usize, total: usize) -> bool {
        let Ok(mut last) = self.0.lock() else {
            return false;
        };
        let due = last.is_none_or(|t| t.elapsed() >= PROGRESS_INTERVAL);
        if due || done == total {
            *last = Some(Instant::now());
        }
        due || done == total
    }
}

#[tauri::command]
pub async fn scan(app: AppHandle, dir: PathBuf) -> CmdResult<ScanSummary> {
    if !dir.exists() {
        return Err(format!("{} does not exist", dir.display()));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let t = Instant::now();
        let throttle = Throttle::new();
        let report = rombro_core::scan_with_progress(&dir, &|p| {
            if throttle.ready(p.done, p.total) {
                // Event delivery is best effort; a closed window is not an error.
                let _ = app.emit("scan://progress", p);
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

#[derive(Serialize)]
pub struct SyncSummary {
    dir: String,
    imported: usize,
    unchanged: usize,
    removed: usize,
    entries: u64,
    /// Arcade DATs newly loaded (SPEC F8).
    dats_updated: usize,
    /// DATs that could not be refreshed (offline, …).
    dat_warnings: Vec<String>,
}

/// Imports RetroArch RDBs from `dir` (or the auto-detected folder) and downloads the arcade
/// DATs; progress goes out as `sync://progress` (phase `rdb`/`dat`, done, total).
#[tauri::command]
pub async fn db_sync(app: AppHandle, dir: Option<PathBuf>) -> CmdResult<SyncSummary> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = match dir {
            Some(d) => d,
            None => rombro_core::paths::rdb_dir()
                .ok_or("RetroArch database folder not found – pick it manually")?,
        };
        let (mut store, _) = open_store()?;
        let emit = |phase: &'static str, done: usize, total: usize| {
            let _ = app.emit("sync://progress", (phase, Progress { done, total }));
        };
        let r = store
            .sync_rdbs_progress(&dir, &|d, t| emit("rdb", d, t))
            .map_err(err)?;
        let d = store
            .sync_dats_progress(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs() as i64),
                &|d, t| emit("dat", d, t),
            )
            .map_err(err)?;
        Ok(SyncSummary {
            dir: dir.display().to_string(),
            imported: r.imported,
            unchanged: r.unchanged,
            removed: r.removed,
            entries: r.entries as u64,
            dats_updated: d.updated.len(),
            dat_warnings: d.warnings,
        })
    })
    .await
    .map_err(err)?
}
