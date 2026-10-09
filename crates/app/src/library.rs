//! Library table: every scanned unit in the library with its identification.
use crate::commands::{CmdResult, Throttle, err, library_snapshot, open_store};
use romburak_core::plan::{BIOS_DIR, Ident, PLAYLIST_DIR, TRASH_DIR};
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
    /// When the file entered the library (unix seconds, from the index; file time as fallback).
    added: i64,
    favorite: bool,
    /// Play statistics (zero when never played).
    plays: u64,
    seconds: u64,
    last_played: i64,
    /// RetroAchievements of this exact file (0 = none or not synced).
    cheevos: u32,
    /// RA title of another version that has achievements, when this one has none.
    cheevos_other: Option<String>,
    /// RA game id (this file's game, else the other version's) for the achievement list.
    cheevos_game: Option<u64>,
    /// The user's unlocks in this file's RA game (when logged in).
    cheevos_progress: Option<romburak_store::RaProgress>,
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

/// Lists the library from its snapshot (see [`library_snapshot`]); `rescan` checks every
/// file on disk instead (maintenance, after changes made outside Romburak).
/// `library` replaces the stored library path; without it the stored one is used.
#[tauri::command]
pub async fn library_list(
    app: AppHandle,
    library: Option<PathBuf>,
    rescan: Option<bool>,
) -> CmdResult<Vec<Row>> {
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
        let throttle = Throttle::new();
        let snap = library_snapshot(&store, &library, rescan.unwrap_or(false), &|p| {
            if throttle.ready(p.done, p.total) {
                let _ = app.emit("scan://progress", p);
            }
        })?;
        let rules = crate::settings::load_rules(&store)?;
        let items = romburak_core::plan::name_only(&snap.items, &rules);
        let added_db = store.added_times(&library).map_err(err)?;
        let keys = store.game_keys(&library).map_err(err)?;
        let stats = store.all_play_stats().map_err(err)?;
        let favorites = store.favorites().map_err(err)?;
        let ra = crate::cheevos::Index::load(&store, &library)?;
        let mut folder_systems = rules.folder_systems;
        // MSU-1 games: no database knows them; the ROM in `<MSU-1>/<Game>/` is the game,
        // named after its folder
        let msu = library.join(romburak_core::plan::MSU1_SYSTEM);
        // Daphne: a collection, one game per `<game>.daphne/` folder (`roms/<game>.zip`
        // is what the core loads); every other file in it is data of the collection
        let daphne = library.join(romburak_core::plan::DAPHNE_SYSTEM);
        let daphne_game = |p: &std::path::Path| {
            let stem = p.file_stem()?.to_string_lossy().into_owned();
            let in_roms = p.parent()? == daphne.join("roms");
            (in_roms && daphne.join(format!("{stem}.daphne")).is_dir()).then_some(stem)
        };
        let mut daphne_seen = std::collections::HashSet::new();
        let items: Vec<_> = items
            .into_iter()
            .filter_map(|mut it| {
                let p = it.files.primary();
                if !p.starts_with(&daphne) {
                    return Some(it);
                }
                match daphne_game(p) {
                    // a multi-ROM zip comes as one item per member: list the game once
                    Some(name) if daphne_seen.insert(p.clone()) => {
                        it.files = romburak_core::plan::Files::Single(p.clone());
                        it.ident = Ident::Named(romburak_core::plan::Game {
                            system: romburak_core::plan::DAPHNE_SYSTEM.to_owned(),
                            name,
                            crc: None,
                        });
                        Some(it)
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        let items: Vec<_> = items
            .into_iter()
            .map(|mut it| {
                let p = it.files.primary();
                let rom = p.extension().is_some_and(|e| {
                    e.eq_ignore_ascii_case("sfc") || e.eq_ignore_ascii_case("smc")
                });
                if matches!(it.ident, Ident::Unknown)
                    && rom
                    && let Some(dir) = p.parent().filter(|d| d.parent() == Some(msu.as_path()))
                    && let Some(name) = dir.file_name()
                {
                    it.ident = Ident::Named(romburak_core::plan::Game {
                        system: romburak_core::plan::MSU1_SYSTEM.to_owned(),
                        name: name.to_string_lossy().into_owned(),
                        crc: None,
                    });
                }
                it
            })
            .collect();
        folder_systems.push(romburak_core::plan::MSU1_SYSTEM.to_owned());
        folder_systems.extend(
            romburak_core::plan::PORTS
                .iter()
                .map(|(s, _)| (*s).to_owned()),
        );
        // Game folders (DOS, ScummVM, ports): the folder of a known key file is the game;
        // its other files are game data, not unknown items.
        let game_dirs: std::collections::HashSet<PathBuf> = items
            .iter()
            .filter_map(|it| match &it.ident {
                Ident::Known(g) | Ident::Named(g) if folder_systems.contains(&g.system) => {
                    it.files.primary().parent().map(|d| d.to_path_buf())
                }
                _ => None,
            })
            .filter(|d| d != &library)
            .collect();
        let mut extra: std::collections::HashMap<PathBuf, usize> = Default::default();
        let in_game = |p: &std::path::Path| {
            p.ancestors()
                .skip(1)
                .take_while(|a| *a != library)
                .find(|a| game_dirs.contains(*a))
                .map(|a| a.to_path_buf())
        };
        let items: Vec<_> = items
            .into_iter()
            .filter(|it| {
                if !matches!(it.ident, Ident::Unknown) {
                    return true;
                }
                match in_game(it.files.primary()) {
                    Some(dir) => {
                        *extra.entry(dir).or_default() += it.files.all().len();
                        false
                    }
                    None => true,
                }
            })
            .collect();
        Ok(items
            .into_iter()
            // trash, playlists and BIOS sets are managed by Romburak, not part of the collection
            .filter(|it| {
                let p = it.files.primary();
                !matches!(it.ident, Ident::Bios(_) | Ident::Firmware(_))
                    && ![TRASH_DIR, PLAYLIST_DIR, BIOS_DIR]
                        .iter()
                        .any(|d| p.starts_with(library.join(d)))
            })
            .map(|it| {
                let p = it.files.primary();
                let path = p.strip_prefix(&library).unwrap_or(p).display().to_string();
                let mut files = it.files.all().len();
                if matches!(it.ident, Ident::Known(_) | Ident::Named(_))
                    && let Some(n) = p.parent().and_then(|d| extra.get(d))
                {
                    files += n;
                }
                let (state, system, name) = match it.ident {
                    Ident::Known(g) => ("known", g.system, g.name),
                    Ident::Named(g) => ("named", g.system, g.name),
                    Ident::Ambiguous(c) => (
                        "ambiguous",
                        String::new(),
                        format!("{} candidates", c.len()),
                    ),
                    Ident::Unknown => ("unknown", String::new(), String::new()),
                    Ident::Skip(r) | Ident::Incomplete(r) => ("skip", String::new(), r),
                    Ident::Bios(g) => ("skip", g.system, g.name), // filtered above
                    Ident::Firmware(_) => ("skip", String::new(), String::new()), // filtered above
                };
                let regions = romburak_core::naming::parse(&name)
                    .regions
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                let key = keys
                    .get(p)
                    .cloned()
                    .unwrap_or_else(|| format!("path:{}", p.display()));
                let st = stats.get(&key).copied().unwrap_or_default();
                let (cheevos, cheevos_other, cheevos_game) = ra.lookup(p, &system, &name);
                Row {
                    cheevos_progress: cheevos_game
                        .filter(|_| cheevos > 0)
                        .and_then(|g| ra.progress(g)),
                    cheevos_game,
                    cheevos,
                    cheevos_other,
                    favorite: favorites.contains(&key),
                    plays: st.plays,
                    seconds: st.seconds,
                    last_played: st.last,
                    added: added_db.get(p).copied().unwrap_or_else(|| added(p)),
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
