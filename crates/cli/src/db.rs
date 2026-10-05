use anyhow::{Context, Result};
use clap::Subcommand;
use rayon::prelude::*;
use rombro_rdb::RdbFile;
use rombro_store::Store;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Subcommand)]
pub enum DbCmd {
    /// Show systems and entry counts of all RDB files
    Stats {
        /// RDB directory (default: auto-detected RetroArch folder)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Import new/changed RDB files into the rombro database
    Sync {
        /// RDB directory (default: auto-detected RetroArch folder)
        #[arg(long)]
        path: Option<PathBuf>,
        /// Database file (default: $XDG_DATA_HOME/rombro/rombro.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Look up entries by crc (8 hex), sha1 (40 hex), md5 (32 hex) or serial
    Lookup {
        key: String,
        #[arg(long)]
        db: Option<PathBuf>,
    },
}

pub fn run(cmd: DbCmd) -> Result<()> {
    match cmd {
        DbCmd::Stats { path } => stats(path.map_or_else(default_dir, Ok)?),
        DbCmd::Sync { path, db } => sync(path.map_or_else(default_dir, Ok)?, db),
        DbCmd::Lookup { key, db } => lookup(&key, db),
    }
}

fn default_dir() -> Result<PathBuf> {
    rombro_core::paths::rdb_dir()
        .context("RetroArch RDB folder not found (pass a path or set ROMBRO_RDB_DIR)")
}

pub(crate) fn open_store(db: Option<PathBuf>) -> Result<Store> {
    let path = match db {
        Some(p) => p,
        None => rombro_store::default_path().context("no data directory")?,
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    Store::open(&path).with_context(|| format!("opening {}", path.display()))
}

fn sync(dir: PathBuf, db: Option<PathBuf>) -> Result<()> {
    let start = Instant::now();
    let mut store = open_store(db)?;
    let r = store.sync_rdbs(&dir)?;
    println!(
        "imported {} files ({} entries, {} merged, {} orphaned), {} unchanged, {} removed in {:.0?}",
        r.imported,
        r.entries,
        r.merged,
        r.orphaned,
        r.unchanged,
        r.removed,
        start.elapsed()
    );
    Ok(())
}

fn lookup(key: &str, db: Option<PathBuf>) -> Result<()> {
    let store = open_store(db)?;
    let start = Instant::now();
    let hex = key.len() % 2 == 0 && key.bytes().all(|b| b.is_ascii_hexdigit());
    let hits = match (hex, key.len()) {
        (true, 8) => store.by_crc(u32::from_str_radix(key, 16)?, None)?,
        (true, 32) => store.by_md5(&decode_hex(key))?,
        (true, 40) => store.by_sha1(&decode_hex(key))?,
        _ => store.by_serial(key, None)?,
    };
    let elapsed = start.elapsed();
    for h in &hits {
        println!("{:<40} {}", h.system, h.name);
    }
    println!("{} hit(s) in {:.1?}", hits.len(), elapsed);
    Ok(())
}

fn decode_hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
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
