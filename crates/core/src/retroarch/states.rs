//! Savestates RetroArch wrote for a game: `<states>/<core name>/<content>.state[N|.auto]`
//! (with `sort_savestates_enable`), each with an optional `<state>.png` screenshot.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveState {
    pub path: PathBuf,
    /// Core folder (`corename`), empty for states directly in the states folder.
    pub core: String,
    /// Slot number; `None` for the auto state (`.state.auto`).
    pub slot: Option<u32>,
    pub modified: SystemTime,
    pub screenshot: Option<PathBuf>,
}

/// Name RetroArch derives savestates from: the content file name without extension
/// (archives: the archive's name).
pub fn content_stem(rom: &Path) -> Option<&str> {
    rom.file_stem()?.to_str()
}

fn slot_of(file: &str, stem: &str) -> Option<Option<u32>> {
    let rest = file.strip_prefix(stem)?.strip_prefix(".state")?;
    match rest {
        "" => Some(Some(0)),
        ".auto" => Some(None),
        n if n.bytes().all(|b| b.is_ascii_digit()) => n.parse().ok().map(Some),
        _ => None,
    }
}

fn scan(dir: &Path, core: &str, stem: &str, out: &mut Vec<SaveState>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let path = e.path();
        let Ok(meta) = e.metadata() else { continue };
        if meta.is_dir() {
            if core.is_empty()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
            {
                scan(&path, name, stem, out);
            }
            continue;
        }
        let Some(file) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(slot) = slot_of(file, stem) else {
            continue;
        };
        let png = path.with_file_name(format!("{file}.png"));
        out.push(SaveState {
            core: core.to_owned(),
            slot,
            modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            screenshot: png.is_file().then_some(png),
            path,
        });
    }
}

/// All savestates of `rom` in `states_dir` (all core folders), newest first.
pub fn list(states_dir: &Path, rom: &Path) -> Vec<SaveState> {
    let mut out = Vec::new();
    if let Some(stem) = content_stem(rom) {
        scan(states_dir, "", stem, &mut out);
    }
    out.sort_by_key(|s| std::cmp::Reverse(s.modified));
    out
}

/// Deletes a state and its screenshot. Refuses paths outside `states_dir`.
pub fn delete(states_dir: &Path, state: &Path) -> std::io::Result<()> {
    if !state.starts_with(states_dir) || state.extension().is_some_and(|e| e == "png") {
        return Err(std::io::Error::other("not a savestate"));
    }
    std::fs::remove_file(state)?;
    let mut png = state.as_os_str().to_owned();
    png.push(".png");
    match std::fs::remove_file(PathBuf::from(png)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_and_deletes() {
        let dir = std::env::temp_dir().join(format!("rb-states-{}", std::process::id()));
        let snes = dir.join("Snes9x");
        std::fs::create_dir_all(&snes).unwrap();
        for f in [
            "Mario (USA).state",
            "Mario (USA).state3",
            "Mario (USA).state3.png",
            "Mario (USA).state.auto",
            "Mario (USA) (Beta).state",
            "Mario (USA).srm",
        ] {
            std::fs::write(snes.join(f), b"x").unwrap();
        }
        std::fs::write(dir.join("Mario (USA).state1"), b"x").unwrap();
        let rom = Path::new("/lib/SNES/Mario (USA).zip");
        let mut l = list(&dir, rom);
        l.sort_by_key(|s| (s.core.clone(), s.slot));
        let got: Vec<_> = l
            .iter()
            .map(|s| (s.core.as_str(), s.slot, s.screenshot.is_some()))
            .collect();
        assert_eq!(
            got,
            [
                ("", Some(1), false),
                ("Snes9x", None, false),
                ("Snes9x", Some(0), false),
                ("Snes9x", Some(3), true)
            ]
        );
        delete(&dir, &snes.join("Mario (USA).state3")).unwrap();
        assert!(!snes.join("Mario (USA).state3.png").exists());
        assert!(delete(&dir, Path::new("/etc/passwd")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
