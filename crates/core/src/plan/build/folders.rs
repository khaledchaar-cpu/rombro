//! Game folders (DOS, ScummVM, ports): the databases identify a game by one key file, but the
//! game is the whole folder around it. The folder moves as a unit, structure untouched.

use super::Builder;
use crate::naming;
use crate::plan::{Files, Game, Ident, Item, lpl};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Systems whose matches are key files inside a game folder.
const FOLDER_SYSTEMS: [&str; 12] = [
    "DOS",
    "ScummVM",
    "DOOM",
    "Quake",
    "Quake II",
    "Quake III",
    "Cannonball",
    "Rick Dangerous",
    "MrBoom",
    "Dinothawr",
    "Flashback",
    "Wolfenstein 3D",
];

/// Folder suffixes frontends use for game folders (`Abuse.dos/`); such a folder is the game.
const MARKERS: [&str; 6] = ["dos", "pc", "scummvm", "exo", "boom", "wine"];

pub(super) fn is_folder_system(system: &str) -> bool {
    FOLDER_SYSTEMS.contains(&system)
}

/// One game folder and the match that names it.
pub(super) struct FolderGame<'a> {
    pub item: &'a Item,
    pub game: &'a Game,
    pub key: &'a Path,
}

/// Game folders keyed by their root directory. Matches lying loose in a folder shared by
/// several games (or in the inbox root) get no folder and are placed as single files.
pub(super) fn find<'a>(
    items: &[&'a Item],
    library: &Path,
    inbox: Option<&Path>,
) -> BTreeMap<PathBuf, FolderGame<'a>> {
    let matched: Vec<FolderGame<'a>> = items
        .iter()
        .filter_map(|it| match (&it.ident, &it.files) {
            (Ident::Known(g), Files::Single(p)) if is_folder_system(&g.system) => {
                Some(FolderGame {
                    item: it,
                    game: g,
                    key: p.as_path(),
                })
            }
            _ => None,
        })
        .collect();
    let base = |p: &Path| -> Option<PathBuf> {
        [Some(library), inbox]
            .into_iter()
            .flatten()
            .filter(|b| p.starts_with(b))
            .max_by_key(|b| b.as_os_str().len())
            .map(Path::to_path_buf)
    };
    // directory -> its child directories that contain matches
    let mut children: HashMap<PathBuf, HashSet<OsString>> = HashMap::new();
    for m in &matched {
        let Some(b) = base(m.key) else { continue };
        for a in m.key.ancestors().skip(1).take_while(|a| *a != b) {
            if let (Some(parent), Some(name)) = (a.parent(), a.file_name()) {
                children
                    .entry(parent.to_path_buf())
                    .or_default()
                    .insert(name.to_owned());
            }
        }
    }
    let shared = |d: &Path| children.get(d).is_some_and(|c| c.len() >= 2);

    let mut out: BTreeMap<PathBuf, FolderGame<'a>> = BTreeMap::new();
    for m in matched {
        let Some(b) = base(m.key) else { continue };
        let Some(root) = root_of(m.key, &b, b == library, &shared) else {
            continue;
        };
        // several key files in one folder: the shallowest (then first by name) names the game
        let rank = |k: &Path| (k.components().count(), k.to_path_buf());
        match out.get(&root) {
            Some(cur) if rank(cur.key) <= rank(m.key) => {}
            _ => {
                out.insert(root, m);
            }
        }
    }
    out
}

fn root_of(
    key: &Path,
    base: &Path,
    in_library: bool,
    shared: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let rel = key.strip_prefix(base).ok()?;
    if in_library {
        // already placed: `<lib>/<System>/<Game>/…`
        let mut c = rel.components();
        let (sys, game) = (c.next()?, c.next()?);
        c.next()?;
        return Some(base.join(sys).join(game));
    }
    let marked = key
        .ancestors()
        .skip(1)
        .take_while(|a| *a != base)
        .filter(|a| {
            a.extension()
                .is_some_and(|e| MARKERS.iter().any(|m| e.eq_ignore_ascii_case(m)))
        })
        .last();
    if let Some(m) = marked {
        return Some(m.to_path_buf());
    }
    let mut cur = key.parent()?;
    if cur == base || shared(cur) {
        return None;
    }
    while let Some(p) = cur.parent() {
        if p == base || shared(p) {
            break;
        }
        cur = p;
    }
    Some(cur.to_path_buf())
}

impl Builder<'_> {
    /// Moves a game folder to `<System>/<Game>/`, keeping everything inside as it is.
    pub(super) fn folder_game(&mut self, root: &Path, fg: &FolderGame<'_>) {
        let g = fg.game;
        let target = self
            .library
            .join(naming::sanitize_file_name(&g.system))
            .join(naming::sanitize_file_name(&naming::release_name(&g.name)));
        let key_rel = fg.key.strip_prefix(root).unwrap_or(fg.key);
        self.lpl
            .entry(g.system.clone())
            .or_default()
            .push(lpl::Entry {
                path: target.join(key_rel),
                label: naming::release_name(&g.name),
                crc: g.crc,
            });
        if root == target {
            self.plan.unchanged += 1;
            return;
        }
        let it = Item {
            files: Files::Single(root.to_path_buf()),
            ident: fg.item.ident.clone(),
            in_library: fg.item.in_library,
        };
        let ops = walkdir::WalkDir::new(root)
            .sort_by_file_name()
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                let rel = e.path().strip_prefix(root).unwrap_or(e.path());
                self.transfer(&it, e.path(), &target.join(rel))
            })
            .collect();
        self.why = "game folder".into();
        if self.commit(&it, ops) {
            self.plan.placed += 1;
        }
    }
}
