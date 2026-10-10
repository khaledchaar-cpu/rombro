//! Matching scanned hashes against the entry index.

use crate::{Record, Result, Store};
use romburak_core::Hashes;
use std::path::Path;

/// How a ROM was matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Match {
    /// SHA1 (or MD5, when the entry has no SHA1 and MD5 was computed) confirmed.
    Verified(Vec<Record>),
    /// CRC32 + size match; the entry has no stronger hash to confirm with.
    CrcOnly(Vec<Record>),
    Unknown,
}

impl Store {
    /// Identifies a ROM: CRC+size candidates, confirmed by SHA1/MD5 where the entry has them.
    /// Candidates whose stored SHA1/MD5 contradicts the ROM are discarded.
    pub fn identify(&self, h: &Hashes) -> Result<Match> {
        let mut verified = Vec::new();
        let mut weak = Vec::new();
        for r in self.by_crc(h.crc, Some(h.size))? {
            match (&r.sha1, &r.md5) {
                (Some(s), _) if s.as_slice() != h.sha1 => {}
                (Some(_), _) => verified.push(r),
                (None, Some(m)) => match h.md5 {
                    Some(x) if m.as_slice() != x => {}
                    Some(_) => verified.push(r),
                    None => weak.push(r),
                },
                (None, None) => weak.push(r),
            }
        }
        if verified.is_empty() {
            // Entries without CRC (rare) can still match by SHA1.
            verified = self.by_sha1(&h.sha1)?;
            verified.retain(|r| r.size.is_none_or(|s| s == h.size));
        }
        Ok(if !verified.is_empty() {
            Match::Verified(verified)
        } else if !weak.is_empty() {
            Match::CrcOnly(weak)
        } else {
            Match::Unknown
        })
    }
}

/// Distinct games among match records (same system + name counts once), in input order.
/// More than one means the match is ambiguous and the user has to decide.
pub fn candidates(records: &[Record]) -> Vec<&Record> {
    let mut out: Vec<&Record> = Vec::new();
    for r in records {
        if !out.iter().any(|o| o.system == r.system && o.name == r.name) {
            out.push(r);
        }
    }
    out
}

/// Among several candidates, the one the file itself points to: the only one whose ROM name
/// has the file's extension (`.a52` = Atari 5200, not the 8-bit `.bin`), else the only one
/// whose system names a folder the file is in. `name` is the file (or archive member) name,
/// `at` its location.
pub fn pick_by_file<'r>(c: &[&'r Record], name: &Path, at: &Path) -> Option<&'r Record> {
    let only = |hits: Vec<&'r Record>| match hits.as_slice() {
        [r] => Some(*r),
        _ => None,
    };
    let ext = name
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    let by_ext = ext.and_then(|ext| {
        only(
            c.iter()
                .copied()
                .filter(|r| {
                    r.rom_name.as_deref().is_some_and(|n| {
                        Path::new(n)
                            .extension()
                            .is_some_and(|e| e.eq_ignore_ascii_case(&ext))
                    })
                })
                .collect(),
        )
    });
    by_ext.or_else(|| {
        only(
            c.iter()
                .copied()
                .filter(|r| {
                    at.ancestors()
                        .any(|a| a.file_name().is_some_and(|n| n == r.system.as_str()))
                })
                .collect(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_by_file_uses_extension_then_folder() {
        let r = |system: &str, rom: &str| Record {
            system: system.into(),
            name: rom.into(),
            rom_name: Some(rom.into()),
            ..Record::default()
        };
        let (a52, bin) = (
            r("Atari - 5200", "Miner.a52"),
            r("Atari - 8-bit Family", "Miner.bin"),
        );
        let c = [&a52, &bin];
        let lib = Path::new("/lib/Inbox/x.zip");
        assert_eq!(
            pick_by_file(&c, Path::new("m.A52"), lib).map(|r| &r.system),
            Some(&a52.system)
        );
        // extension fits neither: the folder decides
        let at = Path::new("/lib/Atari - 8-bit Family/m.zip");
        assert_eq!(
            pick_by_file(&c, Path::new("m.rom"), at).map(|r| &r.system),
            Some(&bin.system)
        );
        assert!(pick_by_file(&c, Path::new("m.rom"), lib).is_none());
        // same extension on both: still ambiguous
        let bin2 = r("Atari - 5200", "Other.bin");
        assert!(pick_by_file(&[&bin, &bin2], Path::new("m.bin"), lib).is_none());
    }

    #[test]
    fn candidates_dedup_by_system_and_name() {
        let r = |system: &str, name: &str| Record {
            system: system.into(),
            name: name.into(),
            ..Record::default()
        };
        let recs = [r("A", "x"), r("A", "x"), r("B", "x"), r("A", "y")];
        let c: Vec<_> = candidates(&recs)
            .iter()
            .map(|r| (&*r.system, &*r.name))
            .collect();
        assert_eq!(c, [("A", "x"), ("B", "x"), ("A", "y")]);
    }
}
