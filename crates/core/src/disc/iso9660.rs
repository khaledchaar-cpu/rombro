//! Minimal read-only access to a data track: sector layout detection and ISO9660 root lookup.

use std::io::{self, Read, Seek, SeekFrom};

const SYNC: [u8; 12] = [
    0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0,
];
pub const USER_LEN: usize = 2048;

/// A data track with 2048-byte user data per sector (cooked `.iso` or raw 2352-byte `.bin`).
pub struct Track<R> {
    inner: R,
    sector_size: u64,
    data_offset: u64,
}

impl<R: Read + Seek> Track<R> {
    /// Detects the sector layout from the first sector's sync pattern and mode byte.
    pub fn open(mut inner: R) -> io::Result<Self> {
        let mut head = [0u8; 16];
        inner.seek(SeekFrom::Start(0))?;
        let n = read_full(&mut inner, &mut head)?;
        let (sector_size, data_offset) = if n == 16 && head[..12] == SYNC {
            // Mode 2 (XA, PS1/Saturn) carries an 8-byte subheader before user data.
            (2352, if head[15] == 2 { 24 } else { 16 })
        } else {
            (2048, 0)
        };
        Ok(Self {
            inner,
            sector_size,
            data_offset,
        })
    }

    /// Reads the user data of sector `lba`.
    pub fn sector(&mut self, lba: u32) -> io::Result<[u8; USER_LEN]> {
        let mut buf = [0u8; USER_LEN];
        self.inner.seek(SeekFrom::Start(
            u64::from(lba) * self.sector_size + self.data_offset,
        ))?;
        if read_full(&mut self.inner, &mut buf)? != USER_LEN {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        Ok(buf)
    }

    /// Reads a file from the ISO9660 root directory (case-insensitive, `;1` version ignored).
    /// `None` if the track has no ISO9660 volume or the file does not exist.
    pub fn read_root_file(&mut self, name: &str, max_len: u32) -> io::Result<Option<Vec<u8>>> {
        let Ok(pvd) = self.sector(16) else {
            return Ok(None);
        };
        if pvd[0] != 1 || &pvd[1..6] != b"CD001" {
            return Ok(None);
        }
        let root = &pvd[156..];
        let (mut lba, len) = (le32(&root[2..]), le32(&root[10..]));
        let mut left = len.min(64 * USER_LEN as u32);
        while left > 0 {
            let sec = self.sector(lba)?;
            let mut off = 0;
            while off + 33 < USER_LEN {
                let rec_len = usize::from(sec[off]);
                if rec_len == 0 || off + rec_len > USER_LEN {
                    break;
                }
                let rec = &sec[off..off + rec_len];
                let name_len = usize::from(rec[32]);
                if let Some(raw) = rec.get(33..33 + name_len) {
                    let raw = raw.split(|&b| b == b';').next().unwrap_or(raw);
                    if raw.eq_ignore_ascii_case(name.as_bytes()) && rec[25] & 2 == 0 {
                        return self.read_extent(le32(&rec[2..]), le32(&rec[10..]).min(max_len));
                    }
                }
                off += rec_len;
            }
            lba += 1;
            left = left.saturating_sub(USER_LEN as u32);
        }
        Ok(None)
    }

    fn read_extent(&mut self, lba: u32, len: u32) -> io::Result<Option<Vec<u8>>> {
        let mut out = Vec::with_capacity(len as usize);
        let mut i = 0;
        while out.len() < len as usize {
            let s = self.sector(lba + i)?;
            let take = (len as usize - out.len()).min(USER_LEN);
            out.extend_from_slice(&s[..take]);
            i += 1;
        }
        Ok(Some(out))
    }
}

fn le32(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

fn read_full<R: Read>(r: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

/// Builds synthetic disc images for tests (also used by integration tests via `disc::testimg`).
#[doc(hidden)]
pub mod testimg {
    use super::{SYNC, USER_LEN};

    /// A cooked ISO9660 image with the given root files (each fits in one sector).
    pub fn iso(files: &[(&str, &[u8])]) -> Vec<u8> {
        let first_file = 20u32;
        let mut img = vec![0u8; (first_file as usize + files.len()) * USER_LEN];
        let pvd = &mut img[16 * USER_LEN..17 * USER_LEN];
        pvd[0] = 1;
        pvd[1..6].copy_from_slice(b"CD001");
        dir_record(&mut pvd[156..190], 18, USER_LEN as u32, b"\0", true);
        let mut off = 18 * USER_LEN;
        for (i, (name, data)) in files.iter().enumerate() {
            let lba = first_file + i as u32;
            let rec_len = 33 + name.len() + (name.len() + 1) % 2;
            let mut rec = vec![0u8; rec_len];
            dir_record(&mut rec, lba, data.len() as u32, name.as_bytes(), false);
            img[off..off + rec_len].copy_from_slice(&rec);
            off += rec_len;
            let at = lba as usize * USER_LEN;
            img[at..at + data.len()].copy_from_slice(data);
        }
        img
    }

    /// Wraps cooked 2048-byte sectors into raw 2352-byte sectors (mode 1 or mode 2 form 1).
    pub fn raw(cooked: &[u8], mode: u8) -> Vec<u8> {
        let mut out = Vec::with_capacity(cooked.len() / USER_LEN * 2352);
        for s in cooked.chunks(USER_LEN) {
            let mut sec = [0u8; 2352];
            sec[..12].copy_from_slice(&SYNC);
            sec[15] = mode;
            let off = if mode == 2 { 24 } else { 16 };
            sec[off..off + s.len()].copy_from_slice(s);
            out.extend_from_slice(&sec);
        }
        out
    }

    fn dir_record(rec: &mut [u8], lba: u32, len: u32, name: &[u8], dir: bool) {
        rec[0] = (33 + name.len() + (name.len() + 1) % 2) as u8;
        rec[2..6].copy_from_slice(&lba.to_le_bytes());
        rec[10..14].copy_from_slice(&len.to_le_bytes());
        rec[25] = if dir { 2 } else { 0 };
        rec[32] = name.len() as u8;
        rec[33..33 + name.len()].copy_from_slice(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn reads_root_file_cooked_and_raw() {
        let img = testimg::iso(&[("SYSTEM.CNF;1", b"BOOT = cdrom:\\SLUS_005.94;1\r\n")]);
        for data in [img.clone(), testimg::raw(&img, 1), testimg::raw(&img, 2)] {
            let mut t = Track::open(Cursor::new(data)).unwrap();
            let f = t.read_root_file("system.cnf", 4096).unwrap().unwrap();
            assert!(f.starts_with(b"BOOT = cdrom:"));
            assert!(t.read_root_file("NOPE.BIN", 4096).unwrap().is_none());
        }
    }

    #[test]
    fn no_volume() {
        let mut t = Track::open(Cursor::new(vec![0u8; 100])).unwrap();
        assert!(t.read_root_file("SYSTEM.CNF", 10).unwrap().is_none());
    }
}
