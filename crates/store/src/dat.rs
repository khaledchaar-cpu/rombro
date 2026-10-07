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
        let tx = self
            .conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM dat_set WHERE system = ?1", [system])?;
        {
            let mut ins = tx.prepare(
                "INSERT OR REPLACE INTO dat_set (system, name, romof, bios, working, roms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for s in sets {
                ins.execute(params![
                    system,
                    s.name,
                    s.romof,
                    s.bios,
                    s.working,
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
        let row: Option<(Option<String>, bool, bool, String)> = self
            .conn
            .query_row(
                "SELECT romof, bios, working, roms FROM dat_set WHERE system = ?1 AND name = ?2",
                [system, name],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        row.map(|(romof, bios, working, roms)| {
            let roms: Vec<DatRom> = serde_json::from_str(&roms)?;
            Ok(DatSet {
                name: name.to_owned(),
                romof,
                bios,
                working,
                roms,
            })
        })
        .transpose()
    }

    /// Whether any loaded DAT names a set `name` (cheap check before reading an archive).
    pub fn dat_knows(&self, name: &str) -> Result<bool> {
        let mut st = self
            .conn
            .prepare_cached("SELECT 1 FROM dat_set WHERE system = ?1 AND name = ?2")?;
        for info in self.dats()? {
            if st.exists([info.system.as_str(), name])? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Reason to reject an archive without database match whose name and members belong to
    /// a set in a loaded arcade DAT that the rules exclude (e.g. MAME "not working"), as
    /// `"<core>: <reason>"` per core. `None` if no DAT knows the set or one core accepts it.
    pub fn rejected_set(&self, name: &str, members: &[(String, u32)]) -> Result<Option<String>> {
        if members.is_empty() || !self.rules()?.arcade_working_only {
            return Ok(None);
        }
        let mut reasons = Vec::new();
        for info in self.dats()? {
            let Some(set) = self.dat_set(&info.system, name)? else {
                continue;
            };
            let hits = members
                .iter()
                .filter(|(_, crc)| set.roms.iter().any(|r| r.crc == *crc))
                .count();
            // Most members suffice: dumps often carry device ROMs MAME lists elsewhere.
            if hits * 4 < members.len() * 3 {
                continue;
            }
            if set.working {
                return Ok(None);
            }
            reasons.push(format!("{0}: {name} not working in {0}", info.system));
        }
        Ok((!reasons.is_empty()).then(|| reasons.join("; ")))
    }

    /// Core (system) whose DAT names `name` as a BIOS set and knows most of `members`:
    /// re-packed BIOS zips (`neogeo.zip`, `stvbios.zip`) never match a database by whole-file
    /// hash. The best-ranked core in the placement order wins.
    pub fn bios_set(&self, name: &str, members: &[(String, u32)]) -> Result<Option<String>> {
        if members.is_empty() {
            return Ok(None);
        }
        let order = self.rules()?.arcade_order;
        let mut best: Option<String> = None;
        for info in self.dats()? {
            let Some(set) = self.dat_set(&info.system, name)? else {
                continue;
            };
            let hits = members
                .iter()
                .filter(|(_, crc)| set.roms.iter().any(|r| r.crc == *crc))
                .count();
            if !set.bios || hits * 4 < members.len() * 3 {
                continue;
            }
            let rank = |s: &str| rombro_core::arcade::rank_in(&order, s);
            if best.as_deref().is_none_or(|b| rank(&info.system) < rank(b)) {
                best = Some(info.system);
            }
        }
        Ok(best)
    }

    /// Cores (best-ranked first) whose DAT has a working set `name` that `members` and
    /// `chds` complete: identifies re-packed romsets that miss the whole-file hash.
    pub fn complete_sets(
        &self,
        name: &str,
        members: &[(String, u32)],
        chds: &[String],
        has_set: impl Fn(&str) -> bool + Copy,
    ) -> Result<Vec<String>> {
        let order = self.rules()?.arcade_order;
        let rank = |s: &str| rombro_core::arcade::rank_in(&order, s);
        let mut systems: Vec<String> = self.dats()?.into_iter().map(|d| d.system).collect();
        systems.sort_by_key(|s| rank(s));
        let mut out = Vec::new();
        for system in systems {
            if let Some(Ok(())) = self.check_set(&system, name, members, chds, has_set)? {
                out.push(system);
            }
        }
        Ok(out)
    }

    /// Checks zip `members` and the CHDs next to it (`chds`: file stems) as set `name` of
    /// `system`'s DAT. `None` if no DAT is loaded for
    /// `system`; `Err(reason)` if the set is unknown to the DAT or incomplete.
    pub fn check_set(
        &self,
        system: &str,
        name: &str,
        members: &[(String, u32)],
        chds: &[String],
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
        if !set.working && self.rules()?.arcade_working_only {
            return Ok(Some(Err(format!("{name} not working in {system}"))));
        }
        Ok(Some(
            dat::check(
                &set,
                members,
                chds,
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
        assert_eq!(s.check_set("MAME", "c", &[], &[], |_| true).unwrap(), None);
        s.import_dat("MAME", "0.289", 7, &sets).unwrap();
        s.import_dat("MAME", "0.289", 8, &sets).unwrap();
        assert_eq!(s.dats().unwrap()[0].fetched, 8);
        let m = vec![("b".to_string(), 2)];
        assert_eq!(
            s.check_set("MAME", "c", &m, &[], |n| n == "p").unwrap(),
            Some(Ok(()))
        );
        assert_eq!(
            s.check_set("MAME", "c", &m, &[], |_| false).unwrap(),
            Some(Err("parent set p missing".into()))
        );
        assert!(
            s.check_set("MAME", "x", &m, &[], |_| true)
                .unwrap()
                .unwrap()
                .is_err()
        );
    }

    #[test]
    fn not_working_sets_fail_unless_allowed() {
        let mut s = Store::open_in_memory().unwrap();
        let sets = dat::parse(
            r#"<mame><machine name="dlair2"><driver status="preliminary"/>
            <rom name="a" size="1" crc="1"/></machine></mame>"#
                .as_bytes(),
        )
        .unwrap();
        s.import_dat("MAME", "0.289", 1, &sets).unwrap();
        let m = vec![("a".to_string(), 1)];
        assert_eq!(
            s.check_set("MAME", "dlair2", &m, &[], |_| true).unwrap(),
            Some(Err("dlair2 not working in MAME".into()))
        );
        let mut rules = s.rules().unwrap();
        rules.arcade_working_only = false;
        s.set_rules(&rules).unwrap();
        assert_eq!(
            s.check_set("MAME", "dlair2", &m, &[], |_| true).unwrap(),
            Some(Ok(()))
        );
    }

    #[test]
    fn rejected_set_names_not_working_dats() {
        let mut s = Store::open_in_memory().unwrap();
        let sets = dat::parse(
            r#"<mame><machine name="scud"><driver status="preliminary"/>
            <rom name="a" size="1" crc="1"/></machine></mame>"#
                .as_bytes(),
        )
        .unwrap();
        s.import_dat("MAME", "0.289", 1, &sets).unwrap();
        let m = vec![("a".to_string(), 1)];
        assert_eq!(
            s.rejected_set("scud", &m).unwrap().as_deref(),
            Some("MAME: scud not working in MAME")
        );
        assert_eq!(s.rejected_set("scud", &[("x".into(), 9)]).unwrap(), None);
        assert_eq!(s.rejected_set("other", &m).unwrap(), None);
    }

    #[test]
    fn bios_set_by_members() {
        let mut s = Store::open_in_memory().unwrap();
        let sets = dat::parse(
            r#"<mame><machine name="neogeo" isbios="yes"><rom name="a" size="1" crc="1"/>
            <rom name="b" size="1" crc="2"/></machine>
            <machine name="game"><rom name="a" size="1" crc="1"/></machine></mame>"#
                .as_bytes(),
        )
        .unwrap();
        s.import_dat("MAME", "0.289", 1, &sets).unwrap();
        let m = vec![("a".to_string(), 1), ("b".to_string(), 2)];
        assert_eq!(s.bios_set("neogeo", &m).unwrap().as_deref(), Some("MAME"));
        assert_eq!(s.bios_set("game", &m).unwrap(), None);
        assert_eq!(s.bios_set("neogeo", &[("x".into(), 9)]).unwrap(), None);
    }

    #[test]
    fn complete_sets_by_members() {
        let mut s = Store::open_in_memory().unwrap();
        let dat = r#"<mame><machine name="1942"><rom name="a" size="1" crc="1"/>
            <rom name="b" size="1" crc="2"/></machine></mame>"#;
        let sets = dat::parse(dat.as_bytes()).unwrap();
        s.import_dat("MAME", "0.289", 1, &sets).unwrap();
        s.import_dat("FBNeo - Arcade Games", "1", 1, &sets).unwrap();
        let full = vec![("a".to_string(), 1), ("b".to_string(), 2)];
        let got = s.complete_sets("1942", &full, &[], |_| false).unwrap();
        assert_eq!(got, ["FBNeo - Arcade Games", "MAME"]);
        let part = vec![("a".to_string(), 1)];
        assert!(
            s.complete_sets("1942", &part, &[], |_| false)
                .unwrap()
                .is_empty()
        );
    }
}
