//! Hash cache for incremental scans: files whose size and mtime are unchanged are not rehashed.

use crate::ScannedRom;
use crate::hash::Hashes;
use crate::header::Header;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Size and modification time (ns since epoch) identifying a file version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub size: u64,
    pub mtime: i64,
}

impl Stamp {
    pub fn of(path: &Path) -> io::Result<Self> {
        let m = std::fs::metadata(path)?;
        let mtime = m
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);
        Ok(Self {
            size: m.len(),
            mtime,
        })
    }
}

/// Hash result of one ROM without its path (the cache key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedRom {
    pub member: Option<String>,
    pub hashes: Hashes,
    pub header: Option<Header>,
    pub headerless: Option<Hashes>,
}

impl CachedRom {
    pub fn from_rom(r: &ScannedRom) -> Self {
        Self {
            member: r.member.clone(),
            hashes: r.hashes,
            header: r.header,
            headerless: r.headerless,
        }
    }

    pub fn into_rom(self, path: &Path) -> ScannedRom {
        ScannedRom {
            path: path.to_path_buf(),
            member: self.member,
            hashes: self.hashes,
            header: self.header,
            headerless: self.headerless,
        }
    }
}

/// Cached hashes per file path.
#[derive(Debug, Default)]
pub struct HashCache {
    pub entries: HashMap<PathBuf, (Stamp, Vec<CachedRom>)>,
    /// Trust entries without checking each file's size and mtime: the folder listing still
    /// finds added and removed files, but a file overwritten in place goes unnoticed. For
    /// the library, which only RomBro changes (its executions keep the index current);
    /// a full rescan checks every file.
    pub trusted: bool,
}

impl HashCache {
    pub fn new(entries: HashMap<PathBuf, (Stamp, Vec<CachedRom>)>) -> Self {
        Self {
            entries,
            trusted: false,
        }
    }

    /// Cached ROMs for `path` if the file is unchanged since it was hashed (or trusted).
    pub fn get(&self, path: &Path) -> Option<Vec<ScannedRom>> {
        let (stamp, roms) = self.entries.get(path)?;
        if !self.trusted && Stamp::of(path).ok()? != *stamp {
            return None;
        }
        Some(roms.iter().cloned().map(|r| r.into_rom(path)).collect())
    }

    /// Size of `path`: the trusted cached one, else from the file system.
    pub fn size(&self, path: &Path) -> Option<u64> {
        match self.entries.get(path) {
            Some((stamp, _)) if self.trusted => Some(stamp.size),
            _ => std::fs::metadata(path).ok().map(|m| m.len()),
        }
    }

    /// Whether `path` is served without touching the file system.
    pub fn trusts(&self, path: &Path) -> bool {
        self.trusted && self.entries.contains_key(path)
    }
}
