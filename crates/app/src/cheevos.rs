//! RetroAchievements: Web API key, game lists and which library games have achievements
//! (Settings → RetroAchievements, library badge).

use crate::commands::{CmdResult, Progress, Throttle, err, open_store};
use romburak_core::cheevos;
use romburak_store::{RaGame, Store};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
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
        let (user, token) = romburak_store::ra_login(user.trim(), &password).map_err(err)?;
        let (mut store, _) = open_store()?;
        store.set_ra_account(Some((&user, &token))).map_err(err)?;
        crate::managed_ra::rewrite_config(&store)?;
        // the login stands even if the progress (needs the Web API key) fails
        let _ = progress(&mut store);
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
        let r = store
            .ra_sync(&key, now()?, &|done, total| {
                let _ = app.emit("cheevos://progress", ("sync", Progress { done, total }));
            })
            .map_err(err)?;
        if r.consoles == 0
            && let Some((_, e)) = r.failed.first()
        {
            return Err(format!("RetroAchievements: {e}"));
        }
        let n = hash_library(&app, &store)?;
        progress(&mut store)?;
        Ok(n)
    })
    .await
    .map_err(err)?
}

/// Downloads the logged-in user's progress (after a played game); returns it per RA game id,
/// empty without login or key.
#[tauri::command]
pub async fn cheevos_progress() -> CmdResult<HashMap<u64, romburak_store::RaProgress>> {
    tauri::async_runtime::spawn_blocking(move || {
        let (mut store, _) = open_store()?;
        progress(&mut store)?;
        store.ra_progress().map_err(err)
    })
    .await
    .map_err(err)?
}

fn progress(store: &mut Store) -> CmdResult<usize> {
    let key = store.setting(KEY).map_err(err)?.filter(|k| !k.is_empty());
    match (key, store.ra_account().map_err(err)?) {
        (Some(key), Some((user, _))) => store
            .ra_progress_sync(&key, &user)
            .map_err(|e| format!("RetroAchievements progress: {e}")),
        _ => Ok(0),
    }
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
pub async fn cheevos_achievements(game: u64) -> CmdResult<Vec<romburak_store::RaAchievement>> {
    tauri::async_runtime::spawn_blocking(move || {
        let (store, _) = open_store()?;
        let key = store.setting(KEY).map_err(err)?.ok_or("no Web API key")?;
        let user = store.ra_account().map_err(err)?.map(|(u, _)| u);
        let (players, list) =
            romburak_store::ra_achievements(&key, game, user.as_deref()).map_err(err)?;
        if players > 0 {
            store.set_ra_players(game, players, now()?).map_err(err)?;
        }
        Ok(list)
    })
    .await
    .map_err(err)?
}

fn now() -> CmdResult<i64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(err)?
        .as_secs() as i64)
}

static PLAYERS_RUNNING: AtomicBool = AtomicBool::new(false);
static PLAYERS_CANCEL: AtomicBool = AtomicBool::new(false);

/// Fetches missing or outdated player counts of the library's RA games in the background
/// (one request per game). Emits `cheevos://players` progress; returns all known player
/// counts per RA game id (`None` when nothing was fetched, also while another run is going).
#[tauri::command]
pub async fn cheevos_players(app: AppHandle) -> CmdResult<Option<HashMap<u64, u64>>> {
    if PLAYERS_RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(None);
    }
    PLAYERS_CANCEL.store(false, Ordering::SeqCst);
    let r = tauri::async_runtime::spawn_blocking(move || {
        let (store, _) = open_store()?;
        let Some(key) = store.setting(KEY).map_err(err)?.filter(|k| !k.is_empty()) else {
            return Ok(None);
        };
        let Some(library) = store.library().map_err(err)? else {
            return Ok(None);
        };
        let mut games: Vec<u64> = store
            .ra_file_games(&library)
            .map_err(err)?
            .into_values()
            .map(|g| g.id)
            .collect();
        games.sort_unstable();
        games.dedup();
        let now = now()?;
        let stale = store.ra_players_stale(&games, now).map_err(err)?;
        if stale.is_empty() {
            return Ok(None);
        }
        let r = store
            .ra_players_fetch(&key, &stale, now, &PLAYERS_CANCEL, &|done, total| {
                let _ = app.emit("cheevos://players", Progress { done, total });
            })
            .map_err(err)?;
        if r.fetched == 0 {
            return Ok(None);
        }
        store.ra_players().map(Some).map_err(err)
    })
    .await
    .map_err(err);
    PLAYERS_RUNNING.store(false, Ordering::SeqCst);
    r?
}

#[tauri::command]
pub fn cheevos_players_cancel() {
    PLAYERS_CANCEL.store(true, Ordering::SeqCst);
}

/// Library row data: achievements of this exact file, or the title of another version RA
/// supports.
pub(crate) struct Index {
    files: HashMap<PathBuf, RaGame>,
    titles: HashMap<(u32, String), RaGame>,
    progress: HashMap<u64, romburak_store::RaProgress>,
    players: HashMap<u64, u64>,
}

impl Index {
    pub(crate) fn load(store: &Store, library: &Path) -> CmdResult<Self> {
        Ok(Self {
            files: store.ra_file_games(library).map_err(err)?,
            titles: store.ra_titles().map_err(err)?,
            progress: store.ra_progress().map_err(err)?,
            players: store.ra_players().map_err(err)?,
        })
    }

    /// The user's unlocks in RA game `game`.
    pub(crate) fn progress(&self, game: u64) -> Option<romburak_store::RaProgress> {
        self.progress.get(&game).cloned()
    }

    /// Distinct RetroAchievements players of RA game `game`, once fetched.
    pub(crate) fn players(&self, game: u64) -> Option<u64> {
        self.players.get(&game).copied()
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
