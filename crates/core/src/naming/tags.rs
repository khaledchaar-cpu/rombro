//! Parser for No-Intro / Redump / TOSEC-style names: `Title (Region) (Langs) (Rev 1) (Beta) [h]`.

/// Release flags that 1G1R rules can exclude or penalize.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    pub beta: bool,
    pub proto: bool,
    pub demo: bool,
    pub kiosk: bool,
    pub sample: bool,
    pub unlicensed: bool,
    pub pirate: bool,
    pub bios: bool,
    pub aftermarket: bool,
    pub virtual_console: bool,
    /// Digital re-release / collection extract (`Digital`, `Switch Online`, `Collection`).
    pub rerelease: bool,
    pub hack: bool,
    pub translation: bool,
    pub bad_dump: bool,
    /// `(Alt 1)`, `[a]`: alternative dump of the same release.
    pub alt: bool,
}

impl Flags {
    /// Not an official, final, retail release.
    pub fn is_unofficial(&self) -> bool {
        self.beta
            || self.proto
            || self.demo
            || self.kiosk
            || self.sample
            || self.pirate
            || self.hack
            || self.translation
            || self.bad_dump
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameInfo<'a> {
    /// Title without any tags, e.g. `Legend of Zelda, The - A Link to the Past`.
    pub title: &'a str,
    pub regions: Vec<&'a str>,
    /// Language codes as written (`En`, `Fr`, ...).
    pub languages: Vec<&'a str>,
    /// Revision / version rank: `Rev 1` → 1, `Rev A`/`REV-A` → 1, `v1.1` → 1001; none → 0.
    pub revision: u32,
    /// Disc/disk/side number for multi-media sets (`Disc 2`, `Disk B`, `Side B` → 2).
    pub disc: Option<u32>,
    pub flags: Flags,
}

const REGIONS: &[&str] = &[
    "World",
    "Europe",
    "USA",
    "Japan",
    "Germany",
    "France",
    "Spain",
    "Italy",
    "UK",
    "Australia",
    "Korea",
    "China",
    "Brazil",
    "Canada",
    "Netherlands",
    "Sweden",
    "Asia",
    "Taiwan",
    "Hong Kong",
    "Russia",
    "Scandinavia",
    "Denmark",
    "Finland",
    "Norway",
    "Poland",
    "Portugal",
    "Greece",
    "Belgium",
    "Austria",
    "Switzerland",
    "Mexico",
    "Argentina",
    "India",
    "Israel",
    "Croatia",
    "New Zealand",
    "Ireland",
    "Latin America",
    "South Africa",
    "Turkey",
    "Unknown",
    "US",
    "EU",
    "JP",
];

/// Splits a name into title and tags. Never fails; unknown tags are ignored.
pub fn parse(name: &str) -> NameInfo<'_> {
    let mut info = NameInfo {
        title: title_of(name),
        ..NameInfo::default()
    };
    for (bracket, tag) in tags(name) {
        let t = tag.trim();
        if bracket {
            parse_bracket(t, &mut info.flags);
        } else {
            parse_paren(t, &mut info);
        }
    }
    if name.starts_with("[BIOS]") {
        info.flags.bios = true;
    }
    info
}

fn title_of(name: &str) -> &str {
    let rest = name.strip_prefix("[BIOS]").unwrap_or(name).trim_start();
    let end = rest.find(['(', '[']).unwrap_or(rest.len());
    rest[..end].trim_end()
}

/// All `(...)` / `[...]` groups after the title; `true` for brackets.
fn tags(name: &str) -> impl Iterator<Item = (bool, &str)> {
    let mut rest = &name[name.len() - name.trim_start_matches("[BIOS]").len()..];
    std::iter::from_fn(move || {
        let start = rest.find(['(', '['])?;
        let bracket = rest.as_bytes()[start] == b'[';
        let close = if bracket { ']' } else { ')' };
        let body = &rest[start + 1..];
        let end = body.find(close).unwrap_or(body.len());
        rest = body.get(end + 1..).unwrap_or("");
        Some((bracket, &body[..end]))
    })
}

fn parse_paren<'a>(t: &'a str, info: &mut NameInfo<'a>) {
    let f = &mut info.flags;
    let parts: Vec<&str> = t.split(',').map(str::trim).collect();
    if info.regions.is_empty() && parts.iter().all(|p| REGIONS.contains(p)) {
        info.regions = parts;
        return;
    }
    if info.languages.is_empty() && parts.iter().all(|p| is_lang(p)) {
        info.languages = parts
            .iter()
            .map(|p| p.split('-').next().unwrap_or(p))
            .collect();
        return;
    }
    let lower = t.to_ascii_lowercase();
    let word = lower.split([' ', '-']).next().unwrap_or("");
    match word {
        "beta" => f.beta = true,
        "proto" | "prototype" => f.proto = true,
        "demo" | "taikenban" | "trial" | "otameshi" | "tentou" => f.demo = true,
        "kiosk" => f.kiosk = true,
        "sample" => f.sample = true,
        "unl" => f.unlicensed = true,
        "pirate" | "bootleg" => f.pirate = true,
        "aftermarket" | "homebrew" => f.aftermarket = true,
        "alt" => f.alt = true,
        "hack" => f.hack = true,
        "disc" | "disk" | "side" => info.disc = lower.split([' ', '-']).nth(1).and_then(media_no),
        "rev" => info.revision = rev_rank(&lower[3..]),
        _ if lower.starts_with("virtual console") => f.virtual_console = true,
        _ if is_rerelease(&lower) => f.rerelease = true,
        _ if is_version(&lower) => info.revision = version_rank(&lower[1..]),
        _ => {}
    }
}

