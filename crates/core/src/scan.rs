//! Parallel directory scan: hashes plain files and archive members (ZIP, 7z).

use crate::cache::HashCache;
use crate::disc::{self, DiscId, DiscKind};
use crate::hash::{Hashes, hash_reader};
use crate::header::{self, Header};
use crate::meter::{Counter, Meter};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek};
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
    /// Hashes without the detected header; for a 2048-byte `.iso`, its raw-sector hashes.
    pub headerless: Option<Hashes>,
}

/// A file (or archive) that could not be scanned.
#[derive(Debug)]
pub struct ScanFailure {
    pub path: PathBuf,
    pub error: ScanError,
}

/// Scan progress: items (files or discs) and the bytes they hold. Bytes advance evenly
/// with the hashing work, so large discs do not stall the bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct ScanTick<'a> {
    pub done: usize,
    pub total: usize,
    pub bytes: u64,
    pub bytes_total: u64,
    /// The large file being hashed right now, if this tick comes from inside one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<&'a str>,
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
    /// Hashes of whole ZIP/7z files (`member: None`); arcade romsets are identified by these.
    pub archives: Vec<ScannedRom>,
    pub discs: Vec<ScannedDisc>,
    pub playlists: Vec<Playlist>,
    pub failures: Vec<ScanFailure>,
}

/// Recursively scans `root` (a directory or single file) in parallel.
/// Results are sorted by path and member for deterministic output.
pub fn scan(root: &Path) -> ScanReport {
    scan_with_progress(root, &|_| {})
}

