//! Combined Sufami Turbo images: the BIOS (256 KiB, mirrored to 1 MiB) followed by one or two
//! carts, as made for emulators without Sufami Turbo support. The databases list BIOS and carts
//! separately, so such a file is treated like an archive whose members are byte ranges.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::Path;

const BIOS: u64 = 0x4_0000;
const BIOS_AREA: u64 = 0x10_0000;
const SLOT: u64 = 0x10_0000;
const HALF: u64 = 0x8_0000;
/// Every Sufami Turbo cart starts with this.
const CART_MAGIC: &[u8] = b"BANDAI SFC-ADX";
/// Largest combined image (BIOS area + two 1 MiB carts + copier header).
const MAX_LEN: u64 = BIOS_AREA + 2 * SLOT + 512;

/// Members of a combined image: name and byte range in the file.
pub type Layout = Vec<(String, Range<u64>)>;

/// Member name of the BIOS (`.sfc`: the databases list it as a Super Famicom ROM).
pub const BIOS_MEMBER: &str = "SuFami Turbo (Japan).sfc";

/// Whether a file with this name and size may be a combined image (worth reading whole).
pub fn candidate(name: &Path, len: u64) -> bool {
    let ext = name
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    matches!(ext.as_str(), "smc" | "sfc" | "swc" | "fig")
        && (BIOS_AREA + HALF..=MAX_LEN).contains(&len)
}

/// The members of a combined image at `path` (name, byte range), or `None` if it is none.
pub fn layout(path: &Path) -> io::Result<Option<Layout>> {
    let len = std::fs::metadata(path)?.len();
    if !candidate(path, len) {
        return Ok(None);
    }
    let mut data = Vec::with_capacity(len as usize);
    File::open(path)?.read_to_end(&mut data)?;
    Ok(parse(&data))
}

/// Members of a combined image held in memory (e.g. a zip member).
pub fn parse(data: &[u8]) -> Option<Layout> {
    let hdr = (data.len() % 0x400) as u64;
    let body = &data[hdr as usize..];
    let len = body.len() as u64;
    if len < BIOS_AREA + HALF || (len - BIOS_AREA) % HALF != 0 {
        return None;
    }
    let bios = &body[..BIOS as usize];
    if !body[..BIOS_AREA as usize]
        .chunks(BIOS as usize)
        .all(|c| c == bios)
    {
        return None;
    }
    let mut out = vec![(BIOS_MEMBER.to_owned(), hdr..hdr + BIOS)];
    let mut at = BIOS_AREA;
    for slot in ["A", "B"] {
        if at >= len {
            break;
        }
        let block = &body[at as usize..(at + SLOT).min(len) as usize];
        if !block.starts_with(CART_MAGIC) {
            return None;
        }
        let (h, rest) = block.split_at(block.len().min(HALF as usize));
        let size = if rest.is_empty() || rest == h {
            HALF
        } else {
            SLOT
        };
        out.push((format!("Slot {slot}.st"), hdr + at..hdr + at + size));
        at += SLOT;
    }
    (at >= len).then_some(out)
}

/// Copies the byte range of `member` to `out`.
pub fn write_member(path: &Path, member: &str, out: &mut impl Write) -> io::Result<()> {
    let range = layout(path)?
        .and_then(|l| l.into_iter().find(|(n, _)| n == member))
        .map(|(_, r)| r)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{member} not in {}", path.display()),
            )
        })?;
    let mut f = File::open(path)?;
    f.seek(SeekFrom::Start(range.start))?;
    io::copy(&mut f.take(range.end - range.start), out).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cart(fill: u8, size: u64) -> Vec<u8> {
        let mut c = CART_MAGIC.to_vec();
        c.resize(size as usize, fill);
        c
    }

    fn image(carts: &[Vec<u8>]) -> Vec<u8> {
        let bios: Vec<u8> = (0..BIOS).map(|i| (i % 251) as u8).collect();
        let mut d = bios.repeat(4);
        for c in carts {
            // carts are mirrored to fill their 1 MiB slot
            d.extend(c.repeat((SLOT / c.len() as u64) as usize));
        }
        d
    }

    #[test]
    fn finds_bios_and_carts() {
        let l = parse(&image(&[cart(1, HALF), cart(2, SLOT)])).unwrap();
        let names: Vec<_> = l.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, [BIOS_MEMBER, "Slot A.st", "Slot B.st"]);
        assert_eq!(l[1].1, BIOS_AREA..BIOS_AREA + HALF);
        assert_eq!(l[2].1, BIOS_AREA + SLOT..BIOS_AREA + 2 * SLOT);
    }

    #[test]
    fn skips_copier_header_and_unmirrored_cart() {
        let mut d = vec![0u8; 512];
        d.extend(image(&[]));
        d.extend(cart(3, HALF));
        let l = parse(&d).unwrap();
        assert_eq!(l[0].1, 512..512 + BIOS);
        assert_eq!(l[1].1, 512 + BIOS_AREA..512 + BIOS_AREA + HALF);
    }

    #[test]
    fn rejects_plain_roms() {
        assert!(parse(&vec![7u8; (BIOS_AREA + HALF) as usize]).is_none());
        let mut d = image(&[cart(1, HALF)]);
        d[BIOS_AREA as usize] = b'X';
        assert!(parse(&d).is_none());
    }

    #[test]
    fn scans_and_extracts_members() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("combo.smc");
        let a = cart(1, HALF);
        std::fs::write(&p, image(std::slice::from_ref(&a))).unwrap();
        let roms = crate::scan::scan_file(&p).unwrap();
        let members: Vec<_> = roms.iter().map(|r| r.member.as_deref()).collect();
        assert_eq!(members, [Some(BIOS_MEMBER), Some("Slot A.st")]);
        assert_eq!(roms[1].hashes.crc, crc32fast::hash(&a));
        let out = tmp.path().join("a.st");
        crate::archive::extract(&p, "Slot A.st", &out).unwrap();
        assert_eq!(std::fs::read(out).unwrap(), a);
    }

    #[test]
    fn scans_and_extracts_parts_of_a_zipped_image() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("combo.zip");
        let a = cart(5, HALF);
        let mut z = zip::ZipWriter::new(File::create(&p).unwrap());
        z.start_file("combo.smc", zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(&image(std::slice::from_ref(&a))).unwrap();
        z.finish().unwrap();
        let roms = crate::scan::scan_file(&p).unwrap();
        let members: Vec<_> = roms.iter().map(|r| r.member.as_deref()).collect();
        let part = "combo.smc/Slot A.st";
        assert_eq!(
            members,
            [None, Some(&*format!("combo.smc/{BIOS_MEMBER}")), Some(part)]
        );
        assert_eq!(roms[2].hashes.crc, crc32fast::hash(&a));
        let out = tmp.path().join("a.st");
        crate::archive::extract(&p, part, &out).unwrap();
        assert_eq!(std::fs::read(out).unwrap(), a);
    }
}
