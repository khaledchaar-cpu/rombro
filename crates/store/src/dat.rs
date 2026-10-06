//! Arcade DATs per core: stored member lists and the completeness check (SPEC F8).

use crate::{Result, Store};
use rombro_core::arcade::dat::{self, DatRom, DatSet};
use rusqlite::{OptionalExtension, params};

/// A loaded DAT: core (system), source version, unix time of the download.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DatInfo {
    pub system: String,
    pub version: String,
    pub fetched: i64,
}

impl Store {
    /// Replaces the DAT of `system`.
    pub fn import_dat(
        &mut self,
        system: &str,
        version: &str,
        fetched: i64,
        sets: &[DatSet],
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM dat_set WHERE system = ?1", [system])?;
        {
            let mut ins = tx.prepare(
                "INSERT OR REPLACE INTO dat_set (system, name, romof, bios, roms) VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for s in sets {
                ins.execute(params![
                    system,
                    s.name,
                    s.romof,
                    s.bios,
                    serde_json::to_string(&s.roms)?
                ])?;
            }
        }
        tx.execute(
            "INSERT OR REPLACE INTO dat_source (system, version, fetched) VALUES (?1, ?2, ?3)",
            params![system, version, fetched],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn dats(&self) -> Result<Vec<DatInfo>> {
        let mut st = self
            .conn
            .prepare("SELECT system, version, fetched FROM dat_source ORDER BY system")?;
        let rows = st.query_map([], |r| {
            Ok(DatInfo {
                system: r.get(0)?,
                version: r.get(1)?,
                fetched: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn dat_set(&self, system: &str, name: &str) -> Result<Option<DatSet>> {
        let row: Option<(Option<String>, bool, String)> = self
            .conn
            .query_row(
                "SELECT romof, bios, roms FROM dat_set WHERE system = ?1 AND name = ?2",
                [system, name],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        row.map(|(romof, bios, roms)| {
            let roms: Vec<DatRom> = serde_json::from_str(&roms)?;
            Ok(DatSet {
                name: name.to_owned(),
                romof,
                bios,
                roms,
            })
        })
        .transpose()
    }

    /// Checks zip `members` as set `name` of `system`'s DAT. `None` if no DAT is loaded for
    /// `system`; `Err(reason)` if the set is unknown to the DAT or incomplete.
    pub fn check_set(
        &self,
        system: &str,
        name: &str,
        members: &[(String, u32)],
        has_set: impl Fn(&str) -> bool,
    ) -> Result<Option<std::result::Result<(), String>>> {
        let loaded: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM dat_source WHERE system = ?1",
                [system],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !loaded {
            return Ok(None);
        }
        let Some(set) = self.dat_set(system, name)? else {
            return Ok(Some(Err(format!("set {name} not in {system} DAT"))));
        };
        let mut chain: Vec<DatSet> = Vec::new();
        let mut next = set.romof.clone();
        while let Some(n) = next.take() {
            if chain.len() >= 8 || chain.iter().any(|s| s.name == n) {
                break;
            }
            if let Some(s) = self.dat_set(system, &n)? {
                next = s.romof.clone();
                chain.push(s);
            }
        }
        Ok(Some(
            dat::check(
                &set,
                members,
                |n| chain.iter().find(|s| s.name == n),
                has_set,
            )
            .map_err(|e| e.to_string()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_and_checks() {
        let mut s = Store::open_in_memory().unwrap();
        let sets = dat::parse(
            r#"<datafile><game name="p"><rom name="a" size="1" crc="1"/></game>
            <game name="c" romof="p"><rom name="a" merge="a" size="1" crc="1"/><rom name="b" size="1" crc="2"/></game></datafile>"#
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(s.check_set("MAME", "c", &[], |_| true).unwrap(), None);
        s.import_dat("MAME", "0.289", 7, &sets).unwrap();
        s.import_dat("MAME", "0.289", 8, &sets).unwrap();
        assert_eq!(s.dats().unwrap()[0].fetched, 8);
        let m = vec![("b".to_string(), 2)];
        assert_eq!(
            s.check_set("MAME", "c", &m, |n| n == "p").unwrap(),
            Some(Ok(()))
        );
        assert_eq!(
            s.check_set("MAME", "c", &m, |_| false).unwrap(),
            Some(Err("parent set p missing".into()))
        );
        assert!(
            s.check_set("MAME", "x", &m, |_| true)
                .unwrap()
                .unwrap()
                .is_err()
        );
    }
}
