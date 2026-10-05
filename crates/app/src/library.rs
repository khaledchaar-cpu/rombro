//! Library table: every scanned unit in the library with its identification.
use crate::commands::{CmdResult, PROGRESS_STEP, Progress, err, indexed_scan, open_store};
use rombro_core::plan::{Ident, PLAYLIST_DIR, TRASH_DIR};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

#[derive(Serialize)]
pub struct Row {
    path: String,
    system: String,
    name: String,
    /// `known`, `ambiguous`, `unknown` or `skip`.
    state: &'static str,
    files: usize,
    /// Region tags parsed from the name (e.g. `Europe`, `USA`).
    regions: Vec<String>,
    /// When the file arrived at its current place (unix seconds; inode change time on Unix).
    added: i64,
}

fn added(p: &std::path::Path) -> i64 {
    let Ok(m) = std::fs::metadata(p) else {
        return 0;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        m.ctime()
    }
    #[cfg(not(unix))]
    {
        m.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs() as i64)
    }
}

/// Last used paths and import mode, restored at app start.
#[derive(Serialize)]
pub struct Session {
    library: Option<String>,
    inbox: Option<String>,
    mode: Option<String>,
}

#[tauri::command]
pub async fn session_get() -> CmdResult<Session> {
    let (store, _) = open_store()?;
    let get = |k: &str| store.setting(k).map_err(err);
    Ok(Session {
        library: get("library")?,
        inbox: get("inbox")?,
        mode: get("mode")?,
    })
}

/// Lists the library from the file index; only new or changed files are hashed.
/// `library` replaces the stored library path; without it the stored one is used.
#[tauri::command]
pub async fn library_list(app: AppHandle, library: Option<PathBuf>) -> CmdResult<Vec<Row>> {
    tauri::async_runtime::spawn_blocking(move || {
        let (store, _) = open_store()?;
        let library = match library {
            Some(l) => std::path::absolute(l).map_err(err)?,
            None => store
                .library()
                .map_err(err)?
                .ok_or("no library configured")?,
        };
        if !library.is_dir() {
            return Err(format!("{} is not a directory", library.display()));
        }
        store.set_library(&library).map_err(err)?;
        let report = indexed_scan(&store, &library, &|done, total| {
            if done % PROGRESS_STEP == 0 || done == total {
                let _ = app.emit("scan://progress", Progress { done, total });
            }
        })?;
        let items = store.items(&report, true).map_err(err)?;
        Ok(items
            .into_iter()
            // trash and playlists are managed by RomBro, not part of the collection
            .filter(|it| {
                let p = it.files.primary();
                ![TRASH_DIR, PLAYLIST_DIR]
                    .iter()
                    .any(|d| p.starts_with(library.join(d)))
            })
            .map(|it| {
                let p = it.files.primary();
                let path = p.strip_prefix(&library).unwrap_or(p).display().to_string();
                let files = it.files.all().len();
                let (state, system, name) = match it.ident {
                    Ident::Known(g) => ("known", g.system, g.name),
                    Ident::Ambiguous(c) => (
                        "ambiguous",
                        String::new(),
                        format!("{} candidates", c.len()),
                    ),
                    Ident::Unknown => ("unknown", String::new(), String::new()),
                    Ident::Skip(r) => ("skip", String::new(), r),
                };
                let regions = rombro_core::naming::parse(&name)
                    .regions
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                Row {
                    added: added(p),
                    regions,
                    path,
                    system,
                    name,
                    state,
                    files,
                }
            })
            .collect())
    })
    .await
    .map_err(err)?
}
