//! `rombro cheevos`: which library games have RetroAchievements.

use anyhow::{Context, Result};
use rombro_core::cheevos;
use std::path::PathBuf;

/// Setting holding the user's RetroAchievements Web API key.
const KEY: &str = "ra.api_key";

#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Store your Web API key (retroachievements.org → Settings → Keys)
    Key { key: String },
    /// Download the game lists (with hashes) of all supported consoles
    Sync,
    /// Count the library games with achievements per system
    Scan {
        /// Library (default: the stored one)
        #[arg(long)]
        library: Option<PathBuf>,
        /// Also list each matched game
        #[arg(long)]
        list: bool,
    },
}

pub fn run(cmd: Cmd, db: Option<PathBuf>) -> Result<()> {
    let mut store = crate::db::open_store(db)?;
    match cmd {
        Cmd::Key { key } => {
            store.set_setting(KEY, key.trim())?;
            println!("Web API key stored");
        }
        Cmd::Sync => {
            let key = store
                .setting(KEY)?
                .context("no Web API key stored (rombro cheevos key <key>)")?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64;
            let r = store.ra_sync(&key, now, &|done, total| {
                eprint!("\rconsole {done}/{total}");
            })?;
            eprintln!();
            println!(
                "{} consoles, {} games with achievements, {} hashes",
                r.consoles, r.games, r.hashes
            );
            for (id, e) in &r.failed {
                println!("failed: console {id}: {e}");
            }
        }
        Cmd::Scan { library, list } => {
            let library = match library {
                Some(l) => l,
                None => store
                    .library()?
                    .context("no library stored (use --library)")?,
            };
            let mut systems: Vec<_> = std::fs::read_dir(&library)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    cheevos::console(&name).map(|c| (name, e.path(), c))
                })
                .collect();
            systems.sort_by(|a, b| a.0.cmp(&b.0));
            let (mut all, mut hit) = (0, 0);
            for (name, dir, console) in systems {
                let (mut n, mut found) = (0, 0);
                for f in std::fs::read_dir(&dir)?.filter_map(|e| e.ok()) {
                    let p = f.path();
                    let fname = f.file_name().to_string_lossy().into_owned();
                    if !p.is_file() || fname.starts_with('.') {
                        continue;
                    }
                    n += 1;
                    let Some(h) = store.ra_hash(&p, console.method)? else {
                        continue;
                    };
                    if let Some(g) = store.ra_game(&h)? {
                        found += 1;
                        if list {
                            println!("  {fname} → {} ({} achievements)", g.title, g.achievements);
                        }
                    }
                }
                println!("{name}: {found}/{n}");
                all += n;
                hit += found;
            }
            println!("total: {hit}/{all} games with achievements");
        }
    }
    Ok(())
}
