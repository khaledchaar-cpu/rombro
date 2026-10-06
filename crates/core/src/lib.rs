//! rombro-core: domain logic (hashing, scanning, naming, 1G1R, planner).

pub mod arcade;
pub mod archive;
pub mod cache;
pub mod disc;
pub mod g1r;
pub mod gamify;
pub mod hash;
pub mod header;
pub mod naming;
pub mod paths;
pub mod plan;
pub mod rules;
pub mod scan;
pub mod thumbnail;

pub use cache::{CachedRom, HashCache, Stamp};
pub use hash::{Hashes, MultiHasher, hash_reader};
pub use scan::{
    Playlist, ScanReport, ScannedDisc, ScannedRom, scan, scan_cached, scan_disc, scan_file,
    scan_with_progress,
};
