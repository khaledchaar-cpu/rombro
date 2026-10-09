//! CD access as rcheevos' cdreader sees it: tracks picked by number, sectors addressed by
//! absolute LBA, files found in the ISO9660 directory (`rc_cd_find_file_sector`).

use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use md5::Md5;
use md5::digest::Update;

use crate::disc::{self, chd::ChdTrack, cso::CsoReader, iso9660::Track};

use super::MAX_BYTES;

const USER: usize = 2048;

/// Which track of a disc to open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    Number(u32),
    FirstData,
    Last,
}

trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

/// One data track: user data per sector, its first sector at absolute LBA `start`.
pub struct CdTrack {
    data: Track<Box<dyn ReadSeek>>,
    start: u32,
}

impl CdTrack {
    /// Opens a track of the disc at `path` (`.cue`, `.gdi`, `.chd`, `.cso`, or a single
    /// `.iso`/`.bin` track). `None` if the disc has no such track.
    pub fn open(path: &Path, which: Which) -> io::Result<Option<Self>> {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let dir = path.parent().unwrap_or(Path::new("."));
        let (reader, start): (Box<dyn ReadSeek>, Option<u32>) = match ext.as_str() {
            "chd" => {
                let list = ChdTrack::track_list(path)?;
                let Some(n) = pick(&list, which) else {
                    return Ok(None);
                };
                match ChdTrack::open_number(path, n)? {
                    Some(t) => (Box::new(BufReader::new(t)), None),
                    None => return Ok(None),
                }
            }
            "cue" | "gdi" => {
                let text = disc::read_text(path)?;
                let tracks = if ext == "cue" {
                    cue_tracks(&text)
                } else {
                    gdi_tracks(&text)
                };
                let list: Vec<(u32, bool)> = tracks.iter().map(|t| (t.number, t.audio)).collect();
                let Some(t) =
                    pick(&list, which).and_then(|n| tracks.into_iter().find(|t| t.number == n))
                else {
                    return Ok(None);
                };
                let file = disc::resolve(&dir.join(&t.file))
                    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, t.file.clone()))?;
                let file = File::open(file)?;
                (
                    Box::new(Offset::new(BufReader::new(file), t.offset)?),
                    t.lba,
                )
            }
            "cso" => (Box::new(BufReader::new(CsoReader::open(path)?)), Some(0)),
            _ if matches!(which, Which::Number(n) if n != 1) => return Ok(None),
            _ => (Box::new(BufReader::new(File::open(path)?)), Some(0)),
        };
        let mut data = Track::open(reader)?;
        // raw sectors carry their own address; cooked ones start where the sheet says
        let start = match start {
            Some(lba) => lba,
            None => data.raw_lba(0)?.unwrap_or(0),
        };
        Ok(Some(Self { data, start }))
    }

    /// Absolute LBA of the track's first sector.
    pub fn first_sector(&self) -> u32 {
        self.start
    }

    /// User data of absolute sector `lba`; `None` outside the track.
    pub fn sector(&mut self, lba: u32) -> Option<[u8; USER]> {
        self.data.sector(lba.checked_sub(self.start)?).ok()
    }

    /// Start sector and size of `path` (`DIR\FILE`, case-insensitive), as rcheevos finds it.
    pub fn find_file(&mut self, path: &str) -> Option<(u32, u32)> {
        let path = path.strip_prefix('\\').unwrap_or(path);
        let (mut sector, mut num_sectors, name) = match path.rsplit_once('\\') {
            Some((dir, name)) => (self.find_file(dir)?.0, 0, name),
            None => {
                let pvd = self.sector(self.start + 16)?;
                let block = u32::from(u16::from_le_bytes([pvd[128], pvd[129]]));
                let n = le32(&pvd[166..]).checked_div(block).unwrap_or(1);
                (le24(&pvd[158..]), n, path)
            }
        };
        let name = name.as_bytes();
        let mut buf = self.sector(sector)?;
        let mut off = 0;
        loop {
            if off >= USER || buf[off] == 0 {
                if num_sectors > 1 {
                    num_sectors -= 1;
                    sector += 1;
                    if let Some(b) = self.sector(sector) {
                        buf = b;
                        off = 0;
                        continue;
                    }
                }
                return None;
            }
            let rec = &buf[off..];
            let n = name.len();
            let len_ok = usize::from(rec.get(32).copied()?) == n || rec.get(33 + n) == Some(&b';');
            if len_ok
                && rec
                    .get(33..33 + n)
                    .is_some_and(|r| r.eq_ignore_ascii_case(name))
            {
                return Some((le24(&rec[2..]), le32(&rec[10..])));
            }
            off += usize::from(buf[off]);
        }
    }

    /// Feeds `size` bytes of the file at `lba` into `md5` (`rc_hash_cd_file`, at most 64 MiB).
    /// `None` if its first sector cannot be read.
    pub fn hash_file(&mut self, md5: &mut Md5, lba: u32, size: u32) -> Option<()> {
        let mut left = (size as u64).min(MAX_BYTES) as usize;
        let mut buf = self.sector(lba)?;
        let mut lba = lba;
        loop {
            let n = left.min(USER);
            md5.update(&buf[..n]);
            left -= n;
            if left == 0 {
                return Some(());
            }
            lba += 1;
            match self.sector(lba) {
                Some(b) => buf = b,
                None => return Some(()),
            }
        }
    }
}

