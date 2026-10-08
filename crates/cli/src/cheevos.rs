//! `rombro cheevos`: which library games have RetroAchievements; login and hardcore for the
//! managed RetroArch.

use anyhow::{Context, Result};
use rombro_core::cheevos;
use std::path::PathBuf;

/// Setting holding the user's RetroAchievements Web API key.
const KEY: &str = "ra.api_key";

#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Store your Web API key (retroachievements.org → Settings → Keys)
    Key { key: String },
    /// Log in (asks for the password; only the token is stored, also in the RetroArch config)
    Login { user: String },
    /// Forget the login and turn achievements off in RetroArch
    Logout,
    /// Show or set hardcore mode (no savestates, rewind or cheats)
    Hardcore {
        #[arg(value_parser = ["on", "off"])]
        mode: Option<String>,
    },
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
        Cmd::Login { user } => {
            use std::io::IsTerminal;
            let password = if std::io::stdin().is_terminal() {
                rpassword::prompt_password("RetroAchievements password: ")?
            } else {
                // piped: first line of stdin
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                line.trim_end_matches(['\r', '\n']).to_owned()
            };
            let (user, token) = rombro_store::ra_login(&user, &password)?;
            store.set_ra_account(Some((&user, &token)))?;
            println!("logged in as {user}");
            write_config(&store)?;
        }
        Cmd::Logout => {
            store.set_ra_account(None)?;
            println!("logged out");
            write_config(&store)?;
        }
        Cmd::Hardcore { mode } => {
            if let Some(m) = mode {
                store.set_ra_hardcore(m == "on")?;
                write_config(&store)?;
            }
            let on = store.ra_hardcore()?;
            println!("hardcore: {}", if on { "on" } else { "off" });
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

/// Rewrites the managed RetroArch config (if installed) so it carries the achievement settings.
fn write_config(store: &rombro_store::Store) -> Result<()> {
    let Some(m) = rombro_core::retroarch::managed::Managed::detect() else {
        return Ok(());
    };
    let m = store.ra_prefs(m)?;
    if m.current().is_some() {
        m.write_config(store.library()?.as_deref())?;
        println!("config: {}", m.cfg().display());
    }
    Ok(())
}
