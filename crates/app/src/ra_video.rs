//! Settings → RetroArch: global shader preset and aspect ratio of the managed RetroArch.

use crate::commands::{CmdResult, Progress, err, open_store};
use crate::managed_ra::rewrite_config;
use romburak_core::retroarch::managed::Managed;
use serde::Serialize;
use tauri::Emitter;

#[derive(Serialize)]
pub struct VideoView {
    /// `None` = RetroArch's own setting, `"off"` or a preset path.
    shader: Option<String>,
    aspect: Option<String>,
    installed: bool,
}

#[tauri::command]
pub fn ra_video() -> CmdResult<VideoView> {
    let (store, _) = open_store()?;
    Ok(VideoView {
        shader: store.setting("ra_shader").map_err(err)?,
        aspect: store.setting("ra_aspect").map_err(err)?,
        installed: Managed::detect().is_some_and(|m| m.current().is_some()),
    })
}

/// Shader presets of `shaders_slang`; downloads the package first if RetroArch has none
/// (progress on `ra://package` in MB).
#[tauri::command]
pub async fn ra_shaders(app: tauri::AppHandle) -> CmdResult<Vec<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let m = Managed::detect().ok_or("no RetroArch stable build for this platform")?;
        let progress = |done: u64, total: Option<u64>| {
            let p = Progress {
                done: (done >> 20) as usize,
                total: (total.unwrap_or(0) >> 20) as usize,
            };
            let _ = app.emit("ra://package", p);
        };
        m.ensure_package("shaders_slang", &romburak_store::http_download, &progress)
            .map_err(err)?;
        Ok(m.shader_presets())
    })
    .await
    .map_err(err)?
}

fn set(key: &str, value: Option<String>) -> CmdResult<()> {
    let (store, _) = open_store()?;
    match value {
        Some(v) => store.set_setting(key, &v).map_err(err)?,
        None => store.delete_setting(key).map_err(err)?,
    }
    rewrite_config(&store)
}

/// `None` = leave to RetroArch, `"off"`, or a preset path from [`ra_shaders`]. A running game
/// switches right away (not for `None`); returns whether it did.
#[tauri::command]
pub fn ra_set_shader(shader: Option<String>) -> CmdResult<bool> {
    let live = match shader.as_deref() {
        None => None,
        Some("off") => Some(String::new()),
        Some(p) => Managed::detect()
            .and_then(|m| Some(m.package_dir(&m.current()?, "shaders_slang")?.join(p)))
            .filter(|f| f.is_file())
            .map(|f| f.to_string_lossy().into_owned()),
    };
    set("ra_shader", shader)?;
    match live {
        Some(f) => crate::live::send(format!("SET_SHADER {f}").trim_end()),
        None => Ok(false),
    }
}

/// `None` = leave to RetroArch, else one of core/4:3/16:9/square/full.
#[tauri::command]
pub fn ra_set_aspect(aspect: Option<String>) -> CmdResult<()> {
    if let Some(a) = &aspect
        && romburak_core::retroarch::managed::video::aspect_index(a).is_none()
    {
        return Err(format!("unknown aspect ratio {a}"));
    }
    set("ra_aspect", aspect)
}