fn pick(list: &[(u32, bool)], which: Which) -> Option<u32> {
    match which {
        Which::Number(n) => list.iter().find(|t| t.0 == n),
        Which::FirstData => list.iter().find(|t| !t.1),
        Which::Last => list.iter().max_by_key(|t| t.0),
    }
    .map(|t| t.0)
}

/// A track of a sheet: its file, the byte offset of its data in that file, and (`.gdi`) LBA.
#[derive(Debug, PartialEq)]
struct SheetTrack {
    number: u32,
    audio: bool,
    file: String,
    offset: u64,
    lba: Option<u32>,
}

/// Tracks of a `.cue` sheet; data begins at `INDEX 01` (pregap in the file skipped).
fn cue_tracks(text: &str) -> Vec<SheetTrack> {
    let mut out: Vec<SheetTrack> = Vec::new();
    let mut file = String::new();
    let mut sector_size = 2352u64;
    for line in text.lines().map(str::trim) {
        let mut words = line.split_whitespace();
        match words.next().map(str::to_ascii_uppercase).as_deref() {
            Some("FILE") => {
                let rest = line[4..].trim();
                file = match rest.strip_prefix('"') {
                    Some(q) => q.split('"').next().unwrap_or_default().to_owned(),
                    None => rest
                        .rsplit_once(char::is_whitespace)
                        .map_or(rest, |r| r.0)
                        .to_owned(),
                };
            }
            Some("TRACK") => {
                let number = words.next().and_then(|n| n.parse().ok()).unwrap_or(0);
                let mode = words.next().unwrap_or_default().to_ascii_uppercase();
                sector_size = mode
                    .rsplit_once('/')
                    .and_then(|(_, s)| s.parse().ok())
                    .unwrap_or(2352);
                out.push(SheetTrack {
                    number,
                    audio: mode == "AUDIO",
                    file: file.clone(),
                    offset: 0,
                    lba: None,
                });
            }
            Some("INDEX") if words.next() == Some("01") => {
                let msf: Vec<u64> = words
                    .next()
                    .unwrap_or_default()
                    .split(':')
                    .filter_map(|v| v.parse().ok())
                    .collect();
                if let (Some(t), [m, s, f]) = (out.last_mut(), msf.as_slice()) {
                    t.offset = (m * 4500 + s * 75 + f) * sector_size;
                }
            }
            _ => {}
        }
    }
    out
}

/// Tracks of a `.gdi` sheet: `<no> <lba> <type> <sector size> <file> <offset>`.
fn gdi_tracks(text: &str) -> Vec<SheetTrack> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let number = it.next()?.parse().ok()?;
            let lba = it.next()?.parse().ok()?;
            let audio = it.next()? == "0";
            it.next()?;
            let rest = it.collect::<Vec<_>>().join(" ");
            let file = match rest.strip_prefix('"') {
                Some(q) => q.split('"').next()?.to_owned(),
                None => rest.split_whitespace().next()?.to_owned(),
            };
            Some(SheetTrack {
                number,
                audio,
                file,
                offset: 0,
                lba: Some(lba),
            })
        })
        .collect()
}

/// A reader that starts `base` bytes into its inner reader.
struct Offset<R> {
    inner: R,
    base: u64,
}

impl<R: Seek> Offset<R> {
    fn new(mut inner: R, base: u64) -> io::Result<Self> {
        inner.seek(SeekFrom::Start(base))?;
        Ok(Self { inner, base })
    }
}

impl<R: Read> Read for Offset<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<R: Seek> Seek for Offset<R> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let to = match to {
            SeekFrom::Start(p) => SeekFrom::Start(p + self.base),
            other => other,
        };
        Ok(self.inner.seek(to)?.saturating_sub(self.base))
    }
}

fn le24(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], 0])
}

fn le32(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cue_tracks_with_pregap_and_shared_file() {
        let t = cue_tracks(
            "FILE \"g.bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n  TRACK 02 AUDIO\n    INDEX 00 00:10:00\n    INDEX 01 00:12:00\nFILE \"g (Track 3).bin\" BINARY\n  TRACK 03 MODE1/2048\n    INDEX 00 00:00:00\n    INDEX 01 00:02:00\n",
        );
        assert_eq!(t.len(), 3);
        assert_eq!((t[0].number, t[0].audio, t[0].offset), (1, false, 0));
        assert_eq!((t[1].audio, t[1].offset), (true, 900 * 2352));
        assert_eq!(
            (t[2].file.as_str(), t[2].offset),
            ("g (Track 3).bin", 150 * 2048)
        );
        assert_eq!(
            pick(&[(1, false), (2, true), (3, false)], Which::Last),
            Some(3)
        );
        assert_eq!(pick(&[(1, true), (2, false)], Which::FirstData), Some(2));
    }

    #[test]
    fn gdi_tracks_with_quoted_names() {
        let t = gdi_tracks(
            "3\n1 0 4 2352 track01.bin 0\n2 600 0 2352 \"a b.raw\" 0\n3 45000 4 2352 track03.bin 0\n",
        );
        assert_eq!(t[1].file, "a b.raw");
        assert!(t[1].audio);
        assert_eq!(t[2].lba, Some(45000));
    }
}
