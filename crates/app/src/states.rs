//! Savestates of a game (Library → details): list, delete; starting from one is `play` with a slot.

use crate::commands::{CmdResult, err};
use crate::play::{game_path, managed};
use romburak_core::retroarch::states;
use serde::Serialize;

#[derive(Serialize)]
pub struct StateView {
    path: String,
    /// Core folder name (`corename`) and its id, if the core is known.
    core: String,
    core_id: Option<String>,
    /// `None` = auto state (loaded by RetroArch itself, not startable by slot).
    slot: Option<u32>,
    /// unix seconds
    modified: i64,
    /// Screenshot path for `thumb://localhost/state/<path>`.
    screenshot: Option<String>,
}

#[tauri::command]
pub async fn save_states(path: String) -> CmdResult<Vec<StateView>> {
    tauri::async_runtime::spawn_blocking(move || {
        let m = managed()?;
        let (rom, _) = game_path(&path)?;
        let cores = m.cores();
        Ok(states::list(&m.states_dir(), &rom)
            .into_iter()
            .map(|s| StateView {
                path: s.path.to_string_lossy().into_owned(),
                core_id: cores
                    .iter()
                    .find(|c| c.name == s.core)
                    .map(|c| c.id.clone()),
                core: s.core,
                slot: s.slot,
                modified: s
                    .modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs() as i64),
                screenshot: s.screenshot.map(|p| p.to_string_lossy().into_owned()),
            })
            .collect())
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub async fn delete_save_state(path: String) -> CmdResult<()> {
    let m = managed()?;
    states::delete(&m.states_dir(), std::path::Path::new(&path)).map_err(err)
}

/// PNG of a savestate screenshot, only from the managed states folder.
pub fn screenshot(path: &str) -> Option<Vec<u8>> {
    let dir = managed().ok()?.states_dir();
    let p = std::path::Path::new(path);
    let ok = p.starts_with(&dir)
        && p.extension().is_some_and(|e| e == "png")
        && !p.components().any(|c| c == std::path::Component::ParentDir);
    ok.then(|| std::fs::read(p).ok()).flatten()
}
