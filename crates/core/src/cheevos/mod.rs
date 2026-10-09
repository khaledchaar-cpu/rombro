//! RetroAchievements: game hashes as rcheevos computes them (`src/hash/` in rcheevos), so a
//! library file can be matched against RA's hash list per console (SPEC §6 M16).

use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

use md5::{Digest, Md5};

mod cd;
mod disc;

const TABLE: &str = include_str!("consoles.tsv");

/// rcheevos hashes at most the first 64 MiB of a file.
const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// How rcheevos hashes a system's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// MD5 of the whole file.
    Whole,
    /// Without a 16-byte `NES\x1a` / `FDS\x1a` header.
    Nes,
    /// Without a 64-byte `LYNX` header.
    Lynx,
    /// Without a 128-byte header (`ATARI7800` at offset 1).
    A7800,
    /// Without a 512-byte copier header (size mod 8 KiB = 512).
    Snes,
    /// Without a 512-byte header (size & 512).
    Pce,
    /// Without a 32-byte `EmuSCV` header.
    Scv,
    /// Byte order normalised to z64 (big endian).
    N64,
    /// MD5 of the file name without extension (romset name).
    Arcade,
    /// Boot executable from `SYSTEM.CNF` (`BOOT`), or `PSX.EXE`.
    Psx,
    /// Boot executable from `SYSTEM.CNF` (`BOOT2`).
    Ps2,
    /// `PARAM.SFO` + `EBOOT.BIN`; `.pbp` whole.
    Psp,
    /// First 512 bytes of the disc (Sega CD, Saturn).
    SegaCd,
    /// Boot header and program sectors of the first data track.
    PceCd,
    /// IP.BIN meta + boot file.
    Dreamcast,
    /// Opera volume header + `LaunchMe`.
    ThreeDo,
    /// Header, ARM9/ARM7 code and icon block.
    Nds,
}

impl Method {
    /// Content is a disc image (sheets, CHD, ISO …) rather than a single ROM file.
    pub fn is_disc(self) -> bool {
        matches!(
            self,
            Self::Psx
                | Self::Ps2
                | Self::Psp
                | Self::SegaCd
                | Self::PceCd
                | Self::Dreamcast
                | Self::ThreeDo
        )
    }
}

/// A system RetroAchievements supports, with its console id and hash method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Console {
    pub id: u32,
    pub method: Method,
}

/// The RA console for an RDB system name (library top folder), if its hashing is supported.
pub fn console(system: &str) -> Option<Console> {
    TABLE.lines().filter(|l| !l.starts_with('#')).find_map(|l| {
        let mut f = l.split('\t');
        (f.next()? == system).then_some(())?;
        let id = f.next()?.parse().ok()?;
        let method = match f.next()? {
            "whole" => Method::Whole,
            "nes" => Method::Nes,
            "lynx" => Method::Lynx,
            "a7800" => Method::A7800,
            "snes" => Method::Snes,
            "pce" => Method::Pce,
            "scv" => Method::Scv,
            "n64" => Method::N64,
            "arcade" => Method::Arcade,
            "psx" => Method::Psx,
            "ps2" => Method::Ps2,
            "psp" => Method::Psp,
            "segacd" => Method::SegaCd,
            "pcecd" => Method::PceCd,
            "dreamcast" => Method::Dreamcast,
            "3do" => Method::ThreeDo,
            "nds" => Method::Nds,
            _ => return None,
        };
        Some(Console { id, method })
    })
}

