//! Arcade DATs (Logiqx / MAME listxml): member lists per romset, used to check whether a
//! zip is complete for a given core (SPEC F8). Checks use the zip directory only (names and
//! CRCs), nothing is unpacked.

use quick_xml::events::{BytesStart, Event};
use std::fmt;
use std::io::BufRead;

#[derive(Debug, thiserror::Error)]
pub enum DatError {
    #[error("xml: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("no romsets found")]
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DatRom {
    pub name: String,
    pub size: u64,
    pub crc: u32,
    /// Shared with the set named by `romof` (parent or BIOS); absent from split sets.
    pub merge: bool,
    /// A `<disk>` (CHD next to the zip, `<set>/<name>.chd`) rather than a zip member.
    #[serde(default)]
    pub disk: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatSet {
    pub name: String,
    pub romof: Option<String>,
    pub bios: bool,
    /// The emulator runs it: false for driver status `preliminary` (MAME's "not working").
    pub working: bool,
    pub roms: Vec<DatRom>,
}

/// Parses `<game>`/`<machine>` entries; ROMs without CRC or marked `nodump` are left out.
pub fn parse(input: impl BufRead) -> Result<Vec<DatSet>, DatError> {
    let mut xml = quick_xml::Reader::from_reader(input);
    let mut buf = Vec::new();
    let mut sets = Vec::new();
    let mut cur: Option<DatSet> = None;
    loop {
        match xml.read_event_into(&mut buf)? {
            Event::Start(e) | Event::Empty(e) => match e.local_name().as_ref() {
                b"game" | b"machine" => {
                    if let Some(s) = cur.take() {
                        sets.push(s);
                    }
                    cur = Some(DatSet {
                        name: attr(&e, b"name").unwrap_or_default(),
                        romof: attr(&e, b"romof"),
                        bios: attr(&e, b"isbios").as_deref() == Some("yes"),
                        working: true,
                        roms: Vec::new(),
                    });
                }
                b"driver" => {
                    if let Some(s) = cur.as_mut()
                        && attr(&e, b"status").as_deref() == Some("preliminary")
                    {
                        s.working = false;
                    }
                }
                b"rom" => {
                    if let Some(s) = cur.as_mut()
                        && attr(&e, b"status").as_deref() != Some("nodump")
                        && let Some(crc) =
                            attr(&e, b"crc").and_then(|c| u32::from_str_radix(&c, 16).ok())
                    {
                        s.roms.push(DatRom {
                            name: attr(&e, b"name").unwrap_or_default(),
                            size: attr(&e, b"size").and_then(|v| v.parse().ok()).unwrap_or(0),
                            crc,
                            merge: attr(&e, b"merge").is_some(),
                            disk: false,
                        });
                    }
                }
                b"disk" => {
                    if let Some(s) = cur.as_mut()
                        && attr(&e, b"status").as_deref() != Some("nodump")
                    {
                        s.roms.push(DatRom {
                            name: attr(&e, b"name").unwrap_or_default(),
                            size: 0,
                            crc: 0,
                            merge: attr(&e, b"merge").is_some(),
                            disk: true,
                        });
                    }
                }
                _ => {}
            },
            Event::End(e) if matches!(e.local_name().as_ref(), b"game" | b"machine") => {
                sets.extend(cur.take());
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    sets.extend(cur);
    if sets.is_empty() {
        return Err(DatError::Empty);
    }
    Ok(sets)
}

fn attr(e: &BytesStart, key: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == key)
        .and_then(|a| {
            a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()
                .map(|v| v.into_owned())
        })
}

/// Why a zip is not usable for a core.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Incomplete {
    pub missing: Vec<String>,
    /// Present by CRC but under another name.
    pub misnamed: Vec<String>,
    /// Split set whose parent set is not next to it.
    pub parent: Option<String>,
}

impl fmt::Display for Incomplete {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let list = |v: &[String]| {
            let mut s = v.iter().take(3).cloned().collect::<Vec<_>>().join(", ");
            if v.len() > 3 {
                s.push_str(", …");
            }
            s
        };
        let mut parts = Vec::new();
        if !self.missing.is_empty() {
            parts.push(format!(
                "{} missing ({})",
                self.missing.len(),
                list(&self.missing)
            ));
        }
        if !self.misnamed.is_empty() {
            parts.push(format!(
                "{} misnamed ({})",
                self.misnamed.len(),
                list(&self.misnamed)
            ));
        }
        if let Some(p) = &self.parent {
            parts.push(format!("parent set {p} missing"));
        }
        f.write_str(&parts.join(", "))
    }
}

/// Checks a zip's `members` (name, CRC) and the CHDs next to it (`chds`: file stems in the
/// set's folder) against `set`. Merged ROMs not in the zip must come from the `romof` chain:
/// BIOS sets are placed separately (not checked), any other owner must be present
/// (`has_set`). Extra members (merged clones) are fine; merged disks live with the parent.
pub fn check<'a>(
    set: &DatSet,
    members: &[(String, u32)],
    chds: &[String],
    resolve: impl Fn(&str) -> Option<&'a DatSet>,
    has_set: impl Fn(&str) -> bool,
) -> Result<(), Incomplete> {
    let mut bad = Incomplete::default();
    for disk in set.roms.iter().filter(|r| r.disk && !r.merge) {
        if !chds.iter().any(|c| c.eq_ignore_ascii_case(&disk.name)) {
            bad.missing.push(format!("{}.chd", disk.name));
        }
    }
    for rom in set.roms.iter().filter(|r| !r.disk) {
        if members
            .iter()
            .any(|(n, c)| *c == rom.crc && n.eq_ignore_ascii_case(&rom.name))
        {
            continue;
        }
        if rom.merge
            && let Some(owner) = owner(set, rom.crc, &resolve)
        {
            if !owner.bios && !has_set(&owner.name) && bad.parent.is_none() {
                bad.parent = Some(owner.name.clone());
            }
            continue;
        }
        if members.iter().any(|(_, c)| *c == rom.crc) {
            bad.misnamed.push(rom.name.clone());
        } else {
            bad.missing.push(rom.name.clone());
        }
    }
    if bad == Incomplete::default() {
        Ok(())
    } else {
        Err(bad)
    }
}

/// The set up the `romof` chain that holds `crc` itself (not merged from further up).
fn owner<'a>(
    set: &DatSet,
    crc: u32,
    resolve: &impl Fn(&str) -> Option<&'a DatSet>,
) -> Option<&'a DatSet> {
    let mut next = set.romof.clone();
    for _ in 0..8 {
        let s = resolve(next.as_deref()?)?;
        if s.roms.iter().any(|r| r.crc == crc && !r.merge && !r.disk) {
            return Some(s);
        }
        next = s.romof.clone();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAT: &str = r#"<?xml version="1.0"?>
<datafile>
  <game name="neogeo" isbios="yes"><rom name="sp-s2.sp1" size="4" crc="9036d879"/></game>
  <game name="1943"><rom name="a.bin" size="4" crc="00000001"/><rom name="b.bin" size="4" crc="00000002"/>
    <rom name="nd.bin" size="4" status="nodump"/></game>
  <game name="1943j" cloneof="1943" romof="1943">
    <rom name="a.bin" merge="a.bin" size="4" crc="00000001"/><rom name="j.bin" size="4" crc="00000003"/></game>
  <machine name="mslug" romof="neogeo"><rom name="sp-s2.sp1" merge="sp-s2.sp1" size="4" crc="9036d879"/>
    <rom name="m.bin" size="4" crc="00000004"/></machine>
</datafile>"#;

    fn sets() -> Vec<DatSet> {
        parse(DAT.as_bytes()).unwrap()
    }

    fn run(name: &str, members: &[(&str, u32)], present: &[&str]) -> Result<(), Incomplete> {
        let sets = sets();
        let set = sets.iter().find(|s| s.name == name).unwrap();
        let m: Vec<(String, u32)> = members.iter().map(|(n, c)| (n.to_string(), *c)).collect();
        check(
            set,
            &m,
            &[],
            |n| sets.iter().find(|s| s.name == n),
            |n| present.contains(&n),
        )
    }

    #[test]
    fn parses_games_machines_and_skips_nodumps() {
        let s = sets();
        assert_eq!(s.len(), 4);
        assert!(s[0].bios);
        assert_eq!(s[1].roms.len(), 2);
        assert_eq!(s[2].romof.as_deref(), Some("1943"));
        assert!(s[2].roms[0].merge && !s[2].roms[1].merge);
        assert_eq!(s[3].roms[1].crc, 4);
    }

    #[test]
    fn complete_split_merged_and_nonmerged() {
        assert_eq!(run("1943", &[("A.BIN", 1), ("b.bin", 2)], &[]), Ok(()));
        // merged parent zip carries clone files too
        assert_eq!(
            run("1943", &[("a.bin", 1), ("b.bin", 2), ("j.bin", 3)], &[]),
            Ok(())
        );
        // split clone with parent next to it, non-merged clone without
        assert_eq!(run("1943j", &[("j.bin", 3)], &["1943"]), Ok(()));
        assert_eq!(run("1943j", &[("a.bin", 1), ("j.bin", 3)], &[]), Ok(()));
        // BIOS ROMs are not required in the set
        assert_eq!(run("mslug", &[("m.bin", 4)], &[]), Ok(()));
    }

    #[test]
    fn reports_missing_misnamed_and_parent() {
        let e = run("1943", &[("x.bin", 1)], &[]).unwrap_err();
        assert_eq!(
            (e.misnamed, e.missing),
            (vec!["a.bin".into()], vec!["b.bin".into()])
        );
        let e = run("1943j", &[("j.bin", 3)], &[]).unwrap_err();
        assert_eq!(e.parent.as_deref(), Some("1943"));
        assert_eq!(e.to_string(), "parent set 1943 missing");
    }

    #[test]
    fn laserdisc_set_needs_its_chd() {
        let dat = r#"<mame><machine name="lair2"><driver status="preliminary"/>
            <rom name="lair2.bin" size="4" crc="00000009"/>
            <disk name="lair2" sha1="00"/><disk name="nd" status="nodump"/></machine></mame>"#;
        let sets = parse(dat.as_bytes()).unwrap();
        assert!(!sets[0].working && sets.len() == 1);
        let m = [("lair2.bin".to_string(), 9)];
        let run = |chds: &[String]| check(&sets[0], &m, chds, |_| None, |_| false);
        assert_eq!(run(&[]).unwrap_err().missing, ["lair2.chd"]);
        assert_eq!(run(&["LAIR2".to_string()]), Ok(()));
    }
}
