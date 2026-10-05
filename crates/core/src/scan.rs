//! Parallel directory scan: hashes plain files and archive members (ZIP, 7z).

use crate::cache::HashCache;
use crate::disc::{self, DiscId, DiscKind};
use crate::hash::{Hashes, hash_reader};
use crate::header::{self, Header};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use walkdir::WalkDir;

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("zip: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("7z: {0}")]
    SevenZ(#[from] sevenz_rust2::Error),
    #[error("walk: {0}")]
    Walk(#[from] walkdir::Error),
}

/// One hashed ROM: a plain file or a member of an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedRom {
    pub path: PathBuf,
    /// Member name inside the archive at `path`.
    pub member: Option<String>,
    pub hashes: Hashes,
    pub header: Option<Header>,
    /// Hashes without the detected header.
    pub headerless: Option<Hashes>,
}

/// A file (or archive) that could not be scanned.
#[derive(Debug)]
pub struct ScanFailure {
    pub path: PathBuf,
    pub error: ScanError,
}

/// A disc image (`.cue`/`.gdi` sheet with its tracks, or a single `.iso`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedDisc {
    pub path: PathBuf,
    pub kind: DiscKind,
    pub id: Option<DiscId>,
    pub tracks: Vec<ScannedRom>,
    /// Tracks referenced by the sheet but not found on disk.
    pub missing: Vec<PathBuf>,
}

/// An `.m3u` playlist (multi-disc set).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub path: PathBuf,
    pub entries: Vec<PathBuf>,
}

#[derive(Debug, Default)]
pub struct ScanReport {
    pub roms: Vec<ScannedRom>,
    pub discs: Vec<ScannedDisc>,
    pub playlists: Vec<Playlist>,
    pub failures: Vec<ScanFailure>,
}

/// Recursively scans `root` (a directory or single file) in parallel.
/// Results are sorted by path and member for deterministic output.
pub fn scan(root: &Path) -> ScanReport {
    scan_with_progress(root, &|_, _| {})
}

/// Like [`scan`], but calls `progress(done, total)` after each file or disc
/// is hashed. May be called concurrently from worker threads.
pub fn scan_with_progress(root: &Path, progress: &(dyn Fn(usize, usize) + Sync)) -> ScanReport {
    scan_cached(root, &HashCache::default(), progress)
}

/// Like [`scan_with_progress`], but reuses `cache` for files whose size and mtime are unchanged.
pub fn scan_cached(
    root: &Path,
    cache: &HashCache,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> ScanReport {
    let mut report = ScanReport::default();
    let mut files = Vec::new();
    for e in WalkDir::new(root).follow_links(true) {
        match e {
            Ok(e) if e.file_type().is_file() => files.push(e.into_path()),
            Ok(_) => {}
            Err(err) => report.failures.push(ScanFailure {
                path: err.path().unwrap_or(root).to_path_buf(),
                error: err.into(),
            }),
        }
    }
    // Disc sheets claim their track files so they are not reported as loose ROMs.
    let mut sheets = Vec::new();
    let mut claimed = HashSet::new();
    for p in &files {
        let ext = ext_of(p);
        if ext == "lpl" {
            // RetroArch playlists (written by RomBro itself) are not ROMs.
            claimed.insert(p.clone());
        } else if ext == "m3u" {
            match disc::read_text(p) {
                Ok(t) => report.playlists.push(Playlist {
                    path: p.clone(),
                    entries: disc::sheet::parse_m3u(&t, p.parent().unwrap_or(root)),
                }),
                Err(e) => report.failures.push(ScanFailure {
                    path: p.clone(),
                    error: e.into(),
                }),
            }
            claimed.insert(p.clone());
        } else if let Some(kind) = DiscKind::from_ext(&ext) {
            match disc::tracks(p, kind) {
                Ok((found, missing)) => {
                    claimed.insert(p.clone());
                    claimed.extend(found.iter().cloned());
                    sheets.push((p.clone(), kind, found, missing));
                }
                Err(e) => report.failures.push(ScanFailure {
                    path: p.clone(),
                    error: e.into(),
                }),
            }
        }
    }
    files.retain(|p| !claimed.contains(p));
    let total = files.len() + sheets.len();
    let done = AtomicUsize::new(0);
    let tick = || progress(done.fetch_add(1, Ordering::Relaxed) + 1, total);
    progress(0, total);
    let discs: Vec<_> = sheets
        .into_par_iter()
        .map(|(path, kind, found, missing)| {
            let r = scan_disc_cached(&path, kind, &found, missing, cache)
                .map_err(|error| ScanFailure { path, error });
            tick();
            r
        })
        .collect();
    for d in discs {
        match d {
            Ok(d) => report.discs.push(d),
            Err(f) => report.failures.push(f),
        }
    }

    let results: Vec<_> = files
        .par_iter()
        .map(|p| {
            let r = cache
                .get(p)
                .map_or_else(|| scan_file(p), Ok)
                .map_err(|error| ScanFailure {
                    path: p.clone(),
                    error,
                });
            tick();
            r
        })
        .collect();
    for r in results {
        match r {
            Ok(roms) => report.roms.extend(roms),
            Err(f) => report.failures.push(f),
        }
    }
    report
        .roms
        .sort_by(|a, b| (&a.path, &a.member).cmp(&(&b.path, &b.member)));
    report.discs.sort_by(|a, b| a.path.cmp(&b.path));
    report.playlists.sort_by(|a, b| a.path.cmp(&b.path));
    report.failures.sort_by(|a, b| a.path.cmp(&b.path));
    report
}

/// Hashes every track of a disc and reads its serial.
pub fn scan_disc(
    path: &Path,
    kind: DiscKind,
    tracks: &[PathBuf],
    missing: Vec<PathBuf>,
) -> Result<ScannedDisc, ScanError> {
    scan_disc_cached(path, kind, tracks, missing, &HashCache::default())
}

fn scan_disc_cached(
    path: &Path,
    kind: DiscKind,
    tracks: &[PathBuf],
    missing: Vec<PathBuf>,
    cache: &HashCache,
) -> Result<ScannedDisc, ScanError> {
    let mut hashed = Vec::with_capacity(tracks.len());
    for t in tracks {
        if let Some(mut hit) = cache.get(t).filter(|h| h.len() == 1) {
            hashed.append(&mut hit);
            continue;
        }
        let (hashes, _) = if disc::is_chd(t) {
            let data = disc::chd::ChdTrack::open(t)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "CHD without CD data track")
            })?;
            hash_reader(BufReader::new(data), None, false)?
        } else {
            hash_reader(BufReader::new(File::open(t)?), None, false)?
        };
        hashed.push(ScannedRom {
            path: t.clone(),
            member: None,
            hashes,
            header: None,
            headerless: None,
        });
    }
    Ok(ScannedDisc {
        path: path.to_path_buf(),
        kind,
        id: disc::identify(tracks)?,
        tracks: hashed,
        missing,
    })
}

