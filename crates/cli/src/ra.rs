//! `rombro ra`: the RetroArch rombro manages itself.

use anyhow::{Context, Result};
use rombro_core::retroarch::managed::{self, Managed, Phase};
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
}

pub fn run(cmd: Cmd) -> Result<()> {
    let m = Managed::detect().context("no RetroArch stable build for this platform")?;
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
            m.install(&v, &rombro_store::http_download, &show)
                .with_context(|| format!("installing RetroArch {v}"))?;
            m.write_config(library.as_deref())?;
            println!("installed, config: {}", m.cfg().display());
        }
        Cmd::Config { library } => {
            m.write_config(library.as_deref())?;
            println!("{}", m.cfg().display());
        }
    }
    Ok(())
}

fn latest() -> Result<String> {
    let html = rombro_store::http_get(&format!("{}/", managed::STABLE_URL))
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
