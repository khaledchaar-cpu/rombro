//! Compressed ISO (`.cso`, CISO v1): fixed-size blocks, each raw-deflate compressed or stored,
//! located by an index after a 24-byte header. Read as the plain ISO it stands for, so it
//! hashes and identifies like the `.iso` the databases list.

use flate2::read::DeflateDecoder;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

const MAGIC: &[u8; 4] = b"CISO";
const HEADER: u64 = 24;
/// Index entry flag: block stored uncompressed.
const PLAIN: u32 = 0x8000_0000;

pub struct CsoReader {
    file: BufReader<File>,
    size: u64,
    block: usize,
    align: u8,
    index: Vec<u32>,
    pos: u64,
    /// Decoded block currently held, by number.
    cached: Option<u64>,
    buf: Vec<u8>,
}

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("cso: {msg}"))
}

impl CsoReader {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = BufReader::new(File::open(path)?);
        let mut h = [0u8; HEADER as usize];
        file.read_exact(&mut h)?;
        if &h[..4] != MAGIC {
            return Err(bad("no CISO header"));
        }
        let le32 = |i: usize| u32::from_le_bytes([h[i], h[i + 1], h[i + 2], h[i + 3]]);
        let size = u64::from(le32(8)) | (u64::from(le32(12)) << 32);
        let block = le32(16) as usize;
        let align = h[21];
        if block == 0 || block > 1 << 20 || h[20] > 1 || align > 31 {
            return Err(bad("unsupported header"));
        }
        let entries = size.div_ceil(block as u64) as usize + 1;
        let mut raw = vec![0u8; entries * 4];
        file.read_exact(&mut raw)?;
        let index = raw
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        Ok(Self {
            file,
            size,
            block,
            align,
            index,
            pos: 0,
            cached: None,
            buf: Vec::with_capacity(block),
        })
    }

    /// Size of the uncompressed image.
    pub fn image_size(&self) -> u64 {
        self.size
    }

    fn load(&mut self, n: u64) -> io::Result<()> {
        if self.cached == Some(n) {
            return Ok(());
        }
        let i = n as usize;
        let (Some(&a), Some(&b)) = (self.index.get(i), self.index.get(i + 1)) else {
            return Err(bad("block out of range"));
        };
        let start = u64::from(a & !PLAIN) << self.align;
        let end = u64::from(b & !PLAIN) << self.align;
        let want = (self.size - n * self.block as u64).min(self.block as u64) as usize;
        self.file.seek(SeekFrom::Start(start))?;
        self.buf.clear();
        if a & PLAIN != 0 {
            self.buf.resize(want, 0);
            self.file.read_exact(&mut self.buf)?;
        } else {
            let packed = (&mut self.file).take(end.saturating_sub(start));
            DeflateDecoder::new(packed)
                .take(want as u64)
                .read_to_end(&mut self.buf)?;
            if self.buf.len() != want {
                return Err(bad("short block"));
            }
        }
        self.cached = Some(n);
        Ok(())
    }
}

impl Read for CsoReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.size || out.is_empty() {
            return Ok(0);
        }
        let n = self.pos / self.block as u64;
        self.load(n)?;
        let off = (self.pos - n * self.block as u64) as usize;
        let k = out.len().min(self.buf.len() - off);
        out[..k].copy_from_slice(&self.buf[off..off + k]);
        self.pos += k as u64;
        Ok(k)
    }
}

impl Seek for CsoReader {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let pos = match to {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::End(d) => self.size.checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        self.pos = pos.ok_or_else(|| bad("seek before start"))?;
        Ok(self.pos)
    }
}

/// Writes `data` as a CISO v1 image (test helper; block 0 stored, the rest compressed).
#[cfg(test)]
pub(crate) fn encode(data: &[u8], block: usize) -> Vec<u8> {
    use flate2::{Compression, write::DeflateEncoder};
    use std::io::Write;
    let blocks: Vec<&[u8]> = data.chunks(block).collect();
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(data.len() as u64).to_le_bytes());
    out.extend_from_slice(&(block as u32).to_le_bytes());
    out.extend_from_slice(&[1, 0, 0, 0]);
    let index_at = out.len();
    out.resize(index_at + (blocks.len() + 1) * 4, 0);
    let mut index = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        let at = out.len() as u32;
        if i == 0 {
            index.push(at | PLAIN);
            out.extend_from_slice(b);
        } else {
            index.push(at);
            let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
            e.write_all(b).unwrap();
            out.extend_from_slice(&e.finish().unwrap());
        }
    }
    index.push(out.len() as u32);
    for (i, v) in index.iter().enumerate() {
        out[index_at + i * 4..index_at + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_seeks_like_the_plain_image() {
        let data: Vec<u8> = (0..10_000u32).map(|i| (i * 7 % 251) as u8).collect();
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.cso");
        std::fs::write(&p, encode(&data, 2048)).unwrap();
        let mut r = CsoReader::open(&p).unwrap();
        assert_eq!(r.image_size(), data.len() as u64);
        let mut all = Vec::new();
        r.read_to_end(&mut all).unwrap();
        assert_eq!(all, data);
        r.seek(SeekFrom::Start(5000)).unwrap();
        let mut b = [0u8; 100];
        r.read_exact(&mut b).unwrap();
        assert_eq!(&b[..], &data[5000..5100]);
    }

    #[test]
    fn scans_with_the_hashes_of_its_iso() {
        use crate::disc::DiscKind;
        let data: Vec<u8> = (0..20_000u32).map(|i| (i % 253) as u8).collect();
        let dir = tempfile::tempdir().unwrap();
        let (iso, cso) = (dir.path().join("g.iso"), dir.path().join("g.cso"));
        std::fs::write(&iso, &data).unwrap();
        std::fs::write(&cso, encode(&data, 2048)).unwrap();
        let hash = |p: &Path, k| {
            crate::scan::scan_disc(p, k, &[p.to_path_buf()], Vec::new())
                .unwrap()
                .tracks[0]
                .hashes
        };
        assert_eq!(hash(&cso, DiscKind::Cso), hash(&iso, DiscKind::Iso));
    }
}
