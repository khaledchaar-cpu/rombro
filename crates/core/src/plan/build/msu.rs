//! MSU-1 games: a patched SNES ROM with CD-quality audio tracks (`<rom>.msu`, `<rom>-N.pcm`)
//! in one folder. No database lists them, so the `.msu` file marks the game; the folder
//! moves as a whole like other game folders, and chip dumps inside (`cx4.data.rom`) stay with it.

use super::folders::FolderGame;
use crate::plan::{Files, Item};
use crate::rules::Rule;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Library folder (and RetroArch playlist) of MSU-1 games.
pub const SYSTEM: &str = "Nintendo - Super Nintendo Entertainment System (MSU-1)";

const ROMS: [&str; 2] = ["sfc", "smc"];

fn has_ext(p: &Path, exts: &[&str]) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| exts.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Whether `files` hold audio tracks `<rom stem>-<n>.pcm` for `rom`.
fn has_tracks(files: &[PathBuf], rom: &Path) -> bool {
    let prefix = format!("{}-", stem(rom));
    files.iter().any(|f| {
        has_ext(f, &["pcm"])
            && stem(f)
                .strip_prefix(&prefix)
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// The `.msu` marker a game folder lacks (relative to it): cores enable MSU-1 only with it.
pub(super) fn missing_marker(root: &Path, key: &Path) -> Option<PathBuf> {
    let rel = key.strip_prefix(root).ok()?.with_extension("msu");
    (!root.join(&rel).exists()).then_some(rel)
}

/// MSU-1 game folders keyed by their directory: exactly one `.msu` file next to a SNES ROM
/// (the one named like it, else the only one), or a ROM with its `-N.pcm` audio tracks.
/// Never the inbox or library root itself.
pub(super) fn find<'a>(items: &[&'a Item], roots: &[&Path]) -> BTreeMap<PathBuf, FolderGame<'a>> {
    let mut out = BTreeMap::new();
    let mut seen = std::collections::HashSet::new();
    for &it in items {
        // the `.msu` file is often empty (a marker) and then not scanned: any file of the
        // folder leads to it
        let Files::Single(f) = &it.files else {
            continue;
        };
        // an MSU-1 folder always holds a scanned SNES ROM: other folders are not read
        if !has_ext(f, &ROMS) {
            continue;
        }
        let Some(dir) = f.parent() else { continue };
        if roots.contains(&dir) || !seen.insert(dir) {
            continue;
        }
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
        let files: Vec<PathBuf> = rd
            .flatten()
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .map(|e| e.path())
            .collect();
        let mut msus = files.iter().filter(|p| has_ext(p, &["msu"]));
        let msu = match (msus.next(), msus.next()) {
            (Some(m), None) => Some(m),
            (None, _) => None,
            _ => continue,
        };
        let roms: Vec<&PathBuf> = files.iter().filter(|p| has_ext(p, &ROMS)).collect();
        let key = match msu {
            Some(m) => roms
                .iter()
                .find(|r| r.file_stem() == m.file_stem())
                .or_else(|| (roms.len() == 1).then(|| &roms[0])),
            // marker missing: the audio tracks (`<rom>-N.pcm`) name the ROM
            None => roms.iter().find(|r| has_tracks(&files, r)),
        }
        .map(|r| (*r).clone());
        let (Some(key), Some(name)) = (key, dir.file_name()) else {
            continue;
        };
        out.insert(
            dir.to_path_buf(),
            FolderGame {
                item: it,
                system: SYSTEM,
                name: name.to_string_lossy().into_owned(),
                key,
                crc: None,
                rule: Rule::Msu1,
            },
        );
    }
    out
}
