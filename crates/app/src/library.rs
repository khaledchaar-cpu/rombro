//! Library table: every scanned unit in the library with its identification.
use crate::commands::{CmdResult, PROGRESS_STEP, Progress, err, open_store};
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

#[tauri::command]
pub async fn library_list(app: AppHandle, library: PathBuf) -> CmdResult<Vec<Row>> {
    if !library.is_dir() {
        return Err(format!("{} is not a directory", library.display()));
    }
    tauri::async_runtime::spawn_blocking(move || {
        let (store, _) = open_store()?;
        let report = rombro_core::scan_with_progress(&library, &|done, total| {
            if done % PROGRESS_STEP == 0 || done == total {
                let _ = app.emit("scan://progress", Progress { done, total });
            }
        });
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
