//! `romburak ra`: the RetroArch romburak manages itself.

use anyhow::{Context, Result};
use romburak_core::retroarch::managed::{self, Managed, Phase};
use romburak_core::retroarch::pick;
use std::io::Write;
use std::path::PathBuf;

#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Show installed and pinned version
    Status {
        /// Also ask the buildbot for the newest stable version
        #[arg(long)]
        check: bool,
    },
    /// Download, verify and install RetroArch (default: the pinned version)
    Install {
        /// Install this version instead
        #[arg(long, conflicts_with = "latest")]
        version: Option<String>,
        /// Install the newest stable version from the buildbot
        #[arg(long)]
        latest: bool,
        /// Library whose `_bios` folder RetroArch uses as system directory
        #[arg(long)]
        library: Option<PathBuf>,
    },
    /// Rewrite the folder settings of the managed `retroarch.cfg`
    Config {
        #[arg(long)]
        library: Option<PathBuf>,
    },
    /// Show or set how games are shown: `fullscreen`, `window`, `window:<1-6>` (scale) or
    /// `auto` (leave it to RetroArch's menu)
    Display {
        mode: Option<String>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Show or set the global shader preset: a path from `--list` (e.g. `crt/crt-royale.slangp`),
    /// `off` or `auto` (leave it to RetroArch's menu). Downloads the shader package if needed.
    Shader {
        preset: Option<String>,
        /// List presets containing this text (`""` = all)
        #[arg(long)]
        list: Option<String>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Show or set the aspect ratio: core, 4:3, 16:9, square, full or auto
    Aspect {
        ratio: Option<String>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// List the cores for each system of the library (`*` = used, recommended marked)
    Cores {
        /// Library (default: the stored one)
        #[arg(long)]
        library: Option<PathBuf>,
        /// Choose the core of a system for good, e.g. `--set "Sony - PlayStation=swanstation"`
        /// (`SYSTEM=` alone returns to the recommendation)
        #[arg(long, value_name = "SYSTEM=CORE")]
        set: Vec<String>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
}

pub fn run(cmd: Cmd) -> Result<()> {
    let mut m = Managed::detect().context("no RetroArch stable build for this platform")?;
    // the stored display mode and achievement login belong into every config romburak writes
    if let Ok(s) = crate::db::open_store(None) {
        m = s.ra_prefs(m)?;
    }
    match cmd {
        Cmd::Status { check } => {
            println!("folder:    {}", m.root.display());
            println!("installed: {}", m.current().as_deref().unwrap_or("-"));
            println!("pinned:    {}", managed::PINNED);
            if check {
                println!("latest:    {}", latest()?);
            }
            if let Some(exe) = m.executable() {
                println!("binary:    {}", exe.display());
            }
        }
        Cmd::Install {
            version,
            latest: newest,
            library,
        } => {
            let v = match (version, newest) {
                (Some(v), _) => v,
                (None, true) => latest()?,
                (None, false) => managed::PINNED.to_owned(),
            };
            println!("RetroArch {v} → {}", m.root.display());
            m.install(&v, &romburak_store::http_download, &show)
                .with_context(|| format!("installing RetroArch {v}"))?;
            m.write_config(library.as_deref())?;
            println!("installed, config: {}", m.cfg().display());
        }
        Cmd::Config { library } => {
            m.write_config(library.as_deref())?;
            println!("{}", m.cfg().display());
        }
        Cmd::Cores { library, set, db } => cores(&m, library, &set, db)?,
        Cmd::Shader { preset, list, db } => {
            let store = crate::db::open_store(db)?;
            if list.is_some() || preset.as_deref().is_some_and(|p| p != "off" && p != "auto") {
                m.ensure_package(
                    "shaders_slang",
                    &romburak_store::http_download,
                    &|done, total| show(Phase::Download { done, total }),
                )?;
            }
            let presets = m.shader_presets();
            if let Some(q) = list {
                let q = q.to_lowercase();
                presets
                    .iter()
                    .filter(|p| p.to_lowercase().contains(&q))
                    .for_each(|p| println!("{p}"));
                return Ok(());
            }
            if let Some(p) = preset {
                match p.as_str() {
                    "auto" => store.delete_setting("ra_shader")?,
                    "off" => store.set_setting("ra_shader", "off")?,
                    p if presets.iter().any(|x| x == p) => store.set_setting("ra_shader", p)?,
                    p => anyhow::bail!("{p}: unknown preset (see --list)"),
                }
                m = store.ra_prefs(m)?;
                if m.current().is_some() {
                    m.write_config(store.library()?.as_deref())?;
                }
            }
            println!(
                "{}",
                store
                    .setting("ra_shader")?
                    .as_deref()
                    .unwrap_or("auto (RetroArch's own setting)")
            );
        }
        Cmd::Aspect { ratio, db } => {
            let store = crate::db::open_store(db)?;
            if let Some(r) = ratio {
                match r.as_str() {
                    "auto" => store.delete_setting("ra_aspect")?,
                    r if managed::video::aspect_index(r).is_some() => {
                        store.set_setting("ra_aspect", r)?
                    }
                    r => anyhow::bail!("{r}: expected core, 4:3, 16:9, square, full or auto"),
                }
                m = store.ra_prefs(m)?;
                if m.current().is_some() {
                    m.write_config(store.library()?.as_deref())?;
                }
            }
            println!(
                "{}",
                store
                    .setting("ra_aspect")?
                    .as_deref()
                    .unwrap_or("auto (RetroArch's own setting)")
            );
        }
        Cmd::Display { mode, db } => {
            let store = crate::db::open_store(db)?;
            if let Some(mode) = mode {
                let d = match mode.as_str() {
                    "auto" => None,
                    s => Some(s.parse().map_err(anyhow::Error::msg)?),
                };
                store.set_ra_display(d)?;
                m.display = d;
                if m.current().is_some() {
                    m.write_config(store.library()?.as_deref())?;
                }
            }
            match store.ra_display()? {
                Some(d) => println!("{d}"),
                None => println!("auto (RetroArch's own setting)"),
            }
        }
    }
    Ok(())
}

fn cores(m: &Managed, library: Option<PathBuf>, set: &[String], db: Option<PathBuf>) -> Result<()> {
    let store = crate::db::open_store(db)?;
    let library = match library {
        Some(l) => std::path::absolute(l)?,
        None => store
            .library()?
            .context("no library stored (use --library)")?,
    };
    if !set.is_empty() {
        let mut rules = store.rules()?;
        for s in set {
            let (system, id) = s
                .split_once('=')
                .with_context(|| format!("--set {s}: expected SYSTEM=CORE"))?;
            match id.trim() {
                "" => rules.cores.remove(system.trim()),
                id => rules.cores.insert(system.trim().to_owned(), id.to_owned()),
            };
        }
        store.set_rules(&rules)?;
    }
    m.ensure_info(&romburak_store::http_download, false)
        .context("loading the core list")?;
    let cores = m.cores();
    let picks = store.rules()?.cores;
    for system in pick::library_systems(&library) {
        let chosen = pick::resolve(&cores, &system, &picks, true).map(|c| c.id.as_str());
        let rec = pick::recommended(&system);
        let list: Vec<String> = pick::options(&cores, &system)
            .iter()
            .map(|c| {
                let mut s = c.id.clone();
                if Some(c.id.as_str()) == chosen {
                    s.insert(0, '*');
                }
                if Some(c.id.as_str()) == rec {
                    s.push_str(" (recommended)");
                }
                if c.installed {
                    s.push_str(" [installed]");
                }
                s
            })
            .collect();
        if !list.is_empty() {
            println!("{system}: {}", list.join(", "));
        }
    }
    Ok(())
}

fn latest() -> Result<String> {
    let html = romburak_store::http_get(&format!("{}/", managed::STABLE_URL))
        .context("buildbot not reachable")?;
    managed::latest_stable(&String::from_utf8_lossy(&html)).context("no stable version listed")
}

fn show(p: Phase) {
    let mut err = std::io::stderr();
    let _ = match p {
        Phase::Download { done, total } => {
            let mb = |b: u64| b as f64 / 1e6;
            match total {
                Some(t) => write!(err, "\rdownload {:.0}/{:.0} MB", mb(done), mb(t)),
                None => write!(err, "\rdownload {:.0} MB", mb(done)),
            }
        }
        Phase::Verify => writeln!(err, "\nverify"),
        Phase::Unpack { done, total } => write!(err, "\runpack {}/{} MB", done >> 20, total >> 20),
        Phase::Done => writeln!(err),
    };
}
