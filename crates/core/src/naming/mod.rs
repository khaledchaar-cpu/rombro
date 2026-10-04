//! Names: tag parsing, title normalization for grouping, file-name sanitizing.

pub mod tags;

pub use tags::{Flags, NameInfo, parse};

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
    fn sanitizes() {
        assert_eq!(sanitize_file_name("Tom & Jerry: A/B?"), "Tom _ Jerry_ A_B_");
    }
}
