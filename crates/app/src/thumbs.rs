//! Thumbnails from thumbnails.libretro.com, cached on disk (misses as `.miss` markers).
use crate::commands::{CmdResult, err, open_store};
use rombro_core::thumbnail::{Kind, cache_path, url};
use std::io::Read;
use std::path::PathBuf;
use tauri::ipc::Response;

const MAX_BYTES: u64 = 8 << 20;
/// Setting: `"0"` disables downloads (cached images are still shown).
const ONLINE_KEY: &str = "thumbs_online";

fn online() -> CmdResult<bool> {
    let (store, _) = open_store()?;
    Ok(store.setting(ONLINE_KEY).map_err(err)?.as_deref() != Some("0"))
}

#[tauri::command]
pub async fn thumbs_online_get() -> CmdResult<bool> {
    online()
}

#[tauri::command]
pub async fn thumbs_online_set(on: bool) -> CmdResult<()> {
    let (store, _) = open_store()?;
    store
        .set_setting(ONLINE_KEY, if on { "1" } else { "0" })
        .map_err(err)
}

fn cache_root() -> CmdResult<PathBuf> {
    let base = match std::env::var_os("XDG_CACHE_HOME") {
        Some(d) => PathBuf::from(d),
        None => PathBuf::from(std::env::var_os("HOME").ok_or("HOME not set")?).join(".cache"),
    };
    Ok(base.join("rombro/thumbnails"))
}

fn fetch(system: &str, name: &str, kind: Kind) -> CmdResult<Option<Vec<u8>>> {
    let path = cache_path(&cache_root()?, system, name, kind);
    if let Ok(b) = std::fs::read(&path) {
        return Ok(Some(b));
    }
    let miss = path.with_extension("miss");
    if miss.exists() || !online()? {
        return Ok(None);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    let body = match ureq::get(&url(system, name, kind)).call() {
        Ok(r) => r.into_body(),
        Err(ureq::Error::StatusCode(404)) => {
            std::fs::write(&miss, b"").map_err(err)?;
            return Ok(None);
        }
        Err(e) => return Err(err(e)),
    };
    let mut bytes = Vec::new();
    body.into_reader()
        .take(MAX_BYTES)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, &bytes).map_err(err)?;
    std::fs::rename(&tmp, &path).map_err(err)?;
    Ok(Some(bytes))
}

/// PNG bytes of a thumbnail (`kind`: boxart/snap/title); empty if the server has none.
#[tauri::command]
pub async fn thumbnail(system: String, name: String, kind: String) -> CmdResult<Response> {
    let kind = Kind::parse(&kind).ok_or("unknown thumbnail kind")?;
    tauri::async_runtime::spawn_blocking(move || {
        Ok(Response::new(
            fetch(&system, &name, kind)?.unwrap_or_default(),
        ))
    })
    .await
    .map_err(err)?
}