/// All console ids in the table (deduplicated, ascending).
pub fn console_ids() -> Vec<u32> {
    let mut ids: Vec<u32> = TABLE
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split('\t').nth(1)?.parse().ok())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Files RetroAchievements can hash in `library`: every file directly in a system folder
/// whose system is supported (dot files left out), with its console. Disc systems: each disc
/// (sheet, CHD, ISO …, also in multi-disc game folders), not the track files of a sheet.
pub fn library_files(library: &Path) -> io::Result<Vec<(PathBuf, Console)>> {
    let mut out = Vec::new();
    for dir in std::fs::read_dir(library)?.filter_map(|e| e.ok()) {
        let Some(console) = console(&dir.file_name().to_string_lossy()) else {
            continue;
        };
        if console.method.is_disc() {
            disc_files(&dir.path(), console, 1, &mut out)?;
            continue;
        }
        for f in std::fs::read_dir(dir.path())?.filter_map(|e| e.ok()) {
            if f.file_type().is_ok_and(|t| t.is_file())
                && !f.file_name().to_string_lossy().starts_with('.')
            {
                out.push((f.path(), console));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Discs in `dir` (and `depth` levels of game folders below it).
fn disc_files(
    dir: &Path,
    console: Console,
    depth: u32,
    out: &mut Vec<(PathBuf, Console)>,
) -> io::Result<()> {
    const DISC: [&str; 8] = ["cue", "gdi", "chd", "iso", "cso", "pbp", "bin", "img"];
    let (mut files, mut tracks) = (Vec::new(), std::collections::HashSet::new());
    for e in std::fs::read_dir(dir)?.filter_map(|e| e.ok()) {
        let (path, name) = (e.path(), e.file_name().to_string_lossy().into_owned());
        if name.starts_with('.') || name.starts_with('_') {
            continue;
        }
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            if depth > 0 {
                disc_files(&path, console, depth - 1, out)?;
            }
            continue;
        }
        let ext = name
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .unwrap_or_default();
        if let Some(kind @ (crate::disc::DiscKind::Cue | crate::disc::DiscKind::Gdi)) =
            crate::disc::DiscKind::from_ext(&ext)
        {
            tracks.extend(
                crate::disc::tracks(&path, kind)
                    .map(|t| t.0)
                    .unwrap_or_default(),
            );
        }
        if DISC.contains(&ext.as_str()) {
            files.push(path);
        }
    }
    out.extend(
        files
            .into_iter()
            .filter(|f| !tracks.contains(f))
            .map(|f| (f, console)),
    );
    Ok(())
}

/// Title key for matching a library name against RA titles across versions: text before the
/// first ` (` / ` [`, lowercased, letters and digits only (`Zelda, The: X` = `Zelda, The - X`).
pub fn title_key(name: &str) -> String {
    let end = [" (", " ["]
        .iter()
        .filter_map(|p| name.find(p))
        .min()
        .unwrap_or(name.len());
    name[..end]
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// RA hash (lowercase hex MD5) of a library file. Zip/7z archives are hashed by their first
/// file's content, except arcade sets (by name). `None` if the content is not hashable
/// (e.g. an N64 file with an unknown byte order).
pub fn hash_file(path: &Path, method: Method) -> io::Result<Option<String>> {
    if method.is_disc() || method == Method::Nds {
        return disc::hash(path, method);
    }
    if method == Method::Arcade {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        return Ok(Some(hex(&Md5::digest(stem.as_bytes()))));
    }
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let data = match ext.as_str() {
        "zip" | "7z" => first_file(path)?,
        _ => {
            let mut buf = Vec::new();
            BufReader::new(File::open(path)?)
                .take(MAX_BYTES)
                .read_to_end(&mut buf)?;
            buf
        }
    };
    Ok(hash_bytes(&data, method))
}

/// RA hash of content already in memory (see [`hash_file`]; `Arcade` hashes `data` as the name).
pub fn hash_bytes(data: &[u8], method: Method) -> Option<String> {
    let data = &data[..data.len().min(MAX_BYTES as usize)];
    let n = data.len();
    let body: &[u8] = match method {
        Method::Whole | Method::Arcade => data,
        Method::Nes if n > 16 && (data.starts_with(b"NES\x1a") || data.starts_with(b"FDS\x1a")) => {
            &data[16..]
        }
        Method::Lynx if n > 64 && data.starts_with(b"LYNX\0") => &data[64..],
        Method::A7800 if n > 128 && &data[1..10] == b"ATARI7800" => &data[128..],
        Method::Snes if n % 0x2000 == 512 => &data[512..],
        Method::Pce if n & 512 != 0 => &data[512..],
        Method::Scv if n > 32 && data.starts_with(b"EmuSCV") => &data[32..],
        Method::N64 => return n64(data),
        _ => data,
    };
    Some(hex(&Md5::digest(body)))
}

fn n64(data: &[u8]) -> Option<String> {
    let mut buf = data.to_vec();
    match data.first()? {
        0x80 | 0xE8 | 0x22 => {}
        // v64: 16-bit byte swapped
        0x37 => buf.chunks_exact_mut(2).for_each(|c| c.swap(0, 1)),
        // n64: little endian 32-bit words
        0x40 => buf.chunks_exact_mut(4).for_each(|c| c.reverse()),
        _ => return None,
    }
    Some(hex(&Md5::digest(&buf)))
}

/// Content of an archive's first file (by entry order), at most 64 MiB.
fn first_file(archive: &Path) -> io::Result<Vec<u8>> {
    let is_zip = archive
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"));
    let first = if is_zip {
        crate::archive::file_paths(archive)?.into_iter().next()
    } else {
        let ar = sevenz_rust2::ArchiveReader::open(archive, sevenz_rust2::Password::empty())
            .map_err(io::Error::other)?;
        ar.archive()
            .files
            .iter()
            .find(|f| !f.is_directory())
            .map(|f| f.name().to_owned())
    };
    let first = first.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "empty archive"))?;
    let mut data = crate::archive::read_member(archive, &first)?;
    data.truncate(MAX_BYTES as usize);
    Ok(data)
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::with_capacity(32), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests;
