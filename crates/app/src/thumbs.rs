//! Thumbnails from thumbnails.libretro.com, cached on disk (misses as `.miss` markers).
use crate::commands::{CmdResult, err, open_store};
use romburak_core::thumbnail::{BASE_URL, Kind, best_match, cache_path, parse_listing, url};
use std::io::Read;
use std::path::Path;
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
    let base = romburak_core::paths::cache().ok_or("no cache directory")?;
    Ok(base.join("thumbnails"))
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
    let bytes = match download(&url(system, name, kind))? {
        Some(b) => b,
        None => match fuzzy_name(system, name, kind, &path)? {
            Some(other) => download(&url(system, &other, kind))?.unwrap_or_default(),
            None => Vec::new(),
        },
    };
    if bytes.is_empty() {
        std::fs::write(&miss, b"").map_err(err)?;
        return Ok(None);
    }
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, &bytes).map_err(err)?;
    std::fs::rename(&tmp, &path).map_err(err)?;
    Ok(Some(bytes))
}

/// Body of `url`, or `None` on 404.
fn download(url: &str) -> CmdResult<Option<Vec<u8>>> {
    let body = match ureq::get(url).call() {
        Ok(r) => r.into_body(),
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(e) => return Err(err(e)),
    };
    let mut bytes = Vec::new();
    body.into_reader()
        .take(MAX_BYTES)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    Ok(Some(bytes))
}

/// Server name with the same title (region/tags ignored); directory listing cached 30 days.
fn fuzzy_name(system: &str, name: &str, kind: Kind, path: &Path) -> CmdResult<Option<String>> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    // One listing download per directory, even with many tiles loading at once.
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let index = path.with_file_name(".index");
    let fresh = std::fs::metadata(&index)
        .and_then(|m| m.modified())
        .is_ok_and(|t| t.elapsed().is_ok_and(|d| d.as_secs() < 30 * 86400));
    let names: Vec<String> = if fresh {
        std::fs::read_to_string(&index)
            .map_err(err)?
            .lines()
            .map(str::to_owned)
            .collect()
    } else {
        let dir_url = url(system, "x", kind);
        let dir_url = &dir_url[..dir_url.rfind('/').unwrap_or(BASE_URL.len()) + 1];
        let html = download(dir_url)?.unwrap_or_default();
        let names = parse_listing(&String::from_utf8_lossy(&html));
        std::fs::write(&index, names.join("\n")).map_err(err)?;
        names
    };
    Ok(best_match(name, &names).map(str::to_owned))
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
