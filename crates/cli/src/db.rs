use anyhow::{Context, Result};
use clap::Subcommand;
use rayon::prelude::*;
use rombro_rdb::RdbFile;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Subcommand)]
pub enum DbCmd {
    /// Show systems and entry counts of all RDB files
    Stats {
        /// RDB directory (default: ~/.config/retroarch/database/rdb)
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

pub fn run(cmd: DbCmd) -> Result<()> {
    match cmd {
        DbCmd::Stats { path } => stats(path.map_or_else(default_dir, Ok)?),
    }
}

fn default_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".config/retroarch/database/rdb"))
}

struct Row {
    system: String,
    games: usize,
    meta: usize,
    hashed: usize,
}

fn stats(dir: PathBuf) -> Result<()> {
    let start = Instant::now();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "rdb"))
        .collect();
    files.sort();

    let rows = files
        .par_iter()
        .map(|p| -> Result<Row> {
            let rdb = RdbFile::open(p).with_context(|| p.display().to_string())?;
            let mut row = Row {
                system: p
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                games: 0,
                meta: 0,
                hashed: 0,
            };
            for e in rdb.entries() {
                let e = e.with_context(|| p.display().to_string())?;
                if e.is_metadata_only() {
                    row.meta += 1;
                } else {
                    row.games += 1;
                    if e.crc.is_some() || e.md5.is_some() || e.sha1.is_some() {
                        row.hashed += 1;
                    }
                }
            }
            Ok(row)
        })
        .collect::<Result<Vec<_>>>()?;
    let elapsed = start.elapsed();

    println!(
        "{:<60} {:>8} {:>8} {:>8}",
        "System", "Games", "Hashed", "Meta"
    );
    for r in &rows {
        println!(
            "{:<60} {:>8} {:>8} {:>8}",
            r.system, r.games, r.hashed, r.meta
        );
    }
    let total: usize = rows.iter().map(|r| r.games).sum();
    println!(
        "\n{} files, {} games, loaded in {:.0?}",
        rows.len(),
        total,
        elapsed
    );
    Ok(())
}
