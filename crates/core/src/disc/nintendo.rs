//! GameCube/Wii images. Compressed containers (RVZ/WIA, WBFS, CISO) can't be hashed against
//! the databases, but each keeps a copy of the disc header with the game ID.

use super::serial::{DiscId, Platform};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

const GC_MAGIC: u32 = 0xC233_9F3D;
const WII_MAGIC: u32 = 0x5D1C_9EA3;

/// Container formats identified by game ID only.
pub fn is_container_ext(ext: &str) -> bool {
    matches!(ext, "rvz" | "wia" | "wbfs" | "ciso")
}

/// Platform and game ID (`GALE01`) from a disc header (first 0x20 bytes), if valid.
pub fn parse_header(h: &[u8]) -> Option<DiscId> {
    let word = |o: usize| Some(u32::from_be_bytes(h.get(o..o + 4)?.try_into().ok()?));
    let platform = if word(0x18)? == WII_MAGIC {
        Platform::Wii
    } else if word(0x1C)? == GC_MAGIC {
        Platform::GameCube
    } else {
        return None;
    };
    let id = std::str::from_utf8(&h[..6]).ok()?;
    if !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    Some(DiscId {
        platform,
        serial: id.to_owned(),
        variant: Some((h[6], h[7])),
    })
}

/// Reads the disc header of a plain image or a compressed container.
pub fn identify(path: &Path) -> io::Result<Option<DiscId>> {
    let mut f = File::open(path)?;
    let mut magic = [0u8; 12];
    if f.read(&mut magic)? < magic.len() {
        return Ok(None);
    }
    let offset = match &magic[..4] {
        // WIA/RVZ: 0x48-byte header 1, then header 2 with the disc header at +0x10
        b"RVZ\x01" | b"WIA\x01" => 0x58,
        // WBFS: disc header copy at the second "hd sector" (1 << shift)
        b"WBFS" => 1u64 << magic[8].min(16),
        // CISO: block map in the first 0x8000 bytes, data follows
        b"CISO" => 0x8000,
        _ => 0,
    };
    let mut h = [0u8; 0x20];
    f.seek(SeekFrom::Start(offset))?;
    if f.read(&mut h)? < h.len() {
        return Ok(None);
    }
    Ok(parse_header(&h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(id: &[u8; 6], disc: u8, ver: u8, wii: bool) -> Vec<u8> {
        let mut h = vec![0u8; 0x20];
        h[..6].copy_from_slice(id);
        h[6] = disc;
        h[7] = ver;
        if wii {
            h[0x18..0x1C].copy_from_slice(&WII_MAGIC.to_be_bytes());
        } else {
            h[0x1C..0x20].copy_from_slice(&GC_MAGIC.to_be_bytes());
        }
        h
    }

    #[test]
    fn reads_ids_from_plain_and_container_images() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("a.iso");
        std::fs::write(&plain, header(b"GALE01", 0, 2, false)).unwrap();
        let id = identify(&plain).unwrap().unwrap();
        assert_eq!(
            (id.platform, id.serial.as_str(), id.variant),
            (Platform::GameCube, "GALE01", Some((0, 2)))
        );

        let mut rvz = b"RVZ\x01".to_vec();
        rvz.resize(0x58, 0);
        rvz.extend(header(b"RSBE01", 0, 1, true));
        let p = dir.path().join("b.rvz");
        std::fs::write(&p, rvz).unwrap();
        let id = identify(&p).unwrap().unwrap();
        assert_eq!((id.platform, id.serial.as_str()), (Platform::Wii, "RSBE01"));

        let mut wbfs = b"WBFS\0\0\0\0\x09".to_vec();
        wbfs.resize(512, 0);
        wbfs.extend(header(b"RMGE01", 0, 0, true));
        let p = dir.path().join("c.wbfs");
        std::fs::write(&p, wbfs).unwrap();
        assert_eq!(identify(&p).unwrap().unwrap().serial, "RMGE01");

        let p = dir.path().join("junk.iso");
        std::fs::write(&p, [7u8; 64]).unwrap();
        assert!(identify(&p).unwrap().is_none());
    }
}