/// Scans one file; archives yield one entry per member.
pub fn scan_file(path: &Path) -> Result<Vec<ScannedRom>, ScanError> {
    match ext_of(path).as_str() {
        "zip" => scan_zip(path),
        "7z" => scan_7z(path),
        ext => {
            let f = File::open(path)?;
            let size = f.metadata()?.len();
            let (hashes, header, headerless) = hash_rom(BufReader::new(f), size, ext)?;
            Ok(vec![ScannedRom {
                path: path.to_path_buf(),
                member: None,
                hashes,
                header,
                headerless,
            }])
        }
    }
}

fn scan_zip(path: &Path) -> Result<Vec<ScannedRom>, ScanError> {
    let mut zip = zip::ZipArchive::new(BufReader::new(File::open(path)?))?;
    let mut out = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let f = zip.by_index(i)?;
        if f.is_dir() {
            continue;
        }
        let name = f.name().to_owned();
        let size = f.size();
        let (hashes, header, headerless) = hash_rom(f, size, &ext_of(Path::new(&name)))?;
        out.push(ScannedRom {
            path: path.to_path_buf(),
            member: Some(name),
            hashes,
            header,
            headerless,
        });
    }
    Ok(out)
}

fn scan_7z(path: &Path) -> Result<Vec<ScannedRom>, ScanError> {
    let mut ar = sevenz_rust2::ArchiveReader::open(path, sevenz_rust2::Password::empty())?;
    let mut out = Vec::new();
    ar.for_each_entries(|e, r| {
        if e.is_directory() || !e.has_stream() {
            return Ok(true);
        }
        let name = e.name().to_owned();
        let (hashes, header, headerless) = hash_rom(r, e.size(), &ext_of(Path::new(&name)))?;
        out.push(ScannedRom {
            path: path.to_path_buf(),
            member: Some(name),
            hashes,
            header,
            headerless,
        });
        Ok(true)
    })?;
    Ok(out)
}

type RomHashes = (Hashes, Option<Header>, Option<Hashes>);

/// Probes for a header, then hashes the full stream (and the headerless variant) in one pass.
fn hash_rom<R: Read>(mut r: R, size: u64, ext: &str) -> io::Result<RomHashes> {
    let mut probe = [0u8; header::PROBE_LEN];
    let mut n = 0;
    while n < probe.len() {
        match r.read(&mut probe[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    let header = header::detect(&probe[..n], size, ext);
    let (full, headerless) = hash_reader((&probe[..n]).chain(r), header.map(Header::size), false)?;
    Ok((full, header, headerless))
}

fn ext_of(p: &Path) -> String {
    p.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}