/// Like [`scan`], but calls `progress` after each file or disc
/// is hashed. May be called concurrently from worker threads.
pub fn scan_with_progress(root: &Path, progress: &(dyn Fn(ScanTick<'_>) + Sync)) -> ScanReport {
    scan_cached(root, &HashCache::default(), progress)
}

/// OS clutter and empty placeholders (`.keep`) are never ROMs; they are left where they are.
/// Empty files would otherwise match RDB entries that carry the empty-file hash.
fn is_ignored(e: &walkdir::DirEntry, cache: &HashCache) -> bool {
    let name = e.file_name().to_string_lossy();
    let junk = matches!(
        name.to_ascii_lowercase().as_str(),
        ".ds_store" | "thumbs.db" | "desktop.ini" | ".directory"
    ) || name.starts_with("._");
    // empty ScummVM launchers are kept: they are repaired from their file name
    let launcher = crate::scummvm::repaired_id(e.path(), b"").is_some();
    junk || (cache.size(e.path()) == Some(0) && !launcher)
}

/// Like [`scan_with_progress`], but reuses `cache` for files whose size and mtime are unchanged.
pub fn scan_cached(
    root: &Path,
    cache: &HashCache,
    progress: &(dyn Fn(ScanTick<'_>) + Sync),
) -> ScanReport {
    let mut report = ScanReport::default();
    let mut files = Vec::new();
    for e in WalkDir::new(root).follow_links(true) {
        match e {
            Ok(e) if e.file_type().is_file() && !is_ignored(&e, cache) => files.push(e.into_path()),
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
    let size = |p: &PathBuf| cache.size(p).unwrap_or(0);
    let sheet_sizes: Vec<u64> = sheets
        .iter()
        .map(|(_, _, found, _)| found.iter().map(size).sum())
        .collect();
    let file_sizes: Vec<u64> = files.par_iter().map(size).collect();
    let total = files.len() + sheets.len();
    let bytes_total = sheet_sizes.iter().chain(&file_sizes).sum();
    let meter = Meter::new(total, bytes_total, progress);
    let discs: Vec<_> = sheets
        .into_par_iter()
        .zip(sheet_sizes)
        .map(|((path, kind, found, missing), n)| {
            // discs are the large files: count while hashing so the bar keeps moving
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            let counter = Counter::new(&meter, n, name.as_deref().unwrap_or_default());
            let r = scan_disc_cached(&path, kind, &found, missing, cache, &counter)
                .map_err(|error| ScanFailure { path, error });
            meter.finish(n, counter.read());
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
        .zip(&file_sizes)
        .map(|(p, &n)| {
            let r = cache
                .get(p)
                // Entries cached before archives were hashed as a whole lack that hash.
                .filter(|roms| !is_archive(p) || roms.iter().any(|r| r.member.is_none()))
                // tiny; cached entries predate launcher repair
                .filter(|_| !ext_of(p).eq("scummvm"))
                .map_or_else(|| scan_file(p), Ok)
                .map_err(|error| ScanFailure {
                    path: p.clone(),
                    error,
                });
            meter.finish(n, 0);
            r
        })
        .collect();
    for r in results {
        match r {
            Ok(roms) => {
                for r in roms {
                    if r.member.is_none() && is_archive(&r.path) {
                        report.archives.push(r);
                    } else {
                        report.roms.push(r);
                    }
                }
            }
            Err(f) => report.failures.push(f),
        }
    }
    report
        .roms
        .sort_by(|a, b| (&a.path, &a.member).cmp(&(&b.path, &b.member)));
    report.archives.sort_by(|a, b| a.path.cmp(&b.path));
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
    let counter = Counter::default();
    scan_disc_cached(path, kind, tracks, missing, &HashCache::default(), &counter)
}

fn scan_disc_cached(
    path: &Path,
    kind: DiscKind,
    tracks: &[PathBuf],
    missing: Vec<PathBuf>,
    cache: &HashCache,
    counter: &Counter,
) -> Result<ScannedDisc, ScanError> {
    let mut hashed = Vec::with_capacity(tracks.len());
    for t in tracks {
        // trusted entries were checked when cached; don't touch the file again
        let cd_iso = kind == DiscKind::Iso && !cache.trusts(t) && disc::is_cd_iso(t);
        if let Some(mut hit) = cache
            .get(t)
            .filter(|h| h.len() == 1 && (!cd_iso || h[0].headerless.is_some()))
            // entries cached before `.cso` support hold the compressed file's hashes
            .filter(|h| !disc::is_cso(t) || cache.size(t).is_some_and(|n| n != h[0].hashes.size))
        {
            hashed.append(&mut hit);
            continue;
        }
        let (hashes, raw) = if cd_iso {
            // 2048-byte sectors: also hash as raw sectors, as the databases list them
            crate::hash::hash_iso(BufReader::new(counter.wrap(File::open(t)?)))?
        } else if disc::is_cso(t) {
            let data = disc::cso::CsoReader::open(t)?;
            hash_reader(BufReader::new(counter.wrap(data)), None, false)?
        } else if kind == DiscKind::Nintendo {
            // compressed container: no database hash exists; the header identifies the file
            hash_reader(File::open(t)?.take(1 << 16), None, false)?
        } else if disc::is_chd(t) {
            let data = disc::chd::ChdTrack::open_redump(t)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "CHD without CD data track")
            })?;
            hash_reader(BufReader::new(counter.wrap(data)), None, false)?
        } else {
            hash_reader(BufReader::new(counter.wrap(File::open(t)?)), None, false)?
        };
        hashed.push(ScannedRom {
            path: t.clone(),
            member: None,
            hashes,
            header: None,
            headerless: raw,
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

/// Scans one file; archives yield one entry per member plus one for the whole file
/// (`member: None`).
pub fn scan_file(path: &Path) -> Result<Vec<ScannedRom>, ScanError> {
    let ext = ext_of(path);
    let members = match ext.as_str() {
        "zip" => scan_zip(path)?,
        "7z" => scan_7z(path)?,
        _ if let Some(layout) = crate::sufami::layout(path)? => {
            // combined image: members only, so the planner treats it like an archive
            return layout
                .into_iter()
                .map(|(name, range)| {
                    let mut f = File::open(path)?;
                    f.seek(io::SeekFrom::Start(range.start))?;
                    let size = range.end - range.start;
                    let ext = ext_of(Path::new(&name));
                    let (hashes, header, headerless) =
                        hash_rom(BufReader::new(f.take(size)), size, &ext)?;
                    Ok(ScannedRom {
                        path: path.to_path_buf(),
                        member: Some(name),
                        hashes,
                        header,
                        headerless,
                    })
                })
                .collect();
        }
        _ => {
            let f = File::open(path)?;
            let size = f.metadata()?.len();
            let (hashes, header, mut headerless) = hash_rom(BufReader::new(f), size, &ext)?;
            if let Some(id) = crate::scummvm::repaired_id_of(path) {
                // broken launcher: identify by the id it will be repaired to
                headerless = Some(hash_reader(id.as_bytes(), None, false)?.0);
            }
            return Ok(vec![ScannedRom {
                path: path.to_path_buf(),
                member: None,
                hashes,
                header,
                headerless,
            }]);
        }
    };
    let (hashes, _) = hash_reader(BufReader::new(File::open(path)?), None, false)?;
    let whole = ScannedRom {
        path: path.to_path_buf(),
        member: None,
        hashes,
        header: None,
        headerless: None,
    };
    Ok(std::iter::once(whole).chain(members).collect())
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
    if matches!(ext, "n64" | "v64" | "z64")
        && let Some(word) = crate::hash::n64_swap(&probe[..n])
    {
        // byte-swapped dump: the normalized hashes act like a stripped header for lookups
        let (raw, norm) = crate::hash::hash_n64((&probe[..n]).chain(r), word)?;
        return Ok((raw, None, Some(norm)));
    }
    let header = header::detect(&probe[..n], size, ext);
    let (full, headerless) = hash_reader((&probe[..n]).chain(r), header.map(Header::size), false)?;
    Ok((full, header, headerless))
}

fn is_archive(p: &Path) -> bool {
    matches!(ext_of(p).as_str(), "zip" | "7z")
}

fn ext_of(p: &Path) -> String {
    p.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}
