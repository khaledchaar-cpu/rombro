//! Romsets of chip-keyed arcade boards (Atomiswave, Naomi): RetroArch's database names one
//! key chip per game, so a zip holding that chip is the whole game set.

use crate::catalog::game;
use crate::{Result, Store};
use rombro_core::ScannedRom;
use rombro_core::arcade::{self, CHIP_KEYED_BIOS};
use rombro_core::plan::{Files, Game, Ident, Item};
use std::path::Path;

impl Store {
    /// The zip `whole` as a set of a chip-keyed board: a member that is the key chip of a
    /// database entry makes it that game; a known BIOS set name makes it that board's BIOS.
    pub(crate) fn chip_set(
        &self,
        members: &[ScannedRom],
        whole: &ScannedRom,
        in_library: bool,
    ) -> Result<Option<Item>> {
        let is_zip = whole
            .path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"));
        let Some(stem) = whole.path.file_stem().map(|s| s.to_string_lossy()) else {
            return Ok(None);
        };
        if !is_zip {
            return Ok(None);
        }
        let item = |ident| Item {
            files: Files::Set {
                archive: whole.path.clone(),
                chds: Vec::new(),
                alt: Vec::new(),
                dat_note: String::new(),
            },
            ident,
            in_library,
        };
        if let Some(&(_, system)) = CHIP_KEYED_BIOS
            .iter()
            .find(|(name, _)| stem.eq_ignore_ascii_case(name))
        {
            return Ok(Some(item(Ident::Bios(Game {
                name: format!("{system} BIOS"),
                system: system.to_owned(),
                crc: None,
            }))));
        }
        for rom in members {
            let Some(member) = rom.member.as_deref() else {
                continue;
            };
            let chip = file_name(member);
            let records = self.identify_rom(rom)?;
            let key = records.iter().find(|r| {
                arcade::is_chip_keyed(&r.system)
                    && r.rom_name
                        .as_deref()
                        .is_some_and(|n| file_name(n).eq_ignore_ascii_case(chip))
            });
            if let Some(r) = key {
                return Ok(Some(item(Ident::Known(game(r)))));
            }
        }
        Ok(None)
    }
}

fn file_name(p: &str) -> &str {
    Path::new(p)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(p)
}
