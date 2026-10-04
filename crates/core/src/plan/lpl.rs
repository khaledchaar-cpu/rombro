//! RetroArch playlist (`.lpl`, JSON format v1.5) export.

use serde_json::json;
use std::path::PathBuf;

/// One playlist entry: content path, display label, CRC32 of the content if known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub label: String,
    pub crc: Option<u32>,
}

/// Renders a playlist for `system`; entries are sorted by label.
pub fn render(system: &str, entries: &[Entry]) -> String {
    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by(|a, b| a.label.cmp(&b.label));
    let db_name = format!("{system}.lpl");
    let items: Vec<_> = sorted
        .iter()
        .map(|e| {
            json!({
                "path": e.path.to_string_lossy(),
                "label": e.label,
                "core_path": "DETECT",
                "core_name": "DETECT",
                "crc32": e.crc.map_or_else(|| "00000000|crc".to_owned(), |c| format!("{c:08X}|crc")),
                "db_name": db_name,
            })
        })
        .collect();
    let doc = json!({
        "version": "1.5",
        "default_core_path": "",
        "default_core_name": "",
        "label_display_mode": 0,
        "right_thumbnail_mode": 0,
        "left_thumbnail_mode": 0,
        "sort_mode": 0,
        "items": items,
    });
    let mut s = serde_json::to_string_pretty(&doc).unwrap_or_default();
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_sorted_entries_with_crc() {
        let e = |label: &str, crc| Entry {
            path: PathBuf::from(format!("/lib/{label}.sfc")),
            label: label.into(),
            crc,
        };
        let s = render("Nintendo - SNES", &[e("B", None), e("A", Some(0xab))]);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["items"][0]["label"], "A");
        assert_eq!(v["items"][0]["crc32"], "000000AB|crc");
        assert_eq!(v["items"][1]["crc32"], "00000000|crc");
        assert_eq!(v["items"][0]["db_name"], "Nintendo - SNES.lpl");
    }
}
