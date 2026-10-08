//! Starting games in the managed RetroArch and the core per game (Library → details).

use crate::commands::{CmdResult, Progress, err, open_store};
use crate::retroarch::CoreOption;
use rombro_core::retroarch::managed::launch::{self, Game, Step};
use rombro_core::retroarch::managed::{Managed, Phase};
use rombro_core::retroarch::pick;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::Emitter;

#[derive(Serialize)]
pub struct GameCores {
    system: Option<String>,
    /// Core the game starts with.
    chosen: Option<String>,
    /// Chosen for this game only.
    overridden: bool,
    options: Vec<CoreOption>,
}

fn managed() -> CmdResult<Managed> {
    Managed::detect().ok_or_else(|| "no RetroArch stable build for this platform".into())
}

/// Library file `path` (relative to the library, as listed) as absolute path plus library.
fn game_path(path: &str) -> CmdResult<(PathBuf, Option<PathBuf>)> {
    let (store, _) = open_store()?;
    let library = store.library().map_err(err)?;
    let p = Path::new(path);
    let abs = match &library {
        Some(l) if p.is_relative() => l.join(p),
        _ => p.to_path_buf(),
    };
    Ok((abs, library))
}

/// Cores that run the game's system (info files are fetched on first use; offline → none).
#[tauri::command]
pub async fn game_cores(path: String) -> CmdResult<GameCores> {
    tauri::async_runtime::spawn_blocking(move || {
        let m = managed()?;
        let (rom, library) = game_path(&path)?;
        let _ = m.ensure_info(&rombro_store::http_download, false);
        let (store, _) = open_store()?;
        let cores = m.cores();
        let system = launch::system_of(&rom, library.as_deref(), &cores);
        let over = store.core_override(&rom).map_err(err)?;
        let picks = store.rules().map_err(err)?.cores;
        let Some(sys) = &system else {
            return Ok(GameCores {
                system,
                chosen: None,
                overridden: false,
                options: vec![],
            });
        };
        let chosen =
            launch::core_for_game(&cores, sys, &picks, over.as_deref()).map(|c| c.id.clone());
        let rec = pick::recommended(sys);
        Ok(GameCores {
            overridden: over.is_some() && over == chosen,
            chosen,
            options: pick::options(&cores, sys)
                .into_iter()
                .map(|c| CoreOption {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    installed: c.installed,
                    recommended: Some(c.id.as_str()) == rec,
                })
                .collect(),
            system,
        })
    })
    .await
    .map_err(err)?
}

/// Sets (`Some`) or clears the core of a single game.
#[tauri::command]
pub async fn set_game_core(path: String, core: Option<String>) -> CmdResult<()> {
    let (rom, _) = game_path(&path)?;
    let (store, _) = open_store()?;
    store.set_core_override(&rom, core.as_deref()).map_err(err)
}

/// Installs what is missing and starts the game; returns the core id. Emits `play://progress`
/// (phase download/verify/unpack/info/core, MB, core id).
#[tauri::command]
pub async fn play(app: tauri::AppHandle, path: String) -> CmdResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let m = managed()?;
        let (rom, library) = game_path(&path)?;
        let (store, _) = open_store()?;
        let over = store.core_override(&rom).map_err(err)?;
        let picks = store.rules().map_err(err)?.cores;
        let game = Game {
            rom: &rom,
            library: library.as_deref().filter(|l| rom.starts_with(l)),
            picks: &picks,
            core: over.as_deref(),
        };
        let emit = |phase: &str, done: u64, total: u64, item: &str| {
            let mb = |b: u64| (b >> 20) as usize;
            let p = Progress {
                done: mb(done),
                total: mb(total),
            };
            let _ = app.emit("play://progress", (phase, p, item));
        };
        let mut l = m
            .prepare(&game, &rombro_store::http_download, &|s| match s {
                Step::RetroArch(Phase::Download { done, total }) => {
                    emit("download", done, total.unwrap_or(0), "")
                }
                Step::RetroArch(Phase::Verify) => emit("verify", 0, 0, ""),
                Step::RetroArch(Phase::Unpack { done, total }) => emit("unpack", done, total, ""),
                Step::RetroArch(Phase::Done) => {}
                Step::Info => emit("info", 0, 0, ""),
                Step::Core(id) => emit("core", 0, 0, &id),
            })
            .map_err(err)?;
        let mut child = l
            .command
            .spawn()
            .map_err(|e| format!("starting RetroArch: {e}"))?;
        // Reap the process; play time tracking (M15c) hooks in here.
        std::thread::spawn(move || child.wait());
        Ok(l.core.id)
    })
    .await
    .map_err(err)?
}
