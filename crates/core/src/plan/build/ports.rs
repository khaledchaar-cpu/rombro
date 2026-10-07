//! Game ports with a libretro core but no RetroArch database: recognized by the file the
//! core loads (its `.info` `supported_extensions`), placed as game folders. Ports without a
//! libretro core are not RetroArch games and stay unknown (inbox leftovers → trash).

use crate::plan::{Files, Game, Ident, Item};

/// (system = game, file the core loads). The core per system is in `recommended.tsv`.
pub const PORTS: [(&str, &str); 1] = [
    // superbroswar_libretro: `supported_extensions = "game"`, content `smw.game`
    ("Super Mario War", "smw.game"),
];

/// `items` with port key files turned into [`Ident::Known`]. The name decides, whatever the
/// hash matched: key files are often empty, and the empty-file hash is in some databases.
pub fn apply(items: Vec<Item>) -> Vec<Item> {
    items
        .into_iter()
        .map(|mut it| {
            if let Files::Single(p) = &it.files
                && let Some(name) = p.file_name().and_then(|n| n.to_str())
                && let Some((system, _)) =
                    PORTS.iter().find(|(_, key)| name.eq_ignore_ascii_case(key))
            {
                it.ident = Ident::Known(Game {
                    system: (*system).to_owned(),
                    name: (*system).to_owned(),
                    crc: None,
                });
            }
            it
        })
        .collect()
}
