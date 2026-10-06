//! `rombro retroarch`: playlists and BIOS files into RetroArch's folders.

use crate::db::open_store;
use anyhow::{Context, Result, bail};
use rombro_core::plan;
use rombro_core::retroarch::{Dirs, export, firmware, info};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(clap::Args)]
pub struct Args {
    /// Library folder (playlists from its `_playlists/`, BIOS files from the whole library)
    pub library: PathBuf,
    /// retroarch.cfg to read the folders from (default: auto-detected)
    #[arg(long)]
    pub cfg: Option<PathBuf>,
    /// Only show what would be done
    #[arg(long)]
    pub dry_run: bool,
    /// Skip playlists
    #[arg(long)]
    pub no_playlists: bool,
    /// Skip BIOS files
    #[arg(long)]
    pub no_bios: bool,
    #[arg(long)]
    pub db: Option<PathBuf>,
}

pub fn run(a: Args) -> Result<()> {
    let store = open_store(a.db)?;
    let library = std::path::absolute(&a.library)?;
    let dirs = match &a.cfg {
        Some(cfg) => Dirs::from_cfg(cfg).with_context(|| cfg.display().to_string())?,
        None => Dirs::detect().context("no retroarch.cfg found (use --cfg)")?,
    };
    println!(
        "RetroArch: playlists {}, system {}",
        dirs.playlists.display(),
        dirs.system.display()
    );
    let cache = store.hash_cache(&library)?;
    let report = rombro_core::scan_cached(&library, &cache, &|_, _| {});
    store.save_scan(&library, &report)?;
    let cores = info::installed(&dirs.info, &dirs.cores);
    let ex = export::plan(
        &library,
        &dirs,
        &cores,
        &firmware::bundled(),
        &export::files_by_sha1(&report),
        !a.no_playlists,
        !a.no_bios,
    );
    for (system, core) in &ex.playlists {
        let core = core
            .as_deref()
            .unwrap_or("no installed core – RetroArch asks");
        println!("PLAYLIST {system}  [{core}]");
    }
    for p in &ex.bios_copied {
        println!("BIOS     {p}");
    }
    for p in &ex.bios_conflicts {
        println!("KEEP     {p}  (exists with other content)");
    }
    for (system, p) in &ex.bios_missing {
        println!("MISSING  {p}  [{system}]");
    }
    println!(
        "\n{} playlists ({} unchanged), {} BIOS to copy ({} present, {} conflicts, {} missing); {} cores installed",
        ex.playlists.len(),
        ex.playlists_unchanged,
        ex.bios_copied.len(),
        ex.bios_present,
        ex.bios_conflicts.len(),
        ex.bios_missing.len(),
        cores.len()
    );
    if a.dry_run || ex.ops.is_empty() {
        return Ok(());
    }
    let r = plan::execute(&ex.ops);
    if !r.done.is_empty() {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let id = store.add_journal(
            ts,
            &library.to_string_lossy(),
            &plan::journal_to_json(&r.done),
        )?;
        println!(
            "executed {} operations (journal #{id}; `rombro undo` reverts)",
            r.done.len()
        );
    }
    if let Some((op, e)) = r.error {
        bail!("stopped at {}: {e}", op.target().display());
    }
    Ok(())
}
