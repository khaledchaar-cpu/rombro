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
            // MAME cores listing it as BIOS get a copy next to their sets (e.g. `rumblef`)
            let crcs: Vec<(String, u32)> = members
                .iter()
                .filter_map(|r| Some((r.member.clone()?, r.hashes.crc)))
                .collect();
            let alt = self
                .bios_systems(&stem, &crcs)?
                .into_iter()
                .map(|system| Game {
                    system,
                    name: stem.clone().into_owned(),
                    crc: None,
                })
                .collect();
            return Ok(Some(Item {
                files: Files::Set {
                    archive: whole.path.clone(),
                    chds: Vec::new(),
                    alt,
                    dat_note: String::new(),
                },
                ident: Ident::Bios(Game {
                    name: format!("{system} BIOS"),
                    system: system.to_owned(),
                    crc: None,
                }),
                in_library,
            }));
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
        // RetroArch's database lists few of these boards' games (mostly GD-ROMs); a cart set
        // MAME knows on such a board runs on Flycast, whatever MAME's driver status says
        let crcs: Vec<u32> = members
            .iter()
            .filter(|r| r.member.is_some())
            .map(|r| r.hashes.crc)
            .collect();
        if let Some(system) = self.dat_board_set(&stem, &crcs)? {
            return Ok(Some(item(Ident::Known(Game {
                system: system.to_owned(),
                name: stem.into_owned(),
                crc: None,
            }))));
        }
        Ok(None)
    }

    /// The chip-keyed board (Flycast system) of arcade set `name`, if a loaded DAT has the
    /// set on that board's BIOS (via its `romof` chain) and `members` hold most of its ROMs.
    pub(crate) fn dat_board_set(&self, name: &str, crcs: &[u32]) -> Result<Option<&'static str>> {
        if crcs.is_empty() {
            return Ok(None);
        }
        for info in self.dats()? {
            let Some(set) = self.dat_set(&info.system, name)? else {
                continue;
            };
            let hits = crcs
                .iter()
                .filter(|c| set.roms.iter().any(|r| r.crc == **c))
                .count();
            // as in `rejected_set`: dumps often carry device ROMs listed elsewhere
            if hits * 4 < crcs.len() * 3 {
                continue;
            }
            let mut romof = set.romof;
            for _ in 0..4 {
                let Some(parent) = romof else { break };
                if let Some(&(_, system)) = CHIP_KEYED_BIOS
                    .iter()
                    .find(|(bios, _)| parent.eq_ignore_ascii_case(bios))
                {
                    return Ok(Some(system));
                }
                romof = self.dat_set(&info.system, &parent)?.and_then(|s| s.romof);
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

#[cfg(test)]
mod tests {
    use crate::Store;
    use rombro_core::plan::{Files, Ident};
    use std::io::Write;

    #[test]
    fn cart_set_on_naomi_bios_is_naomi_even_if_mame_marks_it_not_working() {
        let tmp = tempfile::tempdir().unwrap();
        let mut w =
            zip::ZipWriter::new(std::fs::File::create(tmp.path().join("crzytaxi.zip")).unwrap());
        for (m, data) in [("epr-21684.ic22", &b"boot"[..]), ("mpr-21671.ic1", b"gfx")] {
            w.start_file(m, zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
        let crc = |d: &[u8]| crc32fast::hash(d);
        let dat = format!(
            r#"<mame>
            <machine name="naomi" isbios="yes"><rom name="b" size="1" crc="00000001"/></machine>
            <machine name="crzytaxi" romof="naomi"><driver status="preliminary"/>
              <rom name="epr-21684.ic22" size="4" crc="{:08x}"/>
              <rom name="mpr-21671.ic1" size="3" crc="{:08x}"/></machine></mame>"#,
            crc(b"boot"),
            crc(b"gfx")
        );
        let mut s = Store::open_in_memory().unwrap();
        let sets = rombro_core::arcade::dat::parse(dat.as_bytes()).unwrap();
        s.import_dat("MAME", "0.289", 1, &sets).unwrap();
        let items = s.items(&rombro_core::scan(tmp.path()), false).unwrap();
        assert_eq!(items.len(), 1);
        assert!(matches!(items[0].files, Files::Set { .. }));
        assert!(
            matches!(&items[0].ident, Ident::Known(g) if g.system == "Sega - Naomi" && g.name == "crzytaxi"),
            "{:?}",
            items[0].ident
        );
    }
}
