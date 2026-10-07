//! Daphne laserdisc games: no database lists them, and a collection only works as a whole.
//! The libretro core loads `roms/<game>.zip` and reads video and framefile from the sibling
//! `<game>.daphne/` folder, so a folder holding `roms/` and `*.daphne/` moves as a unit to
//! `Daphne/`, structure untouched; each game with a `.daphne` folder gets a playlist entry.

use super::Builder;
use crate::plan::{Files, Game, Ident, Item, lpl};
use crate::rules::{Rule, Why};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Library folder (and RetroArch playlist) of Daphne games.
pub const SYSTEM: &str = "Daphne";

const EXT: &str = "daphne";

/// Whether `dir` is a Daphne collection: a `roms/` folder next to at least one `*.daphne/`.
fn is_collection(dir: &Path) -> bool {
    dir.join("roms").is_dir() && !games(dir).is_empty()
}

/// Names of the `<game>.daphne/` folders in `dir`.
fn games(dir: &Path) -> BTreeSet<String> {
    let Ok(rd) = dir.read_dir() else {
        return BTreeSet::new();
    };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case(EXT)))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect()
}

/// Collection roots among the folders of `items` within `roots` (the inbox itself may be
/// one, the library root not).
pub(super) fn find(items: &[&Item], roots: &[&Path], library: &Path) -> BTreeSet<PathBuf> {
    let mut seen: BTreeSet<&Path> = BTreeSet::new();
    let mut found = BTreeSet::new();
    for it in items {
        let p = it.files.archive().unwrap_or(it.files.primary());
        for dir in p.ancestors().skip(1) {
            if dir == library || !roots.iter().any(|r| dir.starts_with(r)) {
                break;
            }
            if !seen.insert(dir) {
                break;
            }
            if is_collection(dir) {
                found.insert(dir.to_path_buf());
                break;
            }
        }
    }
    found
}

impl Builder<'_> {
    /// Moves a Daphne collection to `Daphne/` file by file (files already there count as
    /// duplicates) and lists each game whose ROM zip and `.daphne` folder are both present.
    pub(super) fn daphne(&mut self, root: &Path) {
        let target = self.library.join(SYSTEM);
        for name in games(root) {
            let zip = root.join("roms").join(format!("{name}.zip"));
            if zip.is_file() {
                self.lpl
                    .entry(SYSTEM.to_owned())
                    .or_default()
                    .push(lpl::Entry {
                        path: target.join("roms").join(format!("{name}.zip")),
                        label: name,
                        crc: None,
                    });
            }
        }
        if root == target {
            self.plan.unchanged += 1;
            return;
        }
        self.why = Why::new(Rule::GameFolder, "Daphne laserdisc collection");
        let files: Vec<PathBuf> = walkdir::WalkDir::new(root)
            .sort_by_file_name()
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .filter(|e| !super::frontend::is_metadata(&self.meta_roots, e.path()))
            .map(walkdir::DirEntry::into_path)
            .collect();
        let mut placed = false;
        for f in files {
            let rel = f.strip_prefix(root).unwrap_or(&f);
            let it = Item {
                files: Files::Single(f.clone()),
                ident: Ident::Known(Game {
                    system: SYSTEM.to_owned(),
                    name: SYSTEM.to_owned(),
                    crc: None,
                }),
                in_library: f.starts_with(self.library),
            };
            let op = self.transfer(&it, &f, &target.join(rel));
            placed |= self.commit(&it, vec![op]);
        }
        if placed {
            self.plan.placed += 1;
        }
    }
}
