//! rombro-core: domain logic (hashing, scanning, naming, 1G1R, planner).

pub mod disc;
pub mod g1r;
pub mod hash;
pub mod header;
pub mod naming;
pub mod scan;

pub use hash::{Hashes, MultiHasher, hash_reader};
pub use scan::{Playlist, ScanReport, ScannedDisc, ScannedRom, scan, scan_disc, scan_file};
