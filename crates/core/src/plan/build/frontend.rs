//! Frontend metadata (Batocera / EmulationStation): `gamelist.xml` with its backups, the
//! `_info.txt` / `_readme.txt` / `_lisezmoi.txt` notes and scraped media next to it. RetroArch needs none of it. Only folders holding a
//! `gamelist*.xml` count as frontend folders, so game data that happens to sit in an
//! `images/` folder elsewhere is never touched.

use crate::plan::Item;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

/// Media folders frontends scrape into.
const MEDIA_DIRS: [&str; 9] = [
    "images",
    "videos",
    "media",
    "manuals",
    "downloaded_images",
    "downloaded_videos",
    "downloaded_manuals",
    "marquees",
    "thumbnails",
];
/// Notes Batocera puts in every system folder.
const NOTES: [&str; 3] = ["_info.txt", "_readme.txt", "_lisezmoi.txt"];
const MEDIA_EXT: [&str; 10] = [
    "png", "jpg", "jpeg", "gif", "webp", "mp4", "mkv", "avi", "webm", "pdf",
];

/// Folders with a `gamelist*.xml` among the scanned files.
pub(super) fn roots(items: &[&Item]) -> HashSet<PathBuf> {
    items
        .iter()
        .filter_map(|it| {
            let p = it.files.primary();
            let name = p.file_name()?.to_string_lossy().to_lowercase();
            (name.starts_with("gamelist") && name.contains(".xml"))
                .then(|| p.parent().map(Path::to_path_buf))
                .flatten()
        })
        .collect()
}

/// Whether `path` is frontend metadata of one of `roots`.
pub(super) fn is_metadata(roots: &HashSet<PathBuf>, path: &Path) -> bool {
    path.ancestors().skip(1).any(|root| {
        if !roots.contains(root) {
            return false;
        }
        let Ok(rel) = path.strip_prefix(root) else {
            return false;
        };
        let parts: Vec<String> = rel
            .components()
            .filter_map(|c| match c {
                Component::Normal(s) => Some(s.to_string_lossy().to_lowercase()),
                _ => None,
            })
            .collect();
        // Batocera pad-to-keyboard mappings (`Game.zip.p2k.cfg`) sit next to the games
        if parts.last().is_some_and(|n| n.ends_with(".p2k.cfg")) {
            return true;
        }
        match parts.as_slice() {
            [name] => name.starts_with("gamelist") || NOTES.contains(&name.as_str()),
            [dir, .., name] => {
                MEDIA_DIRS.contains(&dir.as_str())
                    && name
                        .rsplit_once('.')
                        .is_some_and(|(_, e)| MEDIA_EXT.contains(&e))
            }
            [] => false,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_metadata_next_to_a_gamelist() {
        let roots = HashSet::from([PathBuf::from("/lib/Quake/quake")]);
        let yes = [
            "/lib/Quake/quake/gamelist.xml.old",
            "/lib/Quake/quake/gamelist.Missing.Serial.txt",
            "/lib/Quake/quake/_info.txt",
            "/lib/Quake/quake/_lisezmoi.txt",
            "/lib/Quake/quake/Quake.zip.p2k.cfg",
            "/lib/Quake/quake/sub/Quake.dim.p2k.cfg",
            "/lib/Quake/quake/images/Quake-image.png",
            "/lib/Quake/quake/media/wheel/Quake.png",
            "/lib/Quake/quake/videos/Quake-video.mp4",
        ];
        let no = [
            "/lib/Quake/quake/id1/pak0.pak",
            "/lib/Quake/quake/Quake.quake",
            "/lib/Quake/quake/images/readme.txt",
            "/lib/Quake/quake/id1/images/skin.png",
            "/lib/DOS/Game/images/title.png",
        ];
        for p in yes {
            assert!(is_metadata(&roots, Path::new(p)), "{p}");
        }
        for p in no {
            assert!(!is_metadata(&roots, Path::new(p)), "{p}");
        }
    }
}
