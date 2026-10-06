//! User settings: planner rules (stored as JSON under the `rules` setting) and their catalog.
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

/// Built-in defaults (for per-rule reset in the UI).
#[tauri::command]
pub fn rules_defaults() -> Rules {
    Rules::default()
}

#[derive(serde::Serialize)]
pub struct RuleInfo {
    id: &'static str,
    title: &'static str,
    explain: &'static str,
    /// Planned ops in the last plan.
    hits: usize,
}

/// All planner rules with explanation and hit count of the last plan.
#[tauri::command]
pub async fn rules_catalog() -> CmdResult<Vec<RuleInfo>> {
    let (store, _) = crate::commands::open_store()?;
    let hits = store.rule_hits().map_err(err)?;
    Ok(rombro_core::rules::Rule::ALL
        .iter()
        .map(|r| RuleInfo {
            id: r.id(),
            title: r.title(),
            explain: r.explain(),
            hits: hits.get(r.id()).copied().unwrap_or(0),
        })
        .collect())
}
