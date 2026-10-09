use anyhow::{Context, Result};
use clap::Subcommand;
use rayon::prelude::*;
use romburak_rdb::RdbFile;
use romburak_store::Store;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Subcommand)]
pub enum DbCmd {
    /// Show systems and entry counts of all RDB files
    Stats {
        /// RDB directory (default: auto-detected RetroArch folder)
        #[arg(long)]
        path: Option<PathBuf>,
        /// Database file for the arcade DAT versions (default: $XDG_DATA_HOME/romburak/romburak.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Download the RetroArch databases (shared with the managed RetroArch) and import
    /// new/changed RDB files into the romburak database
    Sync {
        /// RDB directory to import instead of downloading
        #[arg(long)]
        path: Option<PathBuf>,
        /// Database file (default: $XDG_DATA_HOME/romburak/romburak.db)
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
        DbCmd::Stats { path, db } => stats(path.map_or_else(default_dir, Ok)?, db),
        DbCmd::Sync { path, db } => sync(path.map_or_else(fetched_dir, Ok)?, db),
        DbCmd::Lookup { key, db } => lookup(&key, db),
    }
}

fn default_dir() -> Result<PathBuf> {
    romburak_core::paths::rdb_dir()
        .context("RetroArch RDB folder not found (pass a path or set ROMBURAK_RDB_DIR)")
}

/// Fresh databases from the buildbot; offline the auto-detected folder.
fn fetched_dir() -> Result<PathBuf> {
    let m = romburak_core::retroarch::managed::Managed::detect();
    let fetched = m.as_ref().map(|m| {
        romburak_store::update_rdbs(m, &|done, total| {
            eprint!(
                "\rdownloading databases {} / {} MB",
                done >> 20,
                total.unwrap_or(0) >> 20
            )
        })
    });
    match fetched {
        Some(Ok((dir, updated))) => {
            println!(
                "{}databases: {}",
                if updated { "\n" } else { "" },
                dir.display()
            );
            Ok(dir)
        }
        Some(Err(e)) => {
            eprintln!("database download failed ({e}), using a local RetroArch folder");
            default_dir()
        }
        None => default_dir(),
    }
}

pub(crate) fn open_store(db: Option<PathBuf>) -> Result<Store> {
    let path = match db {
        Some(p) => p,
        None => romburak_store::default_path().context("no data directory")?,
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
    let start = Instant::now();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let d = store.sync_dats(now)?;
    for (system, version) in &d.updated {
        println!("DAT {system}: {version}");
    }
    for w in &d.warnings {
        eprintln!("warning: DAT {w}");
    }
    println!(
        "arcade DATs: {} updated, {} unchanged in {:.0?}",
        d.updated.len(),
        d.unchanged,
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

fn stats(dir: PathBuf, db: Option<PathBuf>) -> Result<()> {
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
    let dats = open_store(db)?.dats()?;
    if !dats.is_empty() {
        println!("\n{:<60} {:>17} {:>10}", "Arcade DAT", "Version", "Fetched");
        for d in dats {
            let day = d.fetched.div_euclid(86_400);
            println!("{:<60} {:>17} {:>10}", d.system, d.version, civil(day));
        }
    }
    Ok(())
}

/// `YYYY-MM-DD` of a day count since the Unix epoch (Howard Hinnant's algorithm).
fn civil(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn civil_dates() {
        assert_eq!(super::civil(0), "1970-01-01");
        assert_eq!(super::civil(20_732), "2026-10-06");
        assert_eq!(super::civil(11_016), "2000-02-29");
    }
}
