//! Export of playlists, BIOS files and missing cores to RetroArch (preview, then execute
//! with journal), and the core choice per system.

use crate::commands::{CmdResult, err, open_store};
use rombro_core::plan;
use rombro_core::retroarch::{Dirs, export, firmware, info, pick};
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
pub struct CoreOption {
    id: String,
    name: String,
    installed: bool,
    recommended: bool,
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

#[derive(Serialize)]
pub struct RetroArchView {
    playlist_dir: String,
    system_dir: String,
    cores: usize,
    /// Cores to install (id).
    cores_install: Vec<String>,
    /// Wanted cores that are missing and not to be installed.
    cores_missing: Vec<MissingView>,
    /// RetroArch knows where to download cores from.
    can_install: bool,
    playlists: Vec<PlaylistView>,
    playlists_unchanged: usize,
    playlists_removed: Vec<String>,
    playlists_no_core: Vec<String>,
    bios_copied: Vec<String>,
    bios_present: usize,
    bios_conflicts: Vec<String>,
    bios_missing: Vec<MissingView>,
    /// ScummVM targets added to or updated in `scummvm.ini`.
    scummvm_targets: Vec<String>,
    /// Operations executed (`None` for a preview).
    executed: Option<usize>,
}

/// The cores to choose from for each playlist of the stored library.
#[tauri::command]
pub async fn retroarch_cores() -> CmdResult<Vec<SystemCores>> {
    tauri::async_runtime::spawn_blocking(move || {
        let (store, _) = open_store()?;
        let library = store.library().map_err(err)?.ok_or("no library set")?;
        let dirs = Dirs::detect().ok_or("no retroarch.cfg found")?;
        let picks = store.rules().map_err(err)?.cores;
        let cores = info::available(&dirs.info, &dirs.cores);
        Ok(export::playlist_systems(&library)
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

/// Plans (and unless `dry_run` executes) the export for the stored library; with
/// `install_cores` missing cores are downloaded from RetroArch's buildbot.
#[tauri::command]
pub async fn retroarch_export(
    app: tauri::AppHandle,
    dry_run: bool,
    install_cores: bool,
) -> CmdResult<RetroArchView> {
    use tauri::Emitter;
    // `retroarch://progress`: (phase scan/download/write, progress, current item)
    let emit = move |phase: &'static str, done: usize, total: usize, item: &str| {
        let _ = app.emit(
            "retroarch://progress",
            (phase, crate::commands::Progress { done, total }, item),
        );
    };
    tauri::async_runtime::spawn_blocking(move || {
        let (store, _) = open_store()?;
        let library = store
            .library()
            .map_err(err)?
            .ok_or("no library set – run an import first")?;
        let dirs = Dirs::detect().ok_or("no retroarch.cfg found")?;
        let throttle = crate::commands::Throttle::new();
        emit("scan", 0, 0, "");
        let report = crate::commands::indexed_scan(&store, &library, true, &|p| {
            if throttle.ready(p.done, p.total) {
                emit("scan", p.done, p.total, "");
            }
        })?;
        let cores = info::available(&dirs.info, &dirs.cores);
        let opts = export::Options {
            playlists: true,
            bios: true,
            install_cores,
            picks: store.rules().map_err(err)?.cores,
            cache: export::default_cache().ok_or("no cache folder")?,
        };
        let mut ex = export::plan(
            &library,
            &dirs,
            &cores,
            &firmware::bundled(),
            &export::files_by_sha1(&report),
            &opts,
        );
        let mut executed = None;
        if !dry_run && !(ex.ops.is_empty() && ex.downloads.is_empty()) {
            export::download(&mut ex, &rombro_store::http_get, &|d, t, name| {
                emit("download", d, t, name)
            })
            .map_err(err)?;
            let r = plan::execute_progress(&ex.ops, &|d, t| emit("write", d, t, ""));
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
            cores: cores.iter().filter(|c| c.installed).count(),
            can_install: dirs.buildbot.is_some(),
            // asset zips are shown with the cores: both are downloads from the buildbot
            cores_install: ex
                .cores_install
                .into_iter()
                .chain(ex.assets.iter().map(|(zip, _)| {
                    let name = zip.file_name().unwrap_or_default().to_string_lossy();
                    format!("{name} (system files)")
                }))
                .collect(),
            cores_missing: ex
                .cores_missing
                .into_iter()
                .map(|(system, path)| MissingView { system, path })
                .collect(),
            playlists: ex
                .playlists
                .into_iter()
                .map(|(system, core)| PlaylistView { system, core })
                .collect(),
            playlists_unchanged: ex.playlists_unchanged,
            playlists_removed: ex.playlists_removed,
            playlists_no_core: ex.playlists_no_core,
            bios_copied: ex.bios_copied,
            bios_present: ex.bios_present,
            bios_conflicts: ex.bios_conflicts,
            bios_missing: ex
                .bios_missing
                .into_iter()
                .map(|(system, path)| MissingView { system, path })
                .collect(),
            scummvm_targets: ex.scummvm_targets,
            executed,
        })
    })
    .await
    .map_err(err)?
}
