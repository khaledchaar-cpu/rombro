//! Disc images: sheets (`.cue`, `.gdi`), playlists (`.m3u`), `.iso`, and serial extraction.

pub mod cdsector;
pub mod chd;
pub mod iso9660;
pub mod nintendo;
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
    Chd,
    /// GameCube/Wii container (RVZ, WIA, WBFS, CISO): identified by game ID only.
    Nintendo,
}

impl DiscKind {
    pub fn from_ext(ext: &str) -> Option<Self> {
        match ext {
            "cue" => Some(Self::Cue),
            "gdi" => Some(Self::Gdi),
            "iso" => Some(Self::Iso),
            "chd" => Some(Self::Chd),
            e if nintendo::is_container_ext(e) => Some(Self::Nintendo),
            _ => None,
        }
    }
}

/// Track files of a disc, resolved on disk (case-insensitive fallback for sheets
/// written on Windows). Missing tracks are returned separately.
pub fn tracks(path: &Path, kind: DiscKind) -> io::Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let listed = match kind {
        DiscKind::Iso | DiscKind::Chd | DiscKind::Nintendo => {
            return Ok((vec![path.to_path_buf()], Vec::new()));
        }
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
        let ext = t
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if (ext == "iso" || nintendo::is_container_ext(&ext))
            && let Some(id) = nintendo::identify(t)?
        {
            return Ok(Some(id));
        }
        if nintendo::is_container_ext(&ext) {
            continue;
        }
        let id = if is_chd(t) {
            match chd::ChdTrack::open(t)? {
                Some(data) => serial::detect(&mut Track::open(BufReader::new(data))?)?,
                None => None,
            }
        } else {
            serial::detect(&mut Track::open(BufReader::new(File::open(t)?))?)?
        };
        if id.is_some() {
            return Ok(id);
        }
    }
    Ok(None)
}

/// A plain `.iso` sized like a CD data track in 2048-byte sectors (not a DVD image), whose
/// raw-sector form may be what the databases list.
pub fn is_cd_iso(path: &Path) -> bool {
    const CD_MAX: u64 = 900_000_000;
    fs::metadata(path).is_ok_and(|m| m.len() % cdsector::USER as u64 == 0 && m.len() <= CD_MAX)
}

/// Sheets may contain non-UTF-8 names (Shift-JIS, Latin-1); decode lossily.
/// CHD images are read through [`chd::ChdTrack`] instead of as plain files.
pub fn is_chd(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("chd"))
}

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
