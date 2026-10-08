//! RetroAchievements: Web API key, game lists and which library games have achievements
//! (Settings → RetroAchievements, library badge).

use crate::commands::{CmdResult, Progress, Throttle, err, open_store};
use rombro_core::cheevos;
use rombro_store::{RaGame, Store};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

/// Setting holding the user's Web API key (shared with the CLI).
const KEY: &str = "ra.api_key";

#[derive(Serialize)]
pub struct Status {
    has_key: bool,
    /// Logged-in RetroAchievements user (for RetroArch).
    user: Option<String>,
    hardcore: bool,
    /// Unix seconds of the last game list sync.
    synced: Option<i64>,
}

#[tauri::command]
pub async fn cheevos_status() -> CmdResult<Status> {
    let (store, _) = open_store()?;
    Ok(Status {
        has_key: store
            .setting(KEY)
            .map_err(err)?
            .is_some_and(|k| !k.is_empty()),
        user: store.ra_account().map_err(err)?.map(|(u, _)| u),
        hardcore: store.ra_hardcore().map_err(err)?,
        synced: store.ra_synced().map_err(err)?,
    })
}

#[tauri::command]
pub async fn cheevos_set_key(key: String) -> CmdResult<()> {
    let (store, _) = open_store()?;
    store.set_setting(KEY, key.trim()).map_err(err)
}

/// Logs in (the password is only sent, never stored) and writes the token into the
/// RetroArch config. Returns the user name as RetroAchievements spells it.
#[tauri::command]
pub async fn cheevos_login(user: String, password: String) -> CmdResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (user, token) = rombro_store::ra_login(user.trim(), &password).map_err(err)?;
        let (store, _) = open_store()?;
        store.set_ra_account(Some((&user, &token))).map_err(err)?;
        crate::managed_ra::rewrite_config(&store)?;
        Ok(user)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub fn cheevos_logout() -> CmdResult<()> {
    let (store, _) = open_store()?;
    store.set_ra_account(None).map_err(err)?;
    crate::managed_ra::rewrite_config(&store)
}

#[tauri::command]
pub fn cheevos_set_hardcore(on: bool) -> CmdResult<()> {
    let (store, _) = open_store()?;
    store.set_ra_hardcore(on).map_err(err)?;
    crate::managed_ra::rewrite_config(&store)
}

/// Downloads the game lists, then hashes the library. Emits `cheevos://progress`
/// (phase `sync`/`hash`, progress). Returns the number of library games with achievements.
#[tauri::command]
pub async fn cheevos_sync(app: AppHandle) -> CmdResult<usize> {
    tauri::async_runtime::spawn_blocking(move || {
        let (mut store, _) = open_store()?;
        let key = store
            .setting(KEY)
            .map_err(err)?
            .filter(|k| !k.is_empty())
            .ok_or("no Web API key")?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(err)?
            .as_secs() as i64;
        let r = store
            .ra_sync(&key, now, &|done, total| {
                let _ = app.emit("cheevos://progress", ("sync", Progress { done, total }));
            })
            .map_err(err)?;
        if r.consoles == 0
            && let Some((_, e)) = r.failed.first()
        {
            return Err(format!("RetroAchievements: {e}"));
        }
        hash_library(&app, &store)
    })
    .await
    .map_err(err)?
}

/// Hashes library files missing from the cache (after imports); same events as the sync.
#[tauri::command]
pub async fn cheevos_hash(app: AppHandle) -> CmdResult<usize> {
    tauri::async_runtime::spawn_blocking(move || {
        let (store, _) = open_store()?;
        hash_library(&app, &store)
    })
    .await
    .map_err(err)?
}

fn hash_library(app: &AppHandle, store: &Store) -> CmdResult<usize> {
    let library = store
        .library()
        .map_err(err)?
        .ok_or("no library configured")?;
    let files = cheevos::library_files(&library).map_err(err)?;
    let throttle = Throttle::new();
    let total = files.len();
    for (i, (path, console)) in files.iter().enumerate() {
        // unreadable files just stay without a badge
        let _ = store.ra_hash(path, console.method);
        if throttle.ready(i + 1, total) {
            let _ = app.emit(
                "cheevos://progress",
                ("hash", Progress { done: i + 1, total }),
            );
        }
    }
    Ok(store.ra_file_games(&library).map_err(err)?.len())
}

/// Achievements of RA game `game` (fetched on demand for the details view).
#[tauri::command]
pub async fn cheevos_achievements(game: u64) -> CmdResult<Vec<rombro_store::RaAchievement>> {
    tauri::async_runtime::spawn_blocking(move || {
        let (store, _) = open_store()?;
        let key = store.setting(KEY).map_err(err)?.ok_or("no Web API key")?;
        rombro_store::ra_achievements(&key, game).map_err(err)
    })
    .await
    .map_err(err)?
}

/// Library row data: achievements of this exact file, or the title of another version RA
/// supports.
pub(crate) struct Index {
    files: HashMap<PathBuf, RaGame>,
    titles: HashMap<(u32, String), RaGame>,
}

impl Index {
    pub(crate) fn load(store: &Store, library: &Path) -> CmdResult<Self> {
        Ok(Self {
            files: store.ra_file_games(library).map_err(err)?,
            titles: store.ra_titles().map_err(err)?,
        })
    }

    /// (achievements of this file, RA title of another supported version, RA game id).
    pub(crate) fn lookup(
        &self,
        path: &Path,
        system: &str,
        name: &str,
    ) -> (u32, Option<String>, Option<u64>) {
        if let Some(g) = self.files.get(path) {
            return (g.achievements, None, Some(g.id));
        }
        // arcade sets are hashed by name: another version is another set, never a hint
        let other = cheevos::console(system)
            .filter(|c| c.method != cheevos::Method::Arcade)
            .and_then(|c| self.titles.get(&(c.id, cheevos::title_key(name))))
            .map(|g| (g.title.clone(), g.id));
        (0, other.as_ref().map(|o| o.0.clone()), other.map(|o| o.1))
    }
}
