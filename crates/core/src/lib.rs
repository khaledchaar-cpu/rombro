//! rombro-core: domain logic (hashing, scanning, naming, 1G1R, planner).

pub mod hash;
pub mod header;
pub mod scan;

pub use hash::{Hashes, MultiHasher, hash_reader};
pub use scan::{ScanReport, ScannedRom, scan, scan_file};
