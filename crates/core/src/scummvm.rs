//! ScummVM launcher files: `<id>.scummvm` holds the game id the core starts (`monkey2`), and
//! the database identifies the game by exactly that content. Frontends or editors sometimes
//! leave them empty or filled with junk (e.g. an empty RTF document); the id in the file name
//! then repairs them.

use std::path::Path;

/// Launcher files larger than this are not plain ids.
const MAX: u64 = 4096;

/// Whether `s` is a ScummVM game id (optionally engine-prefixed: `scumm:monkey2`).
fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b':' | b'.'))
}

/// The id a broken launcher file should hold (its file stem), or `None` if the file is fine,
/// not a launcher, or its name is no id either.
pub fn repaired_id(path: &Path, content: &[u8]) -> Option<String> {
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("scummvm"))
    {
        return None;
    }
    if std::str::from_utf8(content).is_ok_and(|s| is_id(s.trim())) {
        return None;
    }
    let stem = path.file_stem()?.to_str()?.to_ascii_lowercase();
    is_id(&stem).then_some(stem)
}

/// [`repaired_id`] for a file on disk.
pub fn repaired_id_of(path: &Path) -> Option<String> {
    let len = std::fs::metadata(path).ok()?.len();
    if len > MAX {
        return None;
    }
    repaired_id(path, &std::fs::read(path).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_junk_launchers_from_their_name() {
        let p = Path::new("Monkey 2.scummvm/monkey2.scummvm");
        assert_eq!(repaired_id(p, b"{\\rtf1\\ansi}\n"), Some("monkey2".into()));
        assert_eq!(repaired_id(p, b""), Some("monkey2".into()));
        assert_eq!(repaired_id(p, b"monkey2\n"), None);
        assert_eq!(repaired_id(p, b"scumm:monkey2"), None);
        assert_eq!(repaired_id(Path::new("x/MONKEY2.EXE"), b"junk junk"), None);
        assert_eq!(repaired_id(Path::new("x/my game.scummvm"), b"{}"), None);
    }
}
