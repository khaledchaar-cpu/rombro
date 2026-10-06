//! Export of playlists and BIOS files to RetroArch (preview, then execute with journal).

use crate::commands::{CmdResult, err, open_store};
use rombro_core::plan;
use rombro_core::retroarch::{Dirs, export, firmware, info};
use serde::Serialize;

#[derive(Serialize)]
pub struct PlaylistView {
    system: String,
    /// Installed core set as default; `None` = RetroArch asks.
    core: Option<String>,
}

#[derive(Serialize)]
pub struct MissingView {
    system: String,
    path: String,
}

#[derive(Serialize)]
pub struct RetroArchView {
    playlist_dir: String,
    system_dir: String,
    cores: usize,
    playlists: Vec<PlaylistView>,
    playlists_unchanged: usize,
    bios_copied: Vec<String>,
    bios_present: usize,
    bios_conflicts: Vec<String>,
    bios_missing: Vec<MissingView>,
    /// Operations executed (`None` for a preview).
    executed: Option<usize>,
}

/// Plans (and unless `dry_run` executes) the export for the stored library.
#[tauri::command]
pub async fn retroarch_export(dry_run: bool) -> CmdResult<RetroArchView> {
    tauri::async_runtime::spawn_blocking(move || {
        let (store, _) = open_store()?;
        let library = store
            .library()
            .map_err(err)?
            .ok_or("no library set – run an import first")?;
        let dirs = Dirs::detect().ok_or("no retroarch.cfg found")?;
        let report = crate::commands::indexed_scan(&store, &library, &|_, _| {})?;
        let cores = info::installed(&dirs.info, &dirs.cores);
        let ex = export::plan(
            &library,
            &dirs,
            &cores,
            &firmware::bundled(),
            &export::files_by_sha1(&report),
            true,
            true,
        );
        let mut executed = None;
        if !dry_run && !ex.ops.is_empty() {
            let r = plan::execute(&ex.ops);
            if !r.done.is_empty() {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(err)?
                    .as_secs() as i64;
                store
                    .add_journal(
                        ts,
                        &library.to_string_lossy(),
                        &plan::journal_to_json(&r.done),
                    )
                    .map_err(err)?;
            }
            if let Some((op, e)) = r.error {
                return Err(format!("stopped at {}: {e}", op.target().display()));
            }
            executed = Some(r.done.len());
        }
        Ok(RetroArchView {
            playlist_dir: dirs.playlists.display().to_string(),
            system_dir: dirs.system.display().to_string(),
            cores: cores.len(),
            playlists: ex
                .playlists
                .into_iter()
                .map(|(system, core)| PlaylistView { system, core })
                .collect(),
            playlists_unchanged: ex.playlists_unchanged,
            bios_copied: ex.bios_copied,
            bios_present: ex.bios_present,
            bios_conflicts: ex.bios_conflicts,
            bios_missing: ex
                .bios_missing
                .into_iter()
                .map(|(system, path)| MissingView { system, path })
                .collect(),
            executed,
        })
    })
    .await
    .map_err(err)?
}
