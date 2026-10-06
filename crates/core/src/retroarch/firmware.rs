//! Firmware RetroArch cores expect in the `system` folder, from libretro's `System.dat`
//! (bundled snapshot; clrmamepro format).

/// One firmware file: path relative to the system folder, and its digests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Firmware {
    /// System the file belongs to (`comment` line before it).
    pub system: String,
    /// Path inside the system folder, e.g. `scph5501.bin` or `fbneo/neogeo.zip`.
    pub path: String,
    pub size: u64,
    pub sha1: [u8; 20],
}

const SYSTEM_DAT: &str = include_str!("../../data/System.dat");

/// All firmware of the bundled `System.dat`.
pub fn bundled() -> Vec<Firmware> {
    parse(SYSTEM_DAT)
}

/// Library (RDB) systems a `System.dat` system stands for, where the names differ.
pub fn library_systems(system: &str) -> &'static [&'static str] {
    match system {
        "3DO Company, The - 3DO" => &["The 3DO Company - 3DO"],
        "Atari - 400-800" => &["Atari - 8-bit Family"],
        "EPOCH/YENO Super Cassette Vision" => &["Epoch - Super Cassette Vision"],
        "Enterprise - 64/128" => &["Enterprise - 128"],
        "Fairchild Channel F" => &["Fairchild - Channel F"],
        "Id Software - Doom" => &["DOOM"],
        "J2ME" => &["Mobile - J2ME"],
        "NEC - PC-8801" => &["NEC - PC-8001 - PC-8801", "NEC - PC-88"],
        "NEC - PC Engine - TurboGrafx 16 - SuperGrafx" => &[
            "NEC - PC Engine - TurboGrafx 16",
            "NEC - PC Engine CD - TurboGrafx-CD",
            "NEC - PC Engine SuperGrafx",
        ],
        "Nintendo - Famicom Disk System" => &["Nintendo - Family Computer Disk System"],
        "Nintendo - Gameboy" => &["Nintendo - Game Boy"],
        "Nintendo - Gameboy Color" => &["Nintendo - Game Boy Color"],
        "Nintendo - SuFami Turbo" => &["Nintendo - Sufami Turbo"],
        "Phillips - Videopac+" => &["Philips - Videopac+"],
        "SNK - NeoGeo CD" => &["SNK - Neo Geo CD"],
        "Sega - Dreamcast-based Arcade" => &["Sega - Naomi", "Sega - Naomi 2", "Atomiswave"],
        "Sega - Mega CD - Sega CD" => &["Sega - Mega-CD - Sega CD"],
        _ => &[],
    }
}

pub fn parse(text: &str) -> Vec<Firmware> {
    let mut system = String::new();
    let mut out = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(c) = line.strip_prefix("comment \"") {
            system = c.trim_end_matches('"').to_owned();
        } else if let Some(rom) = line.strip_prefix("rom (") {
            if let Some(f) = rom_line(rom, &system) {
                out.push(f);
            }
        }
    }
    out
}

/// `name "x y.bin" size 1 crc … sha1 … )` → firmware.
fn rom_line(rom: &str, system: &str) -> Option<Firmware> {
    let rom = rom.trim().strip_prefix("name ")?;
    let (path, rest) = match rom.strip_prefix('"') {
        Some(q) => q.split_once('"')?,
        None => rom.split_once(' ')?,
    };
    let mut words = rest.split_whitespace();
    let (mut size, mut sha1) = (None, None);
    while let Some(k) = words.next() {
        let v = words.next().unwrap_or_default();
        match k {
            "size" => size = v.parse().ok(),
            "sha1" => sha1 = hex20(v),
            _ => {}
        }
    }
    Some(Firmware {
        system: system.to_owned(),
        path: path.to_owned(),
        size: size?,
        sha1: sha1?,
    })
}

fn hex20(s: &str) -> Option<[u8; 20]> {
    let mut out = [0u8; 20];
    if s.len() != 40 {
        return None;
    }
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_and_plain_names() {
        let f = parse(
            "game (\n\tcomment \"Nintendo - SuFami Turbo\"\n\
             \trom ( name STBIOS.bin size 262144 crc 9B4CA911 md5 d3 sha1 ef86ea192eed03d5c413fdbbfd46043be1d7a127 )\n\
             \tcomment \"Arcade\"\n\
             \trom ( name \"fbneo/neogeo.zip\" size 5 crc 1 sha1 deb62b0074b8cae4f162c257662136733cfc76ad )\n)",
        );
        assert_eq!(f.len(), 2);
        assert_eq!(
            (f[0].system.as_str(), f[0].path.as_str()),
            ("Nintendo - SuFami Turbo", "STBIOS.bin")
        );
        assert_eq!(f[0].sha1[0], 0xef);
        assert_eq!(f[1].path, "fbneo/neogeo.zip");
    }

    #[test]
    fn bundled_dat_lists_known_bios() {
        let f = bundled();
        assert!(f.len() > 400);
        assert!(f.iter().any(|f| f.path == "scph5501.bin"));
    }
}
