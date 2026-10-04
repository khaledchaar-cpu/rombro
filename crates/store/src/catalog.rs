//! Turning a scan report into planner items (identification + stored resolutions).

use crate::{DiscMatch, Match, Record, Result, Store, candidates};
use rombro_core::plan::{Files, Game, Ident, Item};
use rombro_core::{ScanReport, ScannedRom};

impl Store {
    /// Identifies every scanned ROM and disc. Archives holding several ROMs and discs with
    /// missing tracks are skipped; ambiguous matches use a stored resolution if there is one.
    pub fn items(&self, report: &ScanReport, in_library: bool) -> Result<Vec<Item>> {
        let mut out = Vec::new();
        for group in report.roms.chunk_by(|a, b| a.path == b.path) {
            let rom = &group[0];
            let ident = if group.len() > 1 {
                Ident::Skip(format!("archive with {} ROMs", group.len()))
            } else {
                self.ident(&self.identify_rom(rom)?, &rom.hashes.sha1)?
            };
            out.push(Item {
                files: Files::Single(rom.path.clone()),
                ident,
                in_library,
            });
        }
        for d in &report.discs {
            let ident = if !d.missing.is_empty() {
                Ident::Skip(format!("{} missing track(s)", d.missing.len()))
            } else {
                let records = match self.identify_disc(d)? {
                    DiscMatch::Hash(Match::Verified(r) | Match::CrcOnly(r))
                    | DiscMatch::Serial(r) => r,
                    DiscMatch::Hash(Match::Unknown) | DiscMatch::Unknown => Vec::new(),
                };
                let sha1 = d.tracks.first().map(|t| t.hashes.sha1).unwrap_or_default();
                self.ident(&records, &sha1)?
            };
            let files = if d.tracks.len() == 1 && d.tracks[0].path == d.path {
                Files::Single(d.path.clone())
            } else {
                Files::Sheet {
                    sheet: d.path.clone(),
                    tracks: d.tracks.iter().map(|t| t.path.clone()).collect(),
                }
            };
            out.push(Item {
                files,
                ident,
                in_library,
            });
        }
        Ok(out)
    }

    /// Headerless hashes first (RetroArch hashes without headers), then the full file.
    pub fn identify_rom(&self, rom: &ScannedRom) -> Result<Vec<Record>> {
        if let Some(h) = rom.headerless {
            if let Match::Verified(r) | Match::CrcOnly(r) = self.identify(&h)? {
                return Ok(r);
            }
        }
        Ok(match self.identify(&rom.hashes)? {
            Match::Verified(r) | Match::CrcOnly(r) => r,
            Match::Unknown => Vec::new(),
        })
    }

    fn ident(&self, records: &[Record], sha1: &[u8]) -> Result<Ident> {
        let c = candidates(records);
        Ok(match c.as_slice() {
            [] => Ident::Unknown,
            [r] => Ident::Known(game(r)),
            _ => match self.resolution(sha1)? {
                Some((system, name)) => {
                    match c.iter().find(|r| r.system == system && r.name == name) {
                        Some(r) => Ident::Known(game(r)),
                        None => Ident::Ambiguous(c.iter().map(|r| game(r)).collect()),
                    }
                }
                None => Ident::Ambiguous(c.iter().map(|r| game(r)).collect()),
            },
        })
    }
}

fn game(r: &Record) -> Game {
    Game {
        system: r.system.clone(),
        name: r.name.clone(),
        crc: r.crc,
    }
}
