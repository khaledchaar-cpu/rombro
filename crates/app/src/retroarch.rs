//! The core choice per system (managed RetroArch).

use crate::commands::{CmdResult, err, open_store};
use romburak_core::retroarch::managed::Managed;
use romburak_core::retroarch::pick;
use serde::Serialize;

#[derive(Serialize)]
pub struct CoreOption {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) installed: bool,
    pub(crate) recommended: bool,
}

#[derive(Serialize)]
pub struct SystemCores {
    system: String,
    /// Core the playlist gets (installed now or after installing); `None` = none known.
    chosen: Option<String>,
    /// Chosen by the user (stored in the rules) rather than recommended.
    picked: bool,
    options: Vec<CoreOption>,
}

/// The cores to choose from for each system of the stored library (managed RetroArch;
/// info files are fetched on first use).
#[tauri::command]
pub async fn retroarch_cores() -> CmdResult<Vec<SystemCores>> {
    tauri::async_runtime::spawn_blocking(move || {
        let (store, _) = open_store()?;
        let library = store.library().map_err(err)?.ok_or("no library set")?;
        let m = Managed::detect().ok_or("no RetroArch stable build for this platform")?;
        let _ = m.ensure_info(&romburak_store::http_download, false);
        let picks = store.rules().map_err(err)?.cores;
        let cores = m.cores();
        Ok(pick::library_systems(&library)
            .into_iter()
            .map(|system| {
                let rec = pick::recommended(&system);
                SystemCores {
                    chosen: pick::resolve(&cores, &system, &picks, true).map(|c| c.id.clone()),
                    picked: picks.contains_key(&system),
                    options: pick::options(&cores, &system)
                        .into_iter()
                        .map(|c| CoreOption {
                            id: c.id.clone(),
                            name: c.name.clone(),
                            installed: c.installed,
                            recommended: Some(c.id.as_str()) == rec,
                        })
                        .collect(),
                    system,
                }
            })
            .collect())
    })
    .await
    .map_err(err)?
}
