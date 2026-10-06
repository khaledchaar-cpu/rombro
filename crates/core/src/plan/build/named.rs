//! Name-only identification (SPEC §11a): unknown files directly in a folder the rules map to
//! a system (`Rules::name_folders`, e.g. Batocera's `n64dd` → Nintendo 64 for cartridge
//! conversions) become [`Ident::Named`] with the file name as release name. In the library the
//! system folder itself counts, so an audit keeps what an import placed.

use crate::g1r::Rules;
use crate::naming;
use crate::plan::{Files, Game, Ident, Item};
use std::path::Path;

/// Extensions that are never games (notes, frontend files, saves, media).
const NOT_GAMES: [&str; 22] = [
    "txt", "xml", "old", "bak", "nfo", "md", "pdf", "cfg", "ini", "json", "srm", "sav", "state",
    "png", "jpg", "jpeg", "gif", "webp", "mp4", "mkv", "avi", "webm",
];

/// `items` with unknown files in name folders turned into [`Ident::Named`].
pub fn apply(items: &[Item], rules: &Rules) -> Vec<Item> {
    items
        .iter()
        .map(|it| {
            let named = match (&it.ident, &it.files) {
                (Ident::Unknown, Files::Single(p)) if !rules.name_folders.is_empty() => {
                    system_for(p, rules)
                }
                _ => None,
            };
            match named {
                Some(system) => Item {
                    files: it.files.clone(),
                    ident: Ident::Named(Game {
                        system,
                        name: stem(it.files.primary()),
                        crc: None,
                    }),
                    in_library: it.in_library,
                },
                None => it.clone(),
            }
        })
        .collect()
}

fn system_for(p: &Path, rules: &Rules) -> Option<String> {
    let ext = p.extension()?.to_string_lossy().to_lowercase();
    if NOT_GAMES.contains(&ext.as_str()) {
        return None;
    }
    let dir = p.parent()?.file_name()?.to_string_lossy();
    rules
        .name_folders
        .iter()
        .find(|(folder, system)| {
            dir.eq_ignore_ascii_case(folder)
                || dir.eq_ignore_ascii_case(&naming::sanitize_file_name(system))
        })
        .map(|(_, system)| system.clone())
}

fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}
