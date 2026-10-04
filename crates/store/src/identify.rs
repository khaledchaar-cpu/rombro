//! Matching scanned hashes against the entry index.

use crate::{Record, Result, Store};
use rombro_core::Hashes;

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

#[cfg(test)]
mod tests {
    use super::*;

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
