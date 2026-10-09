//! CHD (MAME Compressed Hunks of Data) CD images: the first data track as a byte stream.
//!
//! RetroArch's disc databases hash the data track of a cue/bin dump, so streaming that track's
//! sector data (without subcode) yields hashes and serials comparable to loose `.bin` files.

use chd::Chd;
use chd::metadata::{KnownMetadata, Metadata, MetadataTag};
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use super::cdsector;

/// Pregap frames Redump writes as data sectors before a data track (the rest is silence).
const DATA_PREGAP: u64 = 150;

/// Bytes per stored CD frame: 2352 sector bytes + 96 subcode bytes.
const FRAME: u64 = 2448;

/// A track as described by CHD metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackInfo {
    number: u32,
    kind: String,
    /// First frame of the track's data within the image.
    start: u64,
    frames: u64,
    /// Pregap frames stored in the image right before `start` (skipped for reading).
    stored_pregap: u64,
    /// Pregap frames not stored in the image (Redump dumps carry them in the track file).
    missing_pregap: u64,
}

impl TrackInfo {
    /// Valid bytes per frame for this track type.
    fn sector_len(&self) -> u64 {
        match self.kind.as_str() {
            "MODE1" | "MODE2_FORM1" => 2048,
            "MODE2_FORM2" => 2324,
            "MODE2" | "MODE2_FORM_MIX" => 2336,
            _ => 2352,
        }
    }
}

/// Parses `TRACK:1 TYPE:MODE2_RAW FRAMES:1234 ...` entries and lays the tracks out.
fn layout(entries: &[String]) -> Vec<TrackInfo> {
    let mut parsed: Vec<(u32, String, u64, Option<u64>, u64, bool)> = entries
        .iter()
        .filter_map(|text| {
            let field = |k: &str| {
                text.split_whitespace()
                    .find_map(|kv| kv.strip_prefix(k)?.strip_prefix(':'))
                    .map(|v| v.trim_end_matches('\0'))
            };
            let num = |k: &str| field(k).and_then(|v| v.parse::<u64>().ok());
            Some((
                num("TRACK")? as u32,
                field("TYPE")?.to_owned(),
                num("FRAMES")?,
                num("PAD"),
                num("PREGAP").unwrap_or(0),
                field("PGTYPE").is_some_and(|t| t.starts_with('V')),
            ))
        })
        .collect();
    parsed.sort_by_key(|t| t.0);
    let mut start = 0;
    let mut out = Vec::with_capacity(parsed.len());
    for (number, kind, frames, pad, pregap, pregap_stored) in parsed {
        let pad = pad.unwrap_or(frames.div_ceil(4) * 4 - frames);
        let skip = if pregap_stored { pregap.min(frames) } else { 0 };
        out.push(TrackInfo {
            number,
            kind,
            start: start + skip,
            frames: frames - skip,
            stored_pregap: skip,
            missing_pregap: if pregap_stored { 0 } else { pregap },
        });
        start += frames + pad;
    }
    out
}

/// Tracks of a CHD image from its CD / GD-ROM metadata.
fn tracks_of(chd: &mut Chd<BufReader<File>>) -> io::Result<Vec<TrackInfo>> {
    let metas: Vec<Metadata> = chd.metadata_refs().try_into()?;
    let entries: Vec<String> = metas
        .iter()
        .filter(|m| {
            KnownMetadata::is_cdrom(m.metatag()) || m.metatag() == KnownMetadata::GdRomTrack as u32
        })
        .map(|m| String::from_utf8_lossy(&m.value).into_owned())
        .collect();
    Ok(layout(&entries))
}

/// Sector data of one CHD track, readable and seekable like a `.bin` file.
pub struct ChdTrack {
    chd: Chd<BufReader<File>>,
    track: TrackInfo,
    hunk_bytes: u64,
    hunk: Vec<u8>,
    cmp: Vec<u8>,
    loaded: Option<u32>,
    pos: u64,
    /// Bytes before the track data: its pregap as in a Redump `.bin` (see [`ChdTrack::open_redump`]).
    lead: Lead,
}

/// The pregap in front of a track read as a Redump track file.
#[derive(Default)]
struct Lead {
    /// Synthesized frames: `silent` zero sectors, then empty MODE1 sectors from `lba`.
    synth: u64,
    silent: u64,
    lba: u32,
}

impl ChdTrack {
    /// Opens the first data (non-audio) track; `None` if the CHD holds no CD data track.
    pub fn open(path: &Path) -> io::Result<Option<Self>> {
        Self::open_with(path, |ts| ts.iter().position(|t| t.kind != "AUDIO"))
    }

    /// Track numbers of the image with whether each is an audio track, in order.
    pub fn track_list(path: &Path) -> io::Result<Vec<(u32, bool)>> {
        Ok(
            tracks_of(&mut Chd::open(BufReader::new(File::open(path)?), None)?)?
                .into_iter()
                .map(|t| (t.number, t.kind == "AUDIO"))
                .collect(),
        )
    }

