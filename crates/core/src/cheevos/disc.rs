//! rcheevos hashes of disc systems (`rc_hash_disc.c`) and the Nintendo DS (`rc_hash_rom.c`).

use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use md5::digest::Update;
use md5::{Digest, Md5};

use super::cd::{CdTrack, Which};
use super::{Method, hex};

/// RA hash of a disc (or DS cartridge) at `path` for a disc `method`; `None` if the content
/// does not look like that system's disc.
pub fn hash(path: &Path, method: Method) -> io::Result<Option<String>> {
    let md5 = match method {
        Method::Psx => playstation(path, "BOOT", "cdrom:", true)?,
        Method::Ps2 => playstation(path, "BOOT2", "cdrom0:", false)?,
        Method::Psp => psp(path)?,
        Method::SegaCd => sega_cd(path)?,
        Method::PceCd => pce_cd(path)?,
        Method::Dreamcast => dreamcast(path)?,
        Method::ThreeDo => three_do(path)?,
        Method::Nds => nds(path)?,
        _ => None,
    };
    Ok(md5.map(|m| hex(&m.finalize())))
}

fn open(path: &Path, which: Which) -> io::Result<Option<CdTrack>> {
    CdTrack::open(path, which)
}

/// PS1/PS2: boot executable named in `SYSTEM.CNF` (PS1 falls back to `PSX.EXE`), hashed with
/// its name; a PS-X EXE's size comes from its header.
fn playstation(path: &Path, key: &str, prefix: &str, psx: bool) -> io::Result<Option<Md5>> {
    let Some(mut t) = open(path, Which::Number(1))? else {
        return Ok(None);
    };
    let mut found = boot_name(&mut t, key, prefix).and_then(|n| Some((t.find_file(&n)?, n)));
    if found.is_none() && psx {
        found = t.find_file("PSX.EXE").map(|f| (f, "PSX.EXE".to_owned()));
    }
    let Some(((lba, mut size), name)) = found else {
        return Ok(None);
    };
    let Some(head) = t.sector(lba) else {
        return Ok(None);
    };
    if psx && head.starts_with(b"PS-X EXE") {
        size = u32::from_le_bytes([head[28], head[29], head[30], head[31]]) + 2048;
    }
    let mut md5 = Md5::new();
    Update::update(&mut md5, name.as_bytes());
    Ok(t.hash_file(&mut md5, lba, size).map(|()| md5))
}

/// Executable path after `<key> = <prefix>\` in `SYSTEM.CNF`, up to whitespace or `;`.
fn boot_name(t: &mut CdTrack, key: &str, prefix: &str) -> Option<String> {
    let (lba, _) = t.find_file("SYSTEM.CNF")?;
    let sec = t.sector(lba)?;
    let text = &sec[..sec.iter().position(|&b| b == 0).unwrap_or(sec.len() - 1)];
    let text = String::from_utf8_lossy(text);
    text.lines().find_map(|line| {
        // rcheevos only matches the key at the start of a line
        let rest = line
            .strip_prefix(key)?
            .trim_start()
            .strip_prefix('=')?
            .trim_start();
        let rest = rest
            .strip_prefix(prefix)
            .unwrap_or(rest)
            .trim_start_matches('\\');
        let end = rest
            .find(|c: char| c.is_ascii_whitespace() || c == ';')
            .unwrap_or(rest.len());
        Some(rest[..end.min(63)].to_owned())
    })
}

/// PSP: `PARAM.SFO` then `EBOOT.BIN` from the UMD image; a `.pbp` is hashed whole.
fn psp(path: &Path) -> io::Result<Option<Md5>> {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pbp"))
    {
        let mut md5 = Md5::new();
        let mut r = BufReader::new(File::open(path)?).take(super::MAX_BYTES);
        io::copy(&mut r, &mut Writer(&mut md5))?;
        return Ok(Some(md5));
    }
    let Some(mut t) = open(path, Which::Number(1))? else {
        return Ok(None);
    };
    let mut md5 = Md5::new();
    for file in ["PSP_GAME\\PARAM.SFO", "PSP_GAME\\SYSDIR\\EBOOT.BIN"] {
        let Some((lba, size)) = t.find_file(file) else {
            return Ok(None);
        };
        if t.hash_file(&mut md5, lba, size).is_none() {
            return Ok(None);
        }
    }
    Ok(Some(md5))
}

/// Sega CD / Saturn: the first 512 bytes of sector 0 (volume and ROM header).
fn sega_cd(path: &Path) -> io::Result<Option<Md5>> {
    let Some(mut t) = open(path, Which::Number(1))? else {
        return Ok(None);
    };
    let Some(s) = t.sector(0) else {
        return Ok(None);
    };
    if !s.starts_with(b"SEGADISCSYSTEM  ") && !s.starts_with(b"SEGA SEGASATURN ") {
        return Ok(None);
    }
    Ok(Some(Md5::new_with_prefix(&s[..512])))
}

/// PC Engine CD: title from the boot header in sector 1 of the first data track, then the
/// program sectors it names.
fn pce_cd(path: &Path) -> io::Result<Option<Md5>> {
    let Some(mut t) = open(path, Which::FirstData)? else {
        return Ok(None);
    };
    let first = t.first_sector();
    let Some(h) = t.sector(first + 1) else {
        return Ok(None);
    };
    if &h[32..55] != b"PC Engine CD-ROM SYSTEM" {
        // GameExpress discs (BOOT.BIN on a plain filesystem) are not supported
        return Ok(None);
    }
    let mut md5 = Md5::new_with_prefix(&h[106..128]);
    let start = u32::from_be_bytes([0, h[0], h[1], h[2]]) + first;
    for lba in start..start + u32::from(h[3]) {
        Update::update(&mut md5, &t.sector(lba).unwrap_or([0; 2048]));
    }
    Ok(Some(md5))
}

