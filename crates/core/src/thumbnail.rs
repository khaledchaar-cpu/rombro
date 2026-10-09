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

/// Thumbnail names (without `.png`) from a server directory listing (Apache index HTML).
pub fn parse_listing(html: &str) -> Vec<String> {
    html.split("</a>")
        .filter_map(|part| {
            let text = &part[part.rfind('>')? + 1..];
            let name = text.strip_suffix(".png")?;
            Some(
                name.replace("&amp;", "&")
                    .replace("&#39;", "'")
                    .replace("&quot;", "\"")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">"),
            )
        })
        .collect()
}

/// Title without `(...)`/`[...]` tags, lowercased, thumbnail-sanitized.
fn base_title(name: &str) -> String {
    let end = [name.find(" ("), name.find(" [")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(name.len());
    file_name(name[..end].trim())
        .trim_end_matches(".png")
        .to_lowercase()
}

fn tags(name: &str) -> impl Iterator<Item = &str> {
    name.split(['(', '['])
        .skip(1)
        .filter_map(|t| t.split([')', ']']).next())
}

fn region_rank(name: &str) -> usize {
    const ORDER: [&str; 4] = ["World", "USA", "Europe", "Japan"];
    tags(name)
        .flat_map(|t| t.split(", "))
        .filter_map(|r| ORDER.iter().position(|o| *o == r))
        .min()
        .unwrap_or(ORDER.len())
}

/// Best server thumbnail for `name` when the exact name is missing: same title ignoring
/// region/tags; prefers shared tags, releases, World/USA/Europe/Japan, few extra tags.
pub fn best_match<'a>(name: &str, candidates: &'a [String]) -> Option<&'a str> {
    let base = base_title(name);
    let want: Vec<&str> = tags(name).collect();
    candidates
        .iter()
        .filter(|c| base_title(c) == base)
        .max_by_key(|c| {
            let (shared, extra) = tags(c).fold((0i32, 0i32), |(s, e), t| {
                if want.contains(&t) {
                    (s + 1, e)
                } else {
                    (s, e + 1)
                }
            });
            let unreleased = tags(c).filter(|t| !want.contains(t)).any(|t| {
                ["Beta", "Proto", "Sample", "Demo"]
                    .iter()
                    .any(|b| t.starts_with(b))
            });
            (
                shared,
                !unreleased,
                std::cmp::Reverse(region_rank(c)),
                -extra,
                std::cmp::Reverse(c.len()),
            )
        })
        .map(String::as_str)
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

    #[test]
    fn fuzzy() {
        let html = r#"<a href="x">Parent Directory</a><a href="a.png">Tetris (Japan).png</a>
<a href="b.png">Tetris (World) (Rev 1).png</a><a href="c.png">Tetris (USA) (Beta).png</a>
<a href="d.png">Tetris 2 (USA).png</a><a href="e.png">Tom &amp; Jerry (USA).png</a>"#;
        let names = parse_listing(html);
        assert_eq!(names.len(), 5);
        assert_eq!(names[4], "Tom & Jerry (USA)");
        assert_eq!(
            best_match("Tetris (World)", &names),
            Some("Tetris (World) (Rev 1)")
        );
        assert_eq!(
            best_match("Tetris (Europe)", &names),
            Some("Tetris (World) (Rev 1)")
        );
        assert_eq!(
            best_match("Tetris (Japan) (Rev 2)", &names),
            Some("Tetris (Japan)")
        );
        assert_eq!(
            best_match("Tom & Jerry (Europe)", &names),
            Some("Tom & Jerry (USA)")
        );
        assert_eq!(best_match("Tetris DX (USA)", &names), None);
    }
}
