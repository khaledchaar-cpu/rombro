//! Identifying disc images: data-track hashes first, serial as fallback.

use crate::{Match, Record, Result, Store};
use rombro_core::ScannedDisc;

/// How a disc was matched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscMatch {
    /// A track (usually track 1; the RDBs only list the data track) matched by hash.
    Hash(Match),
    /// No track matched, but the serial read from the disc did.
    Serial(Vec<Record>),
    Unknown,
}

impl Store {
    pub fn identify_disc(&self, disc: &ScannedDisc) -> Result<DiscMatch> {
        let mut weak = None;
        for t in &disc.tracks {
            match self.identify(&t.hashes)? {
                Match::Unknown => {}
                m @ Match::Verified(_) => return Ok(DiscMatch::Hash(m)),
                m @ Match::CrcOnly(_) => {
                    weak.get_or_insert(m);
                }
            }
        }
        if let Some(m) = weak {
            return Ok(DiscMatch::Hash(m));
        }
        let Some(id) = &disc.id else {
            return Ok(DiscMatch::Unknown);
        };
        let system = Some(id.platform.system());
        for key in id.lookup_keys() {
            // Multi-disc sets are sometimes stored as `SCUS-94163-0`, `-1`, ...; include them
            // so an ambiguous serial surfaces all candidates.
            let mut hits = self.by_serial(&key, system)?;
            hits.extend(self.by_serial_prefix(&format!("{key}-"), system)?);
            if !hits.is_empty() {
                return Ok(DiscMatch::Serial(hits));
            }
        }
        Ok(DiscMatch::Unknown)
    }
}
