//! `rombro play`: start a game in the managed RetroArch.

use crate::db::open_store;
use anyhow::{Context, Result};
use rombro_core::retroarch::managed::{
    Managed, Phase,
    launch::{Game, Step},
};
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct Args {
    /// Game file (a disc game's `.m3u`/`.cue` or the ROM)
    file: PathBuf,
    /// Library the file belongs to (default: the stored one)
    #[arg(long)]
    library: Option<PathBuf>,
    /// Use this core (id, e.g. `nestopia`) instead of the system's core
    #[arg(long)]
    core: Option<String>,
    /// Remember `--core` for this game (without `--core`: forget the override)
    #[arg(long)]
    save: bool,
    /// Only prepare and print the command
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    db: Option<PathBuf>,
}

pub fn run(a: Args) -> Result<()> {
    let store = open_store(a.db)?;
    let m = Managed::detect().context("no RetroArch stable build for this platform")?;
    let rom = std::path::absolute(&a.file)?;
    anyhow::ensure!(rom.is_file(), "{}: not a file", rom.display());
    let library = match a.library {
        Some(l) => Some(std::path::absolute(l)?),
        None => store.library()?,
    }
    .filter(|l| rom.starts_with(l));
    if a.save {
        store.set_core_override(&rom, a.core.as_deref())?;
    }
    let over = match a.core {
        Some(c) => Some(c),
        None => store.core_override(&rom)?,
    };
    let picks = store.rules()?.cores;
    let game = Game {
        rom: &rom,
        library: library.as_deref(),
        picks: &picks,
        core: over.as_deref(),
    };
    let mut l = m.prepare(&game, &rombro_store::http_download, &show)?;
    println!("{} · {}", l.system, l.core.id);
    if a.dry_run {
        println!("{:?}", l.command);
        if !l.assets.is_empty() {
            println!("{} system files to extract", l.assets.len());
        }
        return Ok(());
    }
    run_assets(&store, library.as_deref(), &l.assets)?;
    let game = store.game_key(&rom)?;
    let start = std::time::Instant::now();
    let status = l.command.status().context("starting RetroArch")?;
    let secs = start.elapsed().as_secs();
    if store.record_play(&game, secs, now())? {
        let st = store.play_stats(&game)?;
        println!(
            "played {} min · total {} min in {} runs",
            secs / 60,
            st.seconds / 60,
            st.plays
        );
    }
    anyhow::ensure!(status.success(), "RetroArch exited with {status}");
    Ok(())
}

/// Extracts the core's system files: in a library journaled (`rombro undo` reverts them).
fn run_assets(
    store: &rombro_store::Store,
    library: Option<&std::path::Path>,
    ops: &[rombro_core::plan::Op],
) -> Result<()> {
    if ops.is_empty() {
        return Ok(());
    }
    let root = library.unwrap_or(std::path::Path::new(""));
    let (id, err) = store.execute_journaled(root, ops)?;
    if let Some(id) = id {
        println!("extracted {} system files (journal #{id})", ops.len());
    }
    if let Some(e) = err {
        anyhow::bail!("system files: {e}");
    }
    Ok(())
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

fn show(s: Step) {
    match s {
        Step::RetroArch(Phase::Download { done: 0, .. }) => {
            eprintln!("installing RetroArch …")
        }
        Step::RetroArch(_) => {}
        Step::Info => eprintln!("downloading core info files …"),
        Step::Core(id) => eprintln!("installing core {id} …"),
        Step::Assets(zip) => eprintln!("downloading system files {zip} …"),
    }
}