/// Dreamcast: IP.BIN meta (256 bytes) of track 3, then the boot file it names, which usually
/// lives in the last track.
fn dreamcast(path: &Path) -> io::Result<Option<Md5>> {
    let ip = |t: &mut CdTrack| {
        t.sector(t.first_sector())
            .filter(|s| s.starts_with(b"SEGA SEGAKATANA "))
    };
    let mut track = open(path, Which::Number(3))?;
    let mut meta = track.as_mut().and_then(ip);
    if meta.is_none() {
        track = open(path, Which::FirstData)?;
        meta = track.as_mut().and_then(ip);
    }
    let (Some(mut t), Some(meta)) = (track, meta) else {
        return Ok(None);
    };
    let mut md5 = Md5::new_with_prefix(&meta[..256]);
    let boot = &meta[96..112];
    let len = boot
        .iter()
        .position(|b| b.is_ascii_whitespace())
        .unwrap_or(16);
    if len == 0 {
        return Ok(None);
    }
    let Some((lba, size)) = t.find_file(&String::from_utf8_lossy(&boot[..len])) else {
        return Ok(None);
    };
    if t.sector(lba).is_none() {
        match open(path, Which::Last)? {
            Some(last) => t = last,
            None => return Ok(None),
        }
    }
    Ok(t.hash_file(&mut md5, lba, size).map(|()| md5))
}

/// 3DO: Opera volume header (132 bytes), then the `LaunchMe` file of the root directory.
fn three_do(path: &Path) -> io::Result<Option<Md5>> {
    let Some(mut t) = open(path, Which::Number(1))? else {
        return Ok(None);
    };
    let Some(vol) = t.sector(0) else {
        return Ok(None);
    };
    if vol[..7] != [1, 0x5a, 0x5a, 0x5a, 0x5a, 0x5a, 1] {
        return Ok(None);
    }
    let be24 = |b: &[u8]| u32::from_be_bytes([0, b[0], b[1], b[2]]);
    let mut md5 = Md5::new_with_prefix(&vol[..132]);
    let mut block = be24(&vol[0x4d..]);
    let root = be24(&vol[0x65..]) * block;
    let mut sector = root / 2048;
    let (lba, mut size) = loop {
        let Some(b) = t.sector(sector) else {
            return Ok(None);
        };
        let mut off = usize::from(b[0x12]) << 8 | usize::from(b[0x13]);
        let stop = be24(&b[0x0d..]) as usize;
        while off < stop && off + 0x48 <= b.len() {
            let e = &b[off..];
            let name = &e[0x20..0x40];
            let name = &name[..name.iter().position(|&c| c == 0).unwrap_or(32)];
            if e[3] == 2 && name.eq_ignore_ascii_case(b"LaunchMe") {
                block = be24(&e[0x0d..]);
                break;
            }
            off += 0x48 + usize::from(e[0x43]) * 4;
        }
        if off < stop && off + 0x48 <= b.len() {
            let e = &b[off..];
            break (be24(&e[0x45..]) * block / 2048, be24(&e[0x11..]) as usize);
        }
        let next = u32::from(b[2]) << 8 | u32::from(b[3]);
        if next == 0xffff {
            return Ok(None);
        }
        sector = (root + next * block) / 2048;
    };
    let mut lba = lba;
    while size > 0 {
        let s = t.sector(lba).unwrap_or([0; 2048]);
        let n = size.min(2048);
        Update::update(&mut md5, &s[..n]);
        size -= n;
        lba += 1;
    }
    Ok(Some(md5))
}

/// Nintendo DS: header (0x160 bytes), ARM9 and ARM7 code, icon/title block (0xA00 bytes);
/// a 512-byte SuperCard header is skipped.
fn nds(path: &Path) -> io::Result<Option<Md5>> {
    let mut f = BufReader::new(File::open(path)?);
    let mut h = [0u8; 512];
    if read_at(&mut f, 0, &mut h)? != 512 {
        return Ok(None);
    }
    let mut base = 0;
    if h[..4] == [0x2e, 0, 0, 0xea] && h[0xb0..0xb4] == [0x44, 0x46, 0x96, 0] {
        base = 512;
        read_at(&mut f, base, &mut h)?;
    }
    let le = |o: usize| u32::from_le_bytes([h[o], h[o + 1], h[o + 2], h[o + 3]]);
    let (arm9, arm9_len, arm7, arm7_len, icon) = (le(0x20), le(0x2c), le(0x30), le(0x3c), le(0x68));
    if u64::from(arm9_len) + u64::from(arm7_len) > 16 * 1024 * 1024 {
        return Ok(None);
    }
    let mut md5 = Md5::new_with_prefix(&h[..0x160]);
    for (at, len) in [(arm9, arm9_len), (arm7, arm7_len), (icon, 0xa00)] {
        let mut buf = vec![0u8; len as usize];
        read_at(&mut f, base + u64::from(at), &mut buf)?;
        Update::update(&mut md5, &buf);
    }
    Ok(Some(md5))
}

/// Reads as much of `buf` as the file has at `pos` (rest stays zero).
fn read_at<R: Read + Seek>(r: &mut R, pos: u64, buf: &mut [u8]) -> io::Result<usize> {
    r.seek(SeekFrom::Start(pos))?;
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..])? {
            0 => break,
            k => n += k,
        }
    }
    Ok(n)
}

struct Writer<'a>(&'a mut Md5);

impl io::Write for Writer<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Update::update(self.0, buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
