//! Library table: every scanned unit in the library with its identification.
use crate::commands::{CmdResult, PROGRESS_STEP, Progress, err, indexed_scan, open_store};
use rombro_core::plan::Ident;
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
}

/// The managed library path stored in the database (`None` until one is chosen).
#[tauri::command]
pub async fn library_get() -> CmdResult<Option<String>> {
    let (store, _) = open_store()?;
    Ok(store
        .library()
        .map_err(err)?
        .map(|p| p.display().to_string()))
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
                Row {
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
