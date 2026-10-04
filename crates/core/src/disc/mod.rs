//! Disc images: sheets (`.cue`, `.gdi`), playlists (`.m3u`), `.iso`, and serial extraction.

pub mod iso9660;
pub mod serial;
pub mod sheet;

pub use serial::{DiscId, Platform};

use iso9660::Track;
use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscKind {
    Cue,
    Gdi,
    Iso,
}

impl DiscKind {
    pub fn from_ext(ext: &str) -> Option<Self> {
        match ext {
            "cue" => Some(Self::Cue),
            "gdi" => Some(Self::Gdi),
            "iso" => Some(Self::Iso),
            _ => None,
        }
    }
}

/// Track files of a disc, resolved on disk (case-insensitive fallback for sheets
/// written on Windows). Missing tracks are returned separately.
pub fn tracks(path: &Path, kind: DiscKind) -> io::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let listed = match kind {
        DiscKind::Iso => return Ok((vec![path.to_path_buf()], Vec::new())),
        DiscKind::Cue => sheet::parse_cue(&read_text(path)?, dir),
        DiscKind::Gdi => sheet::parse_gdi(&read_text(path)?, dir),
    };
    let (mut found, mut missing) = (Vec::new(), Vec::new());
    for p in listed {
        match resolve(&p) {
            Some(r) => found.push(r),
            None => missing.push(p),
        }
    }
    Ok((found, missing))
}

/// Detects platform and serial from the first track that yields one.
pub fn identify(tracks: &[PathBuf]) -> io::Result<Option<DiscId>> {
    for t in tracks {
        let mut track = Track::open(BufReader::new(File::open(t)?))?;
        if let Some(id) = serial::detect(&mut track)? {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// Sheets may contain non-UTF-8 names (Shift-JIS, Latin-1); decode lossily.
pub fn read_text(path: &Path) -> io::Result<String> {
    Ok(String::from_utf8_lossy(&fs::read(path)?).into_owned())
}

fn resolve(p: &Path) -> Option<PathBuf> {
    if p.is_file() {
        return Some(p.to_path_buf());
    }
    let name = p.file_name()?.to_str()?;
    fs::read_dir(p.parent()?)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|c| {
            c.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })
}
