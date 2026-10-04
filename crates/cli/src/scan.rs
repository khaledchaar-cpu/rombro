use crate::db::open_store;
use anyhow::Result;
use rombro_core::ScannedRom;
use rombro_store::{DiscMatch, Match, Store};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

/// Scans a directory and reports each ROM as verified / crc-only / unknown / duplicate.
pub fn run(dir: PathBuf, db: Option<PathBuf>, unknown_only: bool) -> Result<()> {
    let store = open_store(db)?;
    let t = Instant::now();
    let report = rombro_core::scan(&dir);
    let hashed = t.elapsed();
    let bytes: u64 = report
        .roms
        .iter()
        .chain(report.discs.iter().flat_map(|d| &d.tracks))
        .map(|r| r.hashes.size)
        .sum();

    let mut seen: HashMap<[u8; 20], String> = HashMap::new();
    let (mut verified, mut weak, mut unknown, mut dups) = (0, 0, 0, 0);
    for rom in &report.roms {
        let label = display(rom);
        let (m, headerless) = identify(&store, rom)?;
        let key = if headerless {
            rom.headerless.map_or(rom.hashes.sha1, |h| h.sha1)
        } else {
            rom.hashes.sha1
        };
        let status = match &m {
            Match::Unknown => {
                unknown += 1;
                "UNKNOWN ".to_owned()
            }
            _ if seen.contains_key(&key) => {
                dups += 1;
                format!("DUP     (of {})", seen[&key])
            }
            Match::Verified(r) => {
                verified += 1;
                format!("OK      [{}] {}", r[0].system, r[0].name)
            }
            Match::CrcOnly(r) => {
                weak += 1;
                format!("CRC     [{}] {}", r[0].system, r[0].name)
            }
        };
        if !matches!(m, Match::Unknown) {
            seen.entry(key).or_insert_with(|| label.clone());
        }
        if !unknown_only || matches!(m, Match::Unknown) {
            let hdr = if headerless { " (headerless)" } else { "" };
            println!("{status}{hdr}  {label}");
        }
    }
    let mut disc_unknown = 0;
    for d in &report.discs {
        let serial =
            d.id.as_ref()
                .map(|id| format!(" <{}>", id.serial))
                .unwrap_or_default();
        let status = match store.identify_disc(d)? {
            DiscMatch::Hash(Match::Verified(r)) => {
                format!("OK      [{}] {}", r[0].system, r[0].name)
            }
            DiscMatch::Hash(Match::CrcOnly(r)) => {
                format!("CRC     [{}] {}", r[0].system, r[0].name)
            }
            DiscMatch::Serial(r) => format!("SERIAL  [{}] {}", r[0].system, r[0].name),
            DiscMatch::Hash(Match::Unknown) | DiscMatch::Unknown => {
                disc_unknown += 1;
                "UNKNOWN ".to_owned()
            }
        };
        if !unknown_only || status.starts_with("UNKNOWN") {
            println!("{status}{serial}  {}", d.path.display());
            for m in &d.missing {
                println!("        missing track: {}", m.display());
            }
        }
    }
    for f in &report.failures {
        eprintln!("ERROR   {}: {}", f.path.display(), f.error);
    }
    let mb = bytes as f64 / 1e6;
    println!(
        "\n{} roms ({mb:.1} MB) in {hashed:.2?} ({:.0} MB/s): {verified} verified, {weak} crc-only, \
         {dups} duplicates, {unknown} unknown; {} discs ({disc_unknown} unknown), \
         {} playlists, {} errors",
        report.roms.len(),
        mb / hashed.as_secs_f64().max(1e-9),
        report.discs.len(),
        report.playlists.len(),
        report.failures.len()
    );
    Ok(())
}

/// Tries the headerless variant first (RetroArch hashes without headers), then the full file.
fn identify(store: &Store, rom: &ScannedRom) -> Result<(Match, bool)> {
    if let Some(h) = rom.headerless {
        let m = store.identify(&h)?;
        if !matches!(m, Match::Unknown) {
            return Ok((m, true));
        }
    }
    Ok((store.identify(&rom.hashes)?, false))
}

fn display(rom: &ScannedRom) -> String {
    match &rom.member {
        Some(m) => format!("{}#{m}", rom.path.display()),
        None => rom.path.display().to_string(),
    }
}
