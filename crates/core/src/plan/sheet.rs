//! Rewriting track file names inside `.cue`/`.gdi` sheets.

/// Replaces track file names in a sheet (case-insensitive, once per line).
pub(super) fn rewrite_sheet(text: &str, renames: &[(String, String)]) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let lower = line.to_lowercase();
        let hit = renames.iter().find_map(|(old, new)| {
            lower
                .find(&old.to_lowercase())
                .filter(|_| lower.len() == line.len())
                .map(|i| (i, old.len(), new))
        });
        match hit {
            Some((i, len, new)) => {
                out.push_str(&line[..i]);
                out.push_str(new);
                out.push_str(&line[i + len..]);
            }
            None => out.push_str(line),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_cue_file_lines() {
        let cue = "FILE \"game (track 1).BIN\" BINARY\n  TRACK 01 MODE2/2352\nFILE \"Game (Track 2).bin\" BINARY\n";
        let r = [
            ("Game (Track 1).bin".into(), "New (Track 1).bin".into()),
            ("Game (Track 2).bin".into(), "New (Track 2).bin".into()),
        ];
        assert_eq!(
            rewrite_sheet(cue, &r),
            "FILE \"New (Track 1).bin\" BINARY\n  TRACK 01 MODE2/2352\nFILE \"New (Track 2).bin\" BINARY\n"
        );
    }
}
