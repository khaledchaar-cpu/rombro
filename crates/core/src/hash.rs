//! Streaming CRC32 + SHA1 (+ optional MD5) in a single pass.

use md5::Md5;
use sha1::{Digest, Sha1};
use std::io::{self, Read};

/// Digests of one byte stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Hashes {
    pub size: u64,
    pub crc: u32,
    pub sha1: [u8; 20],
    /// Only computed on request (costs ~4x the time of CRC32+SHA1).
    pub md5: Option<[u8; 16]>,
}

/// Incremental hasher for all digests.
#[derive(Default, Clone)]
pub struct MultiHasher {
    size: u64,
    crc: crc32fast::Hasher,
    sha1: Sha1,
    md5: Option<Md5>,
}

impl MultiHasher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_md5() -> Self {
        Self {
            md5: Some(Md5::new()),
            ..Self::default()
        }
    }

    pub fn update(&mut self, buf: &[u8]) {
        self.size += buf.len() as u64;
        self.crc.update(buf);
        self.sha1.update(buf);
        if let Some(m) = self.md5.as_mut() {
            m.update(buf);
        }
    }

    pub fn finish(self) -> Hashes {
        Hashes {
            size: self.size,
            crc: self.crc.finalize(),
            sha1: self.sha1.finalize().into(),
            md5: self.md5.map(|m| m.finalize().into()),
        }
    }
}

pub(crate) const BUF_SIZE: usize = 256 * 1024;

/// Hashes a stream. If `skip` is given, additionally hashes the stream without its first `skip` bytes
/// (header-stripped variant); `None` is returned for it if the stream is not longer than `skip`.
pub fn hash_reader<R: Read>(
    mut r: R,
    skip: Option<u64>,
    md5: bool,
) -> io::Result<(Hashes, Option<Hashes>)> {
    let new = || {
        if md5 {
            MultiHasher::with_md5()
        } else {
            MultiHasher::new()
        }
    };
    let mut full = new();
    let mut stripped = skip.map(|s| (s, new()));
    let mut buf = vec![0u8; BUF_SIZE];
    let mut pos = 0u64;
    loop {
        let n = match r.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        let chunk = &buf[..n];
        full.update(chunk);
        if let Some((s, h)) = stripped.as_mut() {
            let end = pos + n as u64;
            if end > *s {
                let from = s.saturating_sub(pos) as usize;
                h.update(&chunk[from..]);
            }
        }
        pos += n as u64;
    }
    let stripped = stripped.and_then(|(s, h)| (pos > s).then(|| h.finish()));
    Ok((full.finish(), stripped))
}

/// Hashes a byte-swapped N64 dump as is and normalized to big-endian (`.z64`, the order
/// the databases use). `word` is 2 for `.v64` (pairs swapped) and 4 for `.n64` (little-endian).
pub fn hash_n64<R: Read>(mut r: R, word: usize) -> io::Result<(Hashes, Hashes)> {
    let (mut raw, mut norm) = (MultiHasher::new(), MultiHasher::new());
    let mut buf = vec![0u8; BUF_SIZE];
    loop {
        // fill the buffer completely so words never straddle two reads
        let mut n = 0;
        while n < buf.len() {
            match r.read(&mut buf[n..]) {
                Ok(0) => break,
                Ok(k) => n += k,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if n == 0 {
            break;
        }
        raw.update(&buf[..n]);
        let whole = n - n % word;
        buf[..whole]
            .chunks_exact_mut(word)
            .for_each(<[u8]>::reverse);
        norm.update(&buf[..n]);
        if n < buf.len() {
            break;
        }
    }
    Ok((raw.finish(), norm.finish()))
}

/// Byte order of an N64 dump from its first word: `Some(word)` if it needs swapping.
pub fn n64_swap(probe: &[u8]) -> Option<usize> {
    match probe.get(..4)? {
        [0x37, 0x80, 0x40, 0x12] => Some(2),
        [0x40, 0x12, 0x37, 0x80] => Some(4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn n64_byte_orders_normalize_to_z64() {
        let z64: Vec<u8> = [0x80, 0x37, 0x12, 0x40]
            .iter()
            .copied()
            .cycle()
            .take(4096)
            .collect();
        let v64: Vec<u8> = z64.chunks(2).flat_map(|c| [c[1], c[0]]).collect();
        let n64: Vec<u8> = z64
            .chunks(4)
            .flat_map(|c| [c[3], c[2], c[1], c[0]])
            .collect();
        let want = hash_reader(&z64[..], None, false).unwrap().0;
        for (img, word) in [(&v64, 2), (&n64, 4)] {
            assert_eq!(n64_swap(img), Some(word));
            let (raw, norm) = hash_n64(&img[..], word).unwrap();
            assert_eq!(norm, want);
            assert_ne!(raw, want);
        }
        assert_eq!(n64_swap(&z64), None);
    }

    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn known_vectors() {
        let (h, s) = hash_reader(&b"abc"[..], None, true).unwrap();
        assert_eq!(h.size, 3);
        assert_eq!(h.crc, 0x3524_41c2);
        assert_eq!(hex(&h.sha1), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(hex(&h.md5.unwrap()), "900150983cd24fb0d6963f7d28e17f72");
        assert!(s.is_none());
    }

    #[test]
    fn stripped_matches_tail_across_chunks() {
        let data: Vec<u8> = (0..BUF_SIZE * 2 + 77).map(|i| (i % 251) as u8).collect();
        for skip in [0u64, 16, 512, BUF_SIZE as u64 + 3] {
            let (_, s) = hash_reader(&data[..], Some(skip), false).unwrap();
            let (want, _) = hash_reader(&data[skip as usize..], None, false).unwrap();
            assert_eq!(s, Some(want), "skip {skip}");
        }
        let (_, s) = hash_reader(&data[..16], Some(16), false).unwrap();
        assert!(s.is_none());
    }
}
