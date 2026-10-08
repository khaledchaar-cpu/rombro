//! The RetroArch rombro manages itself: status, install and update (Settings → RetroArch).

use crate::commands::{CmdResult, err, open_store};
use rombro_core::retroarch::managed::{self, Managed, Phase};
use serde::Serialize;

#[derive(Serialize)]
pub struct ManagedView {
    /// A stable build exists for this platform.
    supported: bool,
    folder: Option<String>,
    installed: Option<String>,
    pinned: &'static str,
    /// Newest stable on the buildbot (only when asked for; `None` offline).
    latest: Option<String>,
}

fn managed() -> CmdResult<Managed> {
    Managed::detect().ok_or_else(|| "no RetroArch stable build for this platform".into())
}

#[tauri::command]
pub async fn ra_status(check_latest: bool) -> CmdResult<ManagedView> {
    tauri::async_runtime::spawn_blocking(move || {
        let m = Managed::detect();
        let latest = check_latest
            .then(|| rombro_store::http_get(&format!("{}/", managed::STABLE_URL)).ok())
            .flatten()
            .and_then(|html| managed::latest_stable(&String::from_utf8_lossy(&html)));
        Ok(ManagedView {
            supported: m.is_some(),
            folder: m.as_ref().map(|m| m.root.display().to_string()),
            installed: m.as_ref().and_then(Managed::current),
            pinned: managed::PINNED,
            latest,
        })
    })
    .await
    .map_err(err)?
}

/// Installs `version` (default: pinned) and writes the config. Emits `ra://progress`
/// (phase download/verify/unpack, bytes in MB, "").
#[tauri::command]
pub async fn ra_install(app: tauri::AppHandle, version: Option<String>) -> CmdResult<String> {
    use tauri::Emitter;
    tauri::async_runtime::spawn_blocking(move || {
        let m = managed()?;
        let version = version.unwrap_or_else(|| managed::PINNED.to_owned());
        let emit = |phase: &str, done: u64, total: u64| {
            let mb = |b: u64| (b >> 20) as usize;
            let _ = app.emit(
                "ra://progress",
                (
                    phase,
                    crate::commands::Progress {
                        done: mb(done),
                        total: mb(total),
                    },
                    "",
                ),
            );
        };
        m.install(&version, &rombro_store::http_download, &|p| match p {
            Phase::Download { done, total } => emit("download", done, total.unwrap_or(0)),
            Phase::Verify => emit("verify", 0, 0),
            Phase::Unpack => emit("unpack", 0, 0),
            Phase::Done => {}
        })
        .map_err(|e| format!("RetroArch {version}: {e}"))?;
        let (store, _) = open_store()?;
        let library = store.library().map_err(err)?;
        m.write_config(library.as_deref()).map_err(err)?;
        Ok(version)
    })
    .await
    .map_err(err)?
}
