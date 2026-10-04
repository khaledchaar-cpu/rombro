//! Matching scanned hashes against the entry index.

use crate::{Record, Result, Store};
use rombro_core::Hashes;

/// How a ROM was matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Match {
    /// SHA1 (or MD5, when the entry has no SHA1) confirmed.
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
                (None, Some(m)) if m.as_slice() != h.md5 => {}
                (None, Some(_)) => verified.push(r),
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
