//! `rombro retroarch`: playlists, BIOS files and missing cores into RetroArch's folders.

use crate::db::open_store;
use anyhow::{Context, Result, bail};
use rombro_core::plan;
use rombro_core::retroarch::{Dirs, export, firmware, info, pick};
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
    /// Download missing cores from RetroArch's buildbot (`core_updater_buildbot_cores_url`)
    #[arg(long)]
    pub install_cores: bool,
    /// Core for a system for this run, e.g. `--core "Sony - PlayStation=swanstation"`
    /// (lasting choice: `cores` in `rules --set`)
    #[arg(long, value_name = "SYSTEM=CORE")]
    pub core: Vec<String>,
    /// List the cores available for each playlist (recommended first) and exit
    #[arg(long)]
    pub list_cores: bool,
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
    let cores = info::available(&dirs.info, &dirs.cores);
    let mut picks = store.rules()?.cores;
    for c in &a.core {
        let (system, id) = c
            .split_once('=')
            .with_context(|| format!("--core {c}: expected SYSTEM=CORE"))?;
        picks.insert(system.trim().to_owned(), id.trim().to_owned());
    }
    if a.list_cores {
        return list_cores(&library, &cores, &picks);
    }
    let mut cache = store.hash_cache(&library)?;
    cache.trusted = true;
    let report = rombro_core::scan_cached(&library, &cache, &|_| {});
    store.save_scan(&library, &report, true)?;
    if a.install_cores && dirs.buildbot.is_none() {
        bail!("retroarch.cfg has no core_updater_buildbot_cores_url – cannot install cores");
    }
    let opts = export::Options {
        playlists: !a.no_playlists,
        bios: !a.no_bios,
        install_cores: a.install_cores,
        picks,
        cache: export::default_cache().context("no cache folder")?,
    };
    let mut ex = export::plan(
        &library,
        &dirs,
        &cores,
        &firmware::bundled(),
        &export::files_by_sha1(&report),
        &opts,
    );
    for (system, core) in &ex.playlists {
        let core = core
            .as_deref()
            .unwrap_or("no installed core – RetroArch asks");
        println!("PLAYLIST {system}  [{core}]");
    }
    for system in &ex.playlists_no_core {
        println!("NO CORE  {system}  (no RetroArch core runs it – no playlist)");
    }
    for name in &ex.playlists_removed {
        println!("REMOVE   {name}  (no games left – to the trash)");
    }
    for id in &ex.cores_install {
        println!("CORE     {id}  (install)");
    }
    for (zip, _) in &ex.assets {
        let name = zip.file_name().unwrap_or_default().to_string_lossy();
        println!("ASSETS   {name}  (system files a core needs)");
    }
    for (system, id) in &ex.cores_missing {
        println!("NO CORE  {id}  [{system}]  (missing: --install-cores)");
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
    for id in &ex.scummvm_targets {
        println!("SCUMMVM  {id}  (target in scummvm.ini)");
    }
    println!(
        "\n{} playlists ({} unchanged), {} BIOS to copy ({} present, {} conflicts, {} missing); {} cores installed",
        ex.playlists.len(),
        ex.playlists_unchanged,
        ex.bios_copied.len(),
        ex.bios_present,
        ex.bios_conflicts.len(),
        ex.bios_missing.len(),
        cores.iter().filter(|c| c.installed).count()
    );
    if a.dry_run || ex.ops.is_empty() && ex.downloads.is_empty() {
        return Ok(());
    }
    if !ex.downloads.is_empty() {
        println!("downloading {} archives …", ex.downloads.len());
        export::download(&mut ex, &rombro_store::http_get, &|i, n, name| {
            if !name.is_empty() {
                println!("  [{}/{n}] {name}", i + 1);
            }
        })?;
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

fn list_cores(
    library: &std::path::Path,
    cores: &[info::Core],
    picks: &std::collections::BTreeMap<String, String>,
) -> Result<()> {
    for system in export::playlist_systems(library) {
        let chosen = pick::resolve(cores, &system, picks, true).map(|c| c.id.as_str());
        let rec = pick::recommended(&system);
        let list: Vec<String> = pick::options(cores, &system)
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
        println!("{system}: {}", list.join(", "));
    }
    Ok(())
}
