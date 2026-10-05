//! User settings: 1G1R rules (stored as JSON under the `rules` setting).
use crate::commands::{CmdResult, err};
use rombro_core::g1r::Rules;
use rombro_store::Store;

const RULES_KEY: &str = "rules";

/// Stored rules, or the defaults when none (or unreadable ones) are stored.
pub(crate) fn load_rules(store: &Store) -> CmdResult<Rules> {
    Ok(store
        .setting(RULES_KEY)
        .map_err(err)?
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default())
}

#[tauri::command]
pub async fn rules_get() -> CmdResult<Rules> {
    let (store, _) = crate::commands::open_store()?;
    load_rules(&store)
}

/// Saves `rules`; `None` resets to the defaults. Returns the effective rules.
#[tauri::command]
pub async fn rules_set(rules: Option<Rules>) -> CmdResult<Rules> {
    let (store, _) = crate::commands::open_store()?;
    let rules = rules.unwrap_or_default();
    let json = serde_json::to_string(&rules).map_err(err)?;
    store.set_setting(RULES_KEY, &json).map_err(err)?;
    Ok(rules)
}