/// `[b]`, `[h2]`, `[a]`, `[T-En by X]`, `[tr es]`, `[!]`, `[BIOS]`.
fn parse_bracket(t: &str, f: &mut Flags) {
    let lower = t.to_ascii_lowercase();
    if lower == "bios" {
        f.bios = true;
    } else if is_rerelease(&lower) {
        f.rerelease = true;
    } else if lower.starts_with("t-") || lower.starts_with("t+") || lower.starts_with("tr") {
        f.translation = true;
    } else if let Some(rest) = lower.strip_prefix(['b', 'h', 'a', 'p', 'o', 'f']) {
        if !rest.is_empty() && !rest.starts_with(|c: char| c.is_ascii_digit() || c == ' ') {
            return;
        }
        match lower.as_bytes()[0] {
            b'b' | b'o' => f.bad_dump = true,
            b'h' | b'f' => f.hack = true,
            b'p' => f.pirate = true,
            _ => f.alt = true,
        }
    }
}

/// Official re-releases of the same game: digital, budget lines, reprints, collection extracts.
fn is_rerelease(lower: &str) -> bool {
    const WORDS: &[&str] = &[
        "digital",
        "switch online",
        "collection",
        "reprint",
        "major wave",
        "the best",
        "psone books",
        "greatest hits",
        "platinum",
        "player's choice",
        "classics",
        "satsuki",
        "best price",
        "budget",
    ];
    WORDS.iter().any(|w| lower.contains(w))
}

fn is_lang(p: &str) -> bool {
    let b = p.as_bytes();
    (b.len() == 2 || (b.len() == 5 && b[2] == b'-'))
        && b[0].is_ascii_uppercase()
        && b[1].is_ascii_lowercase()
}

fn is_version(lower: &str) -> bool {
    lower.len() > 1
        && lower.starts_with('v')
        && lower[1..].bytes().all(|c| c.is_ascii_digit() || c == b'.')
}

fn version_rank(v: &str) -> u32 {
    let mut it = v.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    let major = it.next().unwrap_or(0);
    let minor = it.next().unwrap_or(0);
    major * 1000 + minor.min(999)
}

/// `1`, `A`, `-F` → rank (letters count from A = 1).
fn rev_rank(s: &str) -> u32 {
    let s = s.trim_start_matches([' ', '-']).trim();
    s.parse().unwrap_or_else(|_| match s.as_bytes() {
        [c] if c.is_ascii_alphabetic() => u32::from(c.to_ascii_uppercase() - b'A' + 1),
        _ => 0,
    })
}

fn media_no(s: &str) -> Option<u32> {
    s.parse().ok().or_else(|| match s.as_bytes() {
        [c] if c.is_ascii_alphabetic() => Some(u32::from(c.to_ascii_uppercase() - b'A' + 1)),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_languages_revision() {
        let i = parse("Super Mario World (USA, Europe) (En,Fr,De) (Rev 1)");
        assert_eq!(i.title, "Super Mario World");
        assert_eq!(i.regions, ["USA", "Europe"]);
        assert_eq!(i.languages, ["En", "Fr", "De"]);
        assert_eq!(i.revision, 1);
        assert_eq!(i.flags, Flags::default());
    }

    #[test]
    fn flags_and_brackets() {
        let i = parse("Legend of Zelda, The - A Link to the Past (France) (Beta 2)");
        assert_eq!(i.title, "Legend of Zelda, The - A Link to the Past");
        assert!(i.flags.beta && i.flags.is_unofficial());
        assert!(parse("Zelda (USA)[h2]").flags.hack);
        assert!(parse("Zelda (USA)[tr es](Alt 1)").flags.translation);
        assert!(parse("Zelda (USA)[tr es](Alt 1)").flags.alt);
        assert!(parse("Zelda (USA) [T-En by X v1.0]").flags.translation);
        assert!(parse("Zelda (USA) [b]").flags.bad_dump);
        assert!(!parse("Zelda (USA) [!]").flags.is_unofficial());
        assert!(parse("[BIOS] PlayStation (Europe) (v3.0)").flags.bios);
        assert_eq!(
            parse("[BIOS] PlayStation (Europe) (v3.0)").title,
            "PlayStation"
        );
        assert!(
            parse("Mario (Japan) (Virtual Console)")
                .flags
                .virtual_console
        );
        assert!(parse("Zelda (World) (REV-F) (Digital)").flags.rerelease);
        assert!(parse("Game (USA) (Unl)").flags.unlicensed);
        assert!(parse("Afraid Gear [Reprint] (Japan)").flags.rerelease);
        assert!(parse("Acid (Japan) (Major Wave)").flags.rerelease);
        assert!(parse("Addie (Japan) (Otameshi-ban)").flags.demo);
    }

    #[test]
    fn revisions_and_discs() {
        assert_eq!(parse("Zelda (World) (REV-F)").revision, 6);
        assert_eq!(parse("Zelda (USA) (Rev A)").revision, 1);
        assert_eq!(parse("Game (Europe) (v1.12)").revision, 1012);
        assert_eq!(parse("FF7 (USA) (Disc 2)").disc, Some(2));
        assert_eq!(parse("Game (Disk B)").disc, Some(2));
        assert_eq!(parse("Game (USA)").disc, None);
        assert_eq!(parse("No tags").title, "No tags");
    }
}