    /// Opens track `number` (1-based); `None` if the image has no such track.
    pub fn open_number(path: &Path, number: u32) -> io::Result<Option<Self>> {
        Self::open_with(path, |ts| ts.iter().position(|t| t.number == number))
    }

    fn open_with(
        path: &Path,
        pick: impl FnOnce(&[TrackInfo]) -> Option<usize>,
    ) -> io::Result<Option<Self>> {
        let mut chd = Chd::open(BufReader::new(File::open(path)?), None)?;
        let mut tracks = tracks_of(&mut chd)?;
        let Some(i) = pick(&tracks) else {
            return Ok(None);
        };
        let track = tracks.swap_remove(i);
        let hunk_bytes = u64::from(chd.header().hunk_size());
        Ok(Some(Self {
            hunk: chd.get_hunksized_buffer(),
            chd,
            track,
            hunk_bytes,
            cmp: Vec::new(),
            loaded: None,
            pos: 0,
            lead: Lead::default(),
        }))
    }

    /// Like [`ChdTrack::open`], but the track reads like the `.bin` of a Redump cue dump, whose
    /// hashes RetroArch's databases list: the pregap is included, and one CHD did not store is
    /// rebuilt as Redump writes it (audio silence, then 150 empty MODE1 sectors). Not for
    /// filesystem access: sector 0 is no longer the track's first data sector.
    pub fn open_redump(path: &Path) -> io::Result<Option<Self>> {
        let Some(mut t) = Self::open(path)? else {
            return Ok(None);
        };
        let stored = t.track.stored_pregap;
        let missing = t.track.missing_pregap;
        if missing > 0 && t.track.kind == "MODE1_RAW" && !t.is_empty() {
            let mut head = [0u8; 16];
            t.read_exact(&mut head)?;
            t.pos = 0;
            if let Some(lba) = cdsector::header_lba(&head) {
                t.lead.synth = missing;
                t.lead.silent = missing.saturating_sub(DATA_PREGAP);
                t.lead.lba = lba.saturating_sub(missing as u32);
            }
        }
        t.track.start -= stored;
        t.track.frames += stored;
        Ok(Some(t))
    }

    /// Track number within the image (1-based).
    pub fn number(&self) -> u32 {
        self.track.number
    }

    pub fn len(&self) -> u64 {
        (self.lead.synth + self.track.frames) * self.track.sector_len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Read for ChdTrack {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let sector = self.track.sector_len();
        if self.pos >= self.len() || buf.is_empty() {
            return Ok(0);
        }
        let synth = self.lead.synth * sector;
        if self.pos < synth {
            let (frame, off) = (self.pos / sector, (self.pos % sector) as usize);
            let raw = if frame < self.lead.silent {
                [0u8; cdsector::SECTOR]
            } else {
                cdsector::empty_mode1(self.lead.lba + frame as u32)
            };
            let n = buf.len().min(cdsector::SECTOR - off);
            buf[..n].copy_from_slice(&raw[off..off + n]);
            self.pos += n as u64;
            return Ok(n);
        }
        let data_pos = self.pos - synth;
        let (frame, off) = (data_pos / sector, data_pos % sector);
        let addr = (self.track.start + frame) * FRAME + off;
        let hunk = u32::try_from(addr / self.hunk_bytes).map_err(io::Error::other)?;
        if self.loaded != Some(hunk) {
            self.chd
                .hunk(hunk)?
                .read_hunk_in(&mut self.cmp, &mut self.hunk)?;
            self.loaded = Some(hunk);
        }
        let at = (addr % self.hunk_bytes) as usize;
        let n = buf
            .len()
            .min((sector - off) as usize)
            .min(self.hunk.len() - at);
        buf[..n].copy_from_slice(&self.hunk[at..at + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for ChdTrack {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let pos = match to {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::End(d) => self.len().checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        self.pos = pos.ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        Ok(self.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lays_out_tracks_with_padding_and_stored_pregap() {
        let t = layout(&[
            "TRACK:2 TYPE:AUDIO SUBTYPE:NONE FRAMES:250 PREGAP:150 PGTYPE:VAUDIO".into(),
            "TRACK:1 TYPE:MODE2_RAW SUBTYPE:NONE FRAMES:1001 PREGAP:0 PGTYPE:MODE1\0".into(),
        ]);
        assert_eq!(t[0].kind, "MODE2_RAW");
        assert_eq!((t[0].start, t[0].frames), (0, 1001));
        // 1001 frames are padded to 1004; the audio pregap (150 frames) is stored in the image.
        assert_eq!((t[1].start, t[1].frames), (1154, 100));
        let gd = layout(&["TRACK:1 TYPE:MODE1 SUBTYPE:NONE FRAMES:300 PAD:5 PREGAP:0".into()]);
        assert_eq!(gd[0].sector_len(), 2048);
    }
}
