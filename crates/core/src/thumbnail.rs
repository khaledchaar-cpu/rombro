//! libretro thumbnail naming: server URLs and local cache layout (mirrors RetroArch).
use std::path::{Path, PathBuf};

pub const BASE_URL: &str = "https://thumbnails.libretro.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Boxart,
    Snap,
    Title,
}

impl Kind {
    pub fn dir(self) -> &'static str {
        match self {
            Kind::Boxart => "Named_Boxarts",
            Kind::Snap => "Named_Snaps",
            Kind::Title => "Named_Titles",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "boxart" => Some(Kind::Boxart),
            "snap" => Some(Kind::Snap),
            "title" => Some(Kind::Title),
            _ => None,
        }
    }
}

/// Thumbnail file name for a game name (RetroArch: `&*/:`<>?\|` → `_`, plus `.png`).
pub fn file_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| match c {
            '&' | '*' | '/' | ':' | '`' | '<' | '>' | '?' | '\\' | '|' => '_',
            c => c,
        })
        .collect();
    out.push_str(".png");
    out
}

fn encode(seg: &str) -> String {
    let mut out = String::with_capacity(seg.len());
    for b in seg.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Download URL of a thumbnail on the libretro server.
pub fn url(system: &str, name: &str, kind: Kind) -> String {
    format!(
        "{BASE_URL}/{}/{}/{}",
        encode(system),
        kind.dir(),
        encode(&file_name(name))
    )
}

/// Local cache path (`<root>/<system>/<Named_*>/<name>.png`, same layout as RetroArch).
pub fn cache_path(root: &Path, system: &str, name: &str, kind: Kind) -> PathBuf {
    root.join(system.replace('/', "_"))
        .join(kind.dir())
        .join(file_name(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_urls() {
        assert_eq!(file_name("A & B: C?"), "A _ B_ C_.png");
        assert_eq!(
            url(
                "Nintendo - Game Boy",
                "Tetris (World) (Rev 1)",
                Kind::Boxart
            ),
            "https://thumbnails.libretro.com/Nintendo%20-%20Game%20Boy/Named_Boxarts/Tetris%20%28World%29%20%28Rev%201%29.png"
        );
        let p = cache_path(Path::new("/c"), "Sega - X", "Y", Kind::Snap);
        assert_eq!(p, Path::new("/c/Sega - X/Named_Snaps/Y.png"));
        assert_eq!(Kind::parse("title"), Some(Kind::Title));
    }
}
