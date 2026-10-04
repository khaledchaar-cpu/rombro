//! Parallel directory scan: hashes plain files and archive members (ZIP, 7z).

use crate::hash::{Hashes, hash_reader};
use crate::header::{self, Header};
use rayon::prelude::*;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
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

#[derive(Debug, Default)]
pub struct ScanReport {
    pub roms: Vec<ScannedRom>,
    pub failures: Vec<ScanFailure>,
}

/// Recursively scans `root` (a directory or single file) in parallel.
/// Results are sorted by path and member for deterministic output.
pub fn scan(root: &Path) -> ScanReport {
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
    let results: Vec<_> = files
        .par_iter()
        .map(|p| {
            scan_file(p).map_err(|error| ScanFailure {
                path: p.clone(),
                error,
            })
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
    report.failures.sort_by(|a, b| a.path.cmp(&b.path));
    report
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
    let (full, headerless) = hash_reader((&probe[..n]).chain(r), header.map(Header::size))?;
    Ok((full, header, headerless))
}

fn ext_of(p: &Path) -> String {
    p.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}
