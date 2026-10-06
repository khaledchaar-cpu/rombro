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
        // `headerless` of a 2048-byte `.iso` holds its raw-sector hashes
        let hashes = disc
            .tracks
            .iter()
            .flat_map(|t| [Some(&t.hashes), t.headerless.as_ref()]);
        for h in hashes.flatten() {
            match self.identify(h)? {
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
            if let Some((disc_no, rev)) = id.variant {
                pick_variant(&mut hits, disc_no, rev);
            }
            if !hits.is_empty() {
                return Ok(DiscMatch::Serial(hits));
            }
        }
        Ok(DiscMatch::Unknown)
    }
}

/// GameCube/Wii share one game ID across revisions and discs; narrow by the header's values
/// (`(Rev 1)`, `(Disc 2)`) when that leaves at least one entry.
fn pick_variant(hits: &mut Vec<Record>, disc_no: u8, rev: u8) {
    let fits = |r: &Record| {
        let rev_ok = if rev == 0 {
            !r.name.contains("(Rev ")
        } else {
            r.name.contains(&format!("(Rev {rev})"))
        };
        let disc_ok =
            !r.name.contains("(Disc ") || r.name.contains(&format!("(Disc {})", disc_no + 1));
        rev_ok && disc_ok
    };
    if hits.iter().any(fits) {
        hits.retain(fits);
    }
}
