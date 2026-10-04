//! Parsers for disc sheets: `.cue`, `.gdi` and `.m3u` playlists.

use std::path::{Path, PathBuf};

/// Track files referenced by a `.cue` sheet, in order, deduplicated.
/// Paths are resolved relative to the sheet's directory.
pub fn parse_cue(text: &str, dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = strip_keyword(line, "FILE") else {
            continue;
        };
        let name = if let Some(q) = rest.strip_prefix('"') {
            q.split('"').next().unwrap_or_default()
        } else {
            // Unquoted: everything but the trailing file type (BINARY, WAVE, ...).
            rest.rsplit_once(char::is_whitespace)
                .map_or(rest, |(n, _)| n)
                .trim()
        };
        if name.is_empty() {
            continue;
        }
        let p = dir.join(name);
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

/// Track files of a `.gdi` sheet: first line is the track count, then
/// `<no> <lba> <type> <sector size> <file> <offset>` per track (file may be quoted).
pub fn parse_gdi(text: &str, dir: &Path) -> Vec<PathBuf> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            for _ in 0..4 {
                it.next()?;
            }
            let rest: Vec<&str> = it.collect();
            let rest = rest.join(" ");
            let name = if let Some(q) = rest.strip_prefix('"') {
                q.split('"').next()?.to_owned()
            } else {
                rest.split_whitespace().next()?.to_owned()
            };
            Some(dir.join(name))
        })
        .collect()
}

/// Entries of an `.m3u` playlist (comments and blank lines skipped).
pub fn parse_m3u(text: &str, dir: &Path) -> Vec<PathBuf> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| dir.join(l))
        .collect()
}

fn strip_keyword<'a>(line: &'a str, kw: &str) -> Option<&'a str> {
    let head = line.get(..kw.len())?;
    let rest = &line[kw.len()..];
    (head.eq_ignore_ascii_case(kw) && rest.starts_with(char::is_whitespace)).then(|| rest.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cue_quoted_and_unquoted() {
        let cue = "FILE \"Game (Track 1).bin\" BINARY\r\n  TRACK 01 MODE2/2352\r\n    INDEX 01 00:00:00\r\n\
                   FILE track2.bin BINARY\n  TRACK 02 AUDIO\nFILE \"Game (Track 1).bin\" BINARY\n";
        let d = Path::new("/d");
        assert_eq!(
            parse_cue(cue, d),
            vec![d.join("Game (Track 1).bin"), d.join("track2.bin")]
        );
    }

    #[test]
    fn gdi_tracks() {
        let gdi = "3\n1 0 4 2352 track01.bin 0\n2 600 0 2352 \"track 02.raw\" 0\n3 45000 4 2352 track03.bin 0\n";
        let d = Path::new("/d");
        assert_eq!(
            parse_gdi(gdi, d),
            vec![
                d.join("track01.bin"),
                d.join("track 02.raw"),
                d.join("track03.bin")
            ]
        );
    }

    #[test]
    fn m3u_entries() {
        let d = Path::new("/d");
        assert_eq!(
            parse_m3u("#EXTM3U\nA (Disc 1).cue\n\nA (Disc 2).cue\n", d),
            vec![d.join("A (Disc 1).cue"), d.join("A (Disc 2).cue")]
        );
    }
}
