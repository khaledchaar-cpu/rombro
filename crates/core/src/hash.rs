//! Streaming CRC32 + SHA1 + MD5 in a single pass.

use md5::Md5;
use sha1::{Digest, Sha1};
use std::io::{self, Read};

/// Digests of one byte stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hashes {
    pub size: u64,
    pub crc: u32,
    pub sha1: [u8; 20],
    pub md5: [u8; 16],
}

/// Incremental hasher for all three digests.
#[derive(Default, Clone)]
pub struct MultiHasher {
    size: u64,
    crc: crc32fast::Hasher,
    sha1: Sha1,
    md5: Md5,
}

impl MultiHasher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, buf: &[u8]) {
        self.size += buf.len() as u64;
        self.crc.update(buf);
        self.sha1.update(buf);
        self.md5.update(buf);
    }

    pub fn finish(self) -> Hashes {
        Hashes {
            size: self.size,
            crc: self.crc.finalize(),
            sha1: self.sha1.finalize().into(),
            md5: self.md5.finalize().into(),
        }
    }
}

pub(crate) const BUF_SIZE: usize = 256 * 1024;

/// Hashes a stream. If `skip` is given, additionally hashes the stream without its first `skip` bytes
/// (header-stripped variant); `None` is returned for it if the stream is not longer than `skip`.
pub fn hash_reader<R: Read>(mut r: R, skip: Option<u64>) -> io::Result<(Hashes, Option<Hashes>)> {
    let mut full = MultiHasher::new();
    let mut stripped = skip.map(|s| (s, MultiHasher::new()));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn known_vectors() {
        let (h, s) = hash_reader(&b"abc"[..], None).unwrap();
        assert_eq!(h.size, 3);
        assert_eq!(h.crc, 0x3524_41c2);
        assert_eq!(hex(&h.sha1), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(hex(&h.md5), "900150983cd24fb0d6963f7d28e17f72");
        assert!(s.is_none());
    }

    #[test]
    fn stripped_matches_tail_across_chunks() {
        let data: Vec<u8> = (0..BUF_SIZE * 2 + 77).map(|i| (i % 251) as u8).collect();
        for skip in [0u64, 16, 512, BUF_SIZE as u64 + 3] {
            let (_, s) = hash_reader(&data[..], Some(skip)).unwrap();
            let (want, _) = hash_reader(&data[skip as usize..], None).unwrap();
            assert_eq!(s, Some(want), "skip {skip}");
        }
        let (_, s) = hash_reader(&data[..16], Some(16)).unwrap();
        assert!(s.is_none());
    }
}
