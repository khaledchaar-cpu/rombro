//! Turning a scan report into planner items (identification + stored resolutions).

use crate::{DiscMatch, Match, Record, Result, Store, candidates};
use rombro_core::arcade;
use rombro_core::disc::{self, DiscKind};
use rombro_core::plan::{Files, Game, Ident, Item};
use rombro_core::{ScanReport, ScannedDisc, ScannedRom};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

impl Store {
    /// Identifies every scanned ROM and disc. Each ROM of a multi-ROM archive becomes its own
    /// item; discs with missing tracks are skipped; ambiguous matches use a stored resolution if there is one.
    pub fn items(&self, report: &ScanReport, in_library: bool) -> Result<Vec<Item>> {
        let mut out = Vec::new();
        let mut sets = HashSet::new();
        for whole in &report.archives {
            if let Some(item) = self.romset(whole, in_library)? {
                sets.insert(whole.path.as_path());
                out.push(item);
            }
        }
        for group in report.roms.chunk_by(|a, b| a.path == b.path) {
            if sets.contains(group[0].path.as_path()) {
                continue;
            }
            if let Some(item) = self.archived_disc(group, in_library)? {
                out.push(item);
                continue;
            }
            for rom in group {
                let files = match (&rom.member, group.len()) {
                    (Some(member), 2..) => Files::Member {
                        archive: rom.path.clone(),
                        member: member.clone(),
                    },
                    _ => Files::Single(rom.path.clone()),
                };
                out.push(Item {
                    files,
                    ident: self.ident(&self.identify_rom(rom)?, &rom.hashes.sha1)?,
                    in_library,
                });
            }
        }
        let attached: HashSet<&PathBuf> = out
            .iter()
            .filter_map(|it| match &it.files {
                Files::Set { chds, .. } => Some(chds),
                _ => None,
            })
            .flatten()
            .collect();
        let discs: Vec<&ScannedDisc> = report
            .discs
            .iter()
            .filter(|d| !attached.contains(&d.path))
            .collect();
        for d in discs {
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

    /// An archive whose whole-file hash matches an entry (arcade romset): the best-ranked
    /// system wins (FBNeo, newest MAME, …); BIOS sets get their own ident.
    fn romset(&self, whole: &ScannedRom, in_library: bool) -> Result<Option<Item>> {
        let mut records = match self.identify(&whole.hashes)? {
            Match::Verified(r) | Match::CrcOnly(r) => r,
            Match::Unknown => return Ok(None),
        };
        let best = records
            .iter()
            .map(|r| arcade::rank(&r.system))
            .min()
            .unwrap_or_default();
        records.retain(|r| arcade::rank(&r.system) == best);
        let bios = records
            .iter()
            .any(|r| arcade::is_bios(r.rom_name.as_deref(), &r.name));
        let ident = match self.ident(&records, &whole.hashes.sha1)? {
            Ident::Known(g) if bios => Ident::Bios(g),
            i => i,
        };
        Ok(Some(Item {
            files: Files::Set {
                archive: whole.path.clone(),
                chds: set_chds(&whole.path),
            },
            ident,
            in_library,
        }))
    }

    /// An archive holding a disc sheet becomes one disc item (identified by its track hashes;
    /// further sheets or loose members in the same archive are ignored).
    fn archived_disc(&self, group: &[ScannedRom], in_library: bool) -> Result<Option<Item>> {
        let sheets: Vec<(&ScannedRom, DiscKind)> = group
            .iter()
            .filter_map(|r| {
                let ext = Path::new(r.member.as_deref()?).extension()?;
                Some((
                    r,
                    DiscKind::from_ext(&ext.to_string_lossy().to_lowercase())?,
                ))
            })
            .filter(|(_, k)| matches!(k, DiscKind::Cue | DiscKind::Gdi))
            .collect();
        let Some(&(sheet, kind)) = sheets.first() else {
            return Ok(None);
        };
        let (archive, sheet_name) = (&sheet.path, sheet.member.clone().unwrap_or_default());
        let item = |files, ident| Item {
            files,
            ident,
            in_library,
        };
        let skip = |reason: String| {
            Ok(Some(item(
                Files::Single(archive.clone()),
                Ident::Skip(reason),
            )))
        };
        if sheets.len() > 1 {
            return skip(format!("archive with {} disc sheets", sheets.len()));
        }
        let text = match rombro_core::archive::read_member(archive, &sheet_name) {
            Ok(b) => String::from_utf8_lossy(&b).into_owned(),
            Err(e) => return skip(format!("cannot read sheet: {e}")),
        };
        let dir = Path::new(&sheet_name).parent().unwrap_or(Path::new(""));
        let listed = match kind {
            DiscKind::Gdi => disc::sheet::parse_gdi(&text, dir),
            _ => disc::sheet::parse_cue(&text, dir),
        };
        let mut tracks = Vec::new();
        for want in &listed {
            let want = want.to_string_lossy().to_lowercase();
            match group.iter().find(|r| {
                r.member
                    .as_deref()
                    .is_some_and(|m| m.to_lowercase() == want)
            }) {
                Some(r) => tracks.push(r.clone()),
                None => return skip(format!("track missing in archive: {want}")),
            }
        }
        let scanned = ScannedDisc {
            path: archive.clone(),
            kind,
            id: None,
            tracks,
            missing: Vec::new(),
        };
        let records = match self.identify_disc(&scanned)? {
            DiscMatch::Hash(Match::Verified(r) | Match::CrcOnly(r)) | DiscMatch::Serial(r) => r,
            DiscMatch::Hash(Match::Unknown) | DiscMatch::Unknown => Vec::new(),
        };
        let sha1 = scanned
            .tracks
            .first()
            .map(|t| t.hashes.sha1)
            .unwrap_or_default();
        let files = Files::ArchivedSheet {
            archive: archive.clone(),
            sheet: sheet_name,
            tracks: scanned
                .tracks
                .into_iter()
                .filter_map(|t| t.member)
                .collect(),
        };
        Ok(Some(item(files, self.ident(&records, &sha1)?)))
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
            [r] if r.name.starts_with("[BIOS]") => Ident::Bios(game(r)),
            [r] => Ident::Known(game(r)),
            _ => match self.resolution(sha1)? {
                Some((system, name)) => {
                    match c.iter().find(|r| r.system == system && r.name == name) {
                        Some(r) if r.name.starts_with("[BIOS]") => Ident::Bios(game(r)),
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

/// CHDs in the folder named like the romset (`kinst.zip` → `kinst/*.chd`), sorted.
fn set_chds(archive: &Path) -> Vec<PathBuf> {
    let Ok(dir) = std::fs::read_dir(archive.with_extension("")) else {
        return Vec::new();
    };
    let mut chds: Vec<PathBuf> = dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("chd")))
        .collect();
    chds.sort();
    chds
}
