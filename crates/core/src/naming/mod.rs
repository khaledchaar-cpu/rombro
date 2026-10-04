//! Names: tag parsing, title normalization for grouping, file-name sanitizing.

pub mod tags;

pub use tags::{Flags, NameInfo, parse};

use std::path::PathBuf;

const ARTICLES: &[&str] = &[
    "the", "a", "an", "der", "die", "das", "le", "la", "les", "l'", "el", "los", "las", "il", "lo",
];

/// Grouping key for a title: case, punctuation and articles (leading or `, The`) unified.
/// `Legend of Zelda, The - A Link to the Past` and `The Legend of Zelda: A Link to the Past`
/// yield the same key.
pub fn group_key(title: &str) -> String {
    let lower = title.to_lowercase().replace('&', " and ");
    let mut out = String::with_capacity(lower.len());
    for seg in lower.split([':', '-', '~']) {
        let mut seg = seg.trim();
        if let Some((head, tail)) = seg.rsplit_once(", ")
            && ARTICLES.contains(&tail.trim())
        {
            seg = head;
        }
        if let Some((first, rest)) = seg.split_once(' ')
            && ARTICLES.contains(&first)
        {
            seg = rest;
        }
        for c in seg.chars() {
            if c.is_alphanumeric() {
                out.push(c);
            } else if !out.ends_with(' ') && !out.is_empty() && c != '\'' && c != '.' {
                out.push(' ');
            }
        }
        if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
    }
    out.truncate(out.trim_end().len());
    out
}

/// File-system-safe name, compatible with RetroArch thumbnail naming (`&*/:<>?\|` → `_`).
pub fn sanitize_file_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '&' | '*' | '/' | ':' | '<' | '>' | '?' | '\\' | '|' | '"' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

/// Release name without its `(Disc N)` / `(Disk N)` / `(Side X)` tag.
pub fn release_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut rest = name;
    while let Some(i) = rest.find('(') {
        let end = rest[i..].find(')').map_or(rest.len(), |e| i + e + 1);
        let tag = rest[i + 1..end.saturating_sub(1).max(i + 1)].to_ascii_lowercase();
        out.push_str(&rest[..i]);
        if !["disc ", "disk ", "side "]
            .iter()
            .any(|p| tag.starts_with(p))
        {
            out.push_str(&rest[i..end]);
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Library path (relative to the library root) for one file of a release, per SPEC F4:
/// `<System>/<Name>.<ext>`, or for multi-disc sets `<System>/<Release>/<Name>.<ext>`.
pub fn target_path(system: &str, name: &str, ext: &str, multi_disc: bool) -> PathBuf {
    let mut p = PathBuf::from(sanitize_file_name(system));
    if multi_disc {
        p.push(sanitize_file_name(&release_name(name)));
    }
    p.push(format!("{}.{ext}", sanitize_file_name(name)));
    p
}

/// `.m3u` playlist path for a multi-disc release: `<System>/<Release>/<Release>.m3u`.
pub fn playlist_path(system: &str, name: &str) -> PathBuf {
    let rel = sanitize_file_name(&release_name(name));
    PathBuf::from(sanitize_file_name(system))
        .join(&rel)
        .join(format!("{rel}.m3u"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_keys_unify_articles_and_punctuation() {
        let k = group_key("Legend of Zelda, The - A Link to the Past");
        assert_eq!(k, "legend of zelda link to the past");
        assert_eq!(group_key("The Legend of Zelda: A Link to the Past"), k);
        assert_eq!(group_key("Super Mario World"), "super mario world");
        assert_eq!(group_key("Tom & Jerry"), group_key("Tom and Jerry"));
        assert_eq!(group_key("Dr. Mario"), group_key("Dr Mario"));
        assert_eq!(group_key("Pokemon - Blue Version"), "pokemon blue version");
        assert_ne!(
            group_key("Street Fighter II"),
            group_key("Street Fighter III")
        );
    }

    #[test]
    fn release_names_and_paths() {
        assert_eq!(release_name("FF7 (USA) (Disc 2)"), "FF7 (USA)");
        assert_eq!(release_name("Game (Disk B) (Europe)"), "Game (Europe)");
        assert_eq!(release_name("Game (Europe)"), "Game (Europe)");
        assert_eq!(
            target_path("Sony - PlayStation", "FF7 (USA) (Disc 2)", "chd", true),
            PathBuf::from("Sony - PlayStation/FF7 (USA)/FF7 (USA) (Disc 2).chd")
        );
        assert_eq!(
            playlist_path("Sony - PlayStation", "FF7 (USA) (Disc 2)"),
            PathBuf::from("Sony - PlayStation/FF7 (USA)/FF7 (USA).m3u")
        );
        assert_eq!(
            target_path("Nintendo - SNES", "Tom & Jerry (USA)", "zip", false),
            PathBuf::from("Nintendo - SNES/Tom _ Jerry (USA).zip")
        );
    }

    #[test]
    fn sanitizes() {
        assert_eq!(sanitize_file_name("Tom & Jerry: A/B?"), "Tom _ Jerry_ A_B_");
    }
}
