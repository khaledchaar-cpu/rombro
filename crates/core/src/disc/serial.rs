//! Platform and serial detection from a disc's data track.

use super::iso9660::Track;
use std::io::{self, Read, Seek};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Ps1,
    Ps2,
    Psp,
    Saturn,
    SegaCd,
    Dreamcast,
    GameCube,
    Wii,
}

impl Platform {
    /// RetroArch database system name.
    pub fn system(self) -> &'static str {
        match self {
            Self::Ps1 => "Sony - PlayStation",
            Self::Ps2 => "Sony - PlayStation 2",
            Self::Psp => "Sony - PlayStation Portable",
            Self::Saturn => "Sega - Saturn",
            Self::SegaCd => "Sega - Mega-CD - Sega CD",
            Self::Dreamcast => "Sega - Dreamcast",
            Self::GameCube => "Nintendo - GameCube",
            Self::Wii => "Nintendo - Wii",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscId {
    pub platform: Platform,
    /// Serial as printed on the disc, normalized to the RDB style (`SLUS-00594`, `T-31202G`).
    pub serial: String,
    /// GameCube/Wii: (disc number, revision) from the header, to pick among `(Rev n)` entries.
    pub variant: Option<(u8, u8)>,
}

impl DiscId {
    /// Serial spellings to try against the database, most specific first.
    /// Sega discs carry `MK-81020`, while the RDB often stores just `81020`.
    pub fn lookup_keys(&self) -> Vec<String> {
        let mut keys = vec![self.serial.clone()];
        if let Some(s) = self.serial.strip_prefix("MK-") {
            keys.push(s.to_owned());
        }
        // Sega headers often omit the dash the databases use (`T40201N` vs `T-40201N`).
        let letters = self
            .serial
            .bytes()
            .take_while(u8::is_ascii_alphabetic)
            .count();
        if letters > 0 && self.serial[letters..].starts_with(|c: char| c.is_ascii_digit()) {
            keys.push(format!(
                "{}-{}",
                &self.serial[..letters],
                &self.serial[letters..]
            ));
        }
        keys
    }
}

/// Identifies platform and serial of a data track, if recognizable.
pub fn detect<R: Read + Seek>(track: &mut Track<R>) -> io::Result<Option<DiscId>> {
    if let Ok(s0) = track.sector(0)
        && let Some(id) = sega_header(&s0)
    {
        return Ok(Some(id));
    }
    if let Some(cnf) = track.read_root_file("SYSTEM.CNF", 4096)? {
        return Ok(parse_system_cnf(&String::from_utf8_lossy(&cnf)));
    }
    if let Some(umd) = track.read_root_file("UMD_DATA.BIN", 256)? {
        let s = String::from_utf8_lossy(&umd);
        let serial = s.split('|').next().unwrap_or_default().trim();
        if is_sony_serial(serial) {
            return Ok(Some(DiscId {
                platform: Platform::Psp,
                serial: serial.to_owned(),
                variant: None,
            }));
        }
    }
    Ok(None)
}

/// Sega CD / Saturn / Dreamcast system area in sector 0.
fn sega_header(s: &[u8]) -> Option<DiscId> {
    let (platform, field) = if s.starts_with(b"SEGA SEGASATURN") {
        (Platform::Saturn, &s[0x20..0x2a])
    } else if s.starts_with(b"SEGA SEGAKATANA") {
        (Platform::Dreamcast, &s[0x40..0x4a])
    } else if s.starts_with(b"SEGADISCSYSTEM") {
        // "GM T-45034 -00": type, serial, revision.
        (Platform::SegaCd, &s[0x183..0x18b])
    } else {
        return None;
    };
    let serial = String::from_utf8_lossy(field).trim().to_owned();
    (!serial.is_empty()).then_some(DiscId {
        platform,
        serial,
        variant: None,
    })
}

/// PS1 (`BOOT = cdrom:\SLUS_005.94;1`) or PS2 (`BOOT2 = cdrom0:\SLUS_205.25;1`).
pub fn parse_system_cnf(text: &str) -> Option<DiscId> {
    for line in text.lines() {
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let platform = match key.trim().to_ascii_uppercase().as_str() {
            "BOOT2" => Platform::Ps2,
            "BOOT" => Platform::Ps1,
            _ => continue,
        };
        let file = val.trim().rsplit(['\\', ':', '/']).next()?;
        let file = file.split(';').next()?;
        let serial: String = file
            .chars()
            .filter(|&c| c != '.')
            .map(|c| {
                if c == '_' {
                    '-'
                } else {
                    c.to_ascii_uppercase()
                }
            })
            .collect();
        return is_sony_serial(&serial).then_some(DiscId {
            platform,
            serial,
            variant: None,
        });
    }
    None
}

/// `ABCD-12345` (letters, dash, digits).
fn is_sony_serial(s: &str) -> bool {
    s.split_once('-').is_some_and(|(a, b)| {
        a.len() == 4
            && a.bytes().all(|c| c.is_ascii_alphabetic())
            && b.len() == 5
            && b.bytes().all(|c| c.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use super::super::iso9660::testimg;
    use super::*;
    use std::io::Cursor;

    fn detect_bytes(img: Vec<u8>) -> Option<DiscId> {
        detect(&mut Track::open(Cursor::new(img)).unwrap()).unwrap()
    }

    #[test]
    fn system_cnf_variants() {
        let p1 = parse_system_cnf("BOOT = cdrom:\\SLUS_005.94;1\r\nTCB = 4\r\n").unwrap();
        assert_eq!(
            (p1.platform, p1.serial.as_str()),
            (Platform::Ps1, "SLUS-00594")
        );
        let p2 = parse_system_cnf("BOOT2 = cdrom0:\\SLUS_205.25;1\nVER = 1.00\n").unwrap();
        assert_eq!(
            (p2.platform, p2.serial.as_str()),
            (Platform::Ps2, "SLUS-20525")
        );
        let p3 = parse_system_cnf("BOOT=cdrom:slps_123.45").unwrap();
        assert_eq!(p3.serial, "SLPS-12345");
        assert!(parse_system_cnf("BOOT = cdrom:\\PSX.EXE;1").is_none());
    }

    #[test]
    fn ps2_and_psp_from_iso() {
        let ps2 = testimg::iso(&[("SYSTEM.CNF;1", b"BOOT2 = cdrom0:\\SLES_509.33;1\n")]);
        assert_eq!(detect_bytes(ps2).unwrap().serial, "SLES-50933");
        let psp = testimg::iso(&[("UMD_DATA.BIN;1", b"ULUS-10041|0001|G|")]);
        let id = detect_bytes(psp).unwrap();
        assert_eq!(
            (id.platform, id.serial.as_str()),
            (Platform::Psp, "ULUS-10041")
        );
        assert!(detect_bytes(testimg::iso(&[("README.TXT;1", b"hi")])).is_none());
    }

    #[test]
    fn sega_headers() {
        let mut sat = vec![0u8; 2048];
        sat[..16].copy_from_slice(b"SEGA SEGASATURN ");
        sat[0x20..0x2a].copy_from_slice(b"MK-81020  ");
        let id = detect_bytes(testimg::raw(&sat, 1)).unwrap();
        assert_eq!(id.platform, Platform::Saturn);
        assert_eq!(id.lookup_keys(), ["MK-81020", "81020"]);
        let dc = DiscId {
            platform: id.platform,
            serial: "T40201N".into(),
            variant: None,
        };
        assert_eq!(dc.lookup_keys(), ["T40201N", "T-40201N"]);

        let mut scd = vec![0u8; 2048];
        scd[..14].copy_from_slice(b"SEGADISCSYSTEM");
        scd[0x180..0x18e].copy_from_slice(b"GM T-45034 -00");
        assert_eq!(detect_bytes(scd).unwrap().serial, "T-45034");

        let mut dc = vec![0u8; 2048];
        dc[..16].copy_from_slice(b"SEGA SEGAKATANA ");
        dc[0x40..0x4a].copy_from_slice(b"T-14402M  ");
        let id = detect_bytes(dc).unwrap();
        assert_eq!(
            (id.platform, id.serial.as_str()),
            (Platform::Dreamcast, "T-14402M")
        );
    }
}
