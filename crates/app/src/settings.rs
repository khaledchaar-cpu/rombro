//! User settings: 1G1R rules (stored as JSON under the `rules` setting).
use crate::commands::{CmdResult, err};
use rombro_core::g1r::Rules;
use rombro_store::Store;

pub(crate) fn load_rules(store: &Store) -> CmdResult<Rules> {
    store.rules().map_err(err)
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
    store.set_rules(&rules).map_err(err)?;
    Ok(rules)
}
