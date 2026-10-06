//! Raw CD-ROM sectors (ECMA-130): rebuilding empty MODE1 sectors with EDC and ECC, as disc
//! dumps contain them in pregaps that CHD images leave out.

/// Bytes of a raw sector.
pub const SECTOR: usize = 2352;

/// Frames before LBA 0 (the 2-second lead-in offset of MSF addresses).
const MSF_OFFSET: u32 = 150;

const SYNC: [u8; 12] = [
    0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0,
];

struct Tables {
    f: [u8; 256],
    b: [u8; 256],
    edc: [u32; 256],
}

const TABLES: Tables = {
    let (mut f, mut b, mut edc) = ([0u8; 256], [0u8; 256], [0u32; 256]);
    let mut i = 0u32;
    while i < 256 {
        let j = (i << 1) ^ if i & 0x80 != 0 { 0x11d } else { 0 };
        f[i as usize] = j as u8;
        b[(i ^ j) as usize] = i as u8;
        let mut e = i;
        let mut k = 0;
        while k < 8 {
            e = (e >> 1) ^ if e & 1 != 0 { 0xd801_8001 } else { 0 };
            k += 1;
        }
        edc[i as usize] = e;
        i += 1;
    }
    Tables { f, b, edc }
};

/// LBA from the MSF header of a raw data sector; `None` without sync pattern.
pub fn header_lba(raw: &[u8]) -> Option<u32> {
    if raw.len() < 16 || raw[..12] != SYNC {
        return None;
    }
    let un = |v: u8| u32::from(v >> 4) * 10 + u32::from(v & 0x0f);
    (un(raw[12]) * 4500 + un(raw[13]) * 75 + un(raw[14])).checked_sub(MSF_OFFSET)
}

/// User data bytes of a MODE1 sector.
pub const USER: usize = 2048;

/// A MODE1 sector at `lba` with zeroed user data, complete with EDC and ECC.
pub fn empty_mode1(lba: u32) -> [u8; SECTOR] {
    mode1(lba, &[0u8; USER])
}

/// A MODE1 sector at `lba` holding `data`, complete with EDC and ECC: what a raw
/// (Redump) dump contains where a 2048-byte `.iso` only keeps the user data.
pub fn mode1(lba: u32, data: &[u8; USER]) -> [u8; SECTOR] {
    let mut s = [0u8; SECTOR];
    s[16..16 + USER].copy_from_slice(data);
    s[..12].copy_from_slice(&SYNC);
    let a = lba + MSF_OFFSET;
    let bcd = |v: u32| (((v / 10) << 4) | (v % 10)) as u8;
    s[12] = bcd(a / 4500);
    s[13] = bcd(a / 75 % 60);
    s[14] = bcd(a % 75);
    s[15] = 1;
    let edc = s[..2064].iter().fold(0u32, |e, &x| {
        (e >> 8) ^ TABLES.edc[((e ^ u32::from(x)) & 0xff) as usize]
    });
    s[2064..2068].copy_from_slice(&edc.to_le_bytes());
    ecc(&mut s, 86, 24, 2, 86, 0x81c);
    ecc(&mut s, 52, 43, 86, 88, 0x8c8);
    s
}

/// One ECC pass (P: 86×24, Q: 52×43) over header and data, written at `dst`.
fn ecc(s: &mut [u8; SECTOR], major: usize, minor: usize, mult: usize, inc: usize, dst: usize) {
    let size = major * minor;
    for maj in 0..major {
        let mut idx = (maj >> 1) * mult + (maj & 1);
        let (mut a, mut b) = (0u8, 0u8);
        for _ in 0..minor {
            let t = s[12 + idx];
            idx += inc;
            if idx >= size {
                idx -= size;
            }
            a ^= t;
            b ^= t;
            a = TABLES.f[a as usize];
        }
        a = TABLES.b[(TABLES.f[a as usize] ^ b) as usize];
        s[dst + maj] = a;
        s[dst + maj + major] = a ^ b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_mode1_roundtrips_header_and_has_stable_checksums() {
        let s = empty_mode1(2658);
        assert_eq!(header_lba(&s), Some(2658));
        assert_eq!(&s[12..16], &[0x00, 0x37, 0x33, 0x01]);
        // CRC of the bit-identical empty sector at this LBA in a real PC Engine CD dump.
        let mut h = crc32fast::Hasher::new();
        h.update(&s);
        assert_eq!(h.finalize(), 0xde1b_ad07);
    }

    #[test]
    fn header_lba_needs_sync() {
        assert_eq!(header_lba(&[0u8; 16]), None);
    }
}
