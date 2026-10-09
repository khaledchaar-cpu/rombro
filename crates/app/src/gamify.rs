//! Gamification (SPEC F6): stats for the library rows the UI already holds; can be switched off.
use crate::commands::{CmdResult, err, open_store};
use romburak_store::Stats;

/// Setting: `"0"` hides gamification.
const KEY: &str = "gamification";

/// Computes KPIs/achievements for the identified games `(system, name)` and stores new unlocks.
#[tauri::command]
pub async fn gamify_stats(
    games: Vec<(String, String)>,
    unknown: u64,
    ambiguous: u64,
) -> CmdResult<Stats> {
    tauri::async_runtime::spawn_blocking(move || {
        let (store, _) = open_store()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(err)?
            .as_secs() as i64;
        store.gamify(&games, unknown, ambiguous, now).map_err(err)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub async fn gamify_enabled_get() -> CmdResult<bool> {
    let (store, _) = open_store()?;
    Ok(store.setting(KEY).map_err(err)?.as_deref() != Some("0"))
}

#[tauri::command]
pub async fn gamify_enabled_set(on: bool) -> CmdResult<()> {
    let (store, _) = open_store()?;
    store
        .set_setting(KEY, if on { "1" } else { "0" })
        .map_err(err)
}
