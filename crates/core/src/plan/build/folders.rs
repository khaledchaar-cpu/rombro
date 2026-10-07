//! Game folders (DOS, ScummVM, ports): the databases identify a game by one key file, but the
//! game is the whole folder around it. The folder moves as a unit, structure untouched.

use super::Builder;
use crate::naming;
use crate::plan::{Files, Game, Ident, Item, lpl};
use crate::rules::{Rule, Why};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Systems whose matches are key files inside a game folder (default of
/// [`crate::g1r::Rules::folder_systems`]).
pub const FOLDER_SYSTEMS: [&str; 18] = [
    "DOS",
    "ScummVM",
    "DOOM",
    "Quake",
    "Quake II",
    "Quake III",
    "Cannonball",
    "Cave Story",
    "ChaiLove",
    "Dinothawr",
    "Flashback",
    "Jump 'n Bump",
    "Lutro",
    "MrBoom",
    "Rick Dangerous",
    "RPG Maker",
    "Tomb Raider",
    "Wolfenstein 3D",
];

/// Folder suffixes frontends use for game folders (`Abuse.dos/`); such a folder is the game.
const MARKERS: [&str; 6] = ["dos", "pc", "scummvm", "exo", "boom", "wine"];

/// A database match inside a game folder.
struct Match<'a> {
    item: &'a Item,
    game: &'a Game,
    key: &'a Path,
}

/// One game folder. Key files are often shared between games (`dosbox.bat`, Sierra drivers),
/// so the folder keeps its own name; the system comes from its suffix or the matches.
pub(super) struct FolderGame<'a> {
    pub item: &'a Item,
    pub system: &'a str,
    pub name: String,
    /// The match that best fits the folder name; the playlist entry points to it.
    pub key: PathBuf,
    pub crc: Option<u32>,
    pub rule: Rule,
}

/// Game folders keyed by their root directory. Matches lying loose in a folder shared by
/// several games (or in the inbox root) get no folder and are placed as single files.
pub(super) fn find<'a>(
    items: &[&'a Item],
    library: &Path,
    inbox: Option<&Path>,
    systems: &'a [String],
) -> BTreeMap<PathBuf, FolderGame<'a>> {
    let matched: Vec<Match<'a>> = items
        .iter()
        .filter_map(|it| match (&it.ident, &it.files) {
            (Ident::Known(g), Files::Single(p) | Files::Set { archive: p, .. })
                if systems.contains(&g.system) =>
            {
                Some(Match {
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

    let mut groups: BTreeMap<PathBuf, Vec<Match<'a>>> = BTreeMap::new();
    for m in matched {
        let Some(b) = base(m.key) else { continue };
        if let Some(root) = root_of(m.key, &b, b == library, &shared) {
            groups.entry(root).or_default().push(m);
        }
    }
    groups
        .into_iter()
        .filter_map(|(root, ms)| {
            let fg = folder_game(&root, &ms, systems)?;
            Some((root, fg))
        })
        .collect()
}

fn folder_game<'a>(root: &Path, ms: &[Match<'a>], systems: &'a [String]) -> Option<FolderGame<'a>> {
    let dir = root.file_name()?.to_string_lossy();
    let (stem, marker) = match dir.rsplit_once('.') {
        Some((stem, ext)) if MARKERS.iter().any(|m| ext.eq_ignore_ascii_case(m)) => {
            (stem.to_owned(), Some(ext.to_ascii_lowercase()))
        }
        _ => (dir.clone().into_owned(), None),
    };
    let by_marker = match marker.as_deref() {
        Some("scummvm") => Some("ScummVM"),
        Some("boom") => Some("DOOM"),
        Some(_) => Some("DOS"),
        None => None,
    };
    // a folder already filed under a folder system (`<lib>/ScummVM/Game/`) stays there –
    // keeps audits stable once the marker suffix is gone
    let by_parent = root
        .parent()
        .and_then(Path::file_name)
        .and_then(|p| systems.iter().find(|s| p == s.as_str()))
        .map(String::as_str);
    // otherwise a port/engine database beats generic DOS/ScummVM (their DBs also list e.g.
    // Wolfenstein 3D), then the system most matches belong to
    let system = by_marker.or(by_parent).or_else(|| {
        systems.iter().map(String::as_str).max_by_key(|s| {
            let n = ms.iter().filter(|m| m.game.system == *s).count();
            let specific = n > 0 && !matches!(*s, "DOS" | "ScummVM");
            (
                specific,
                n,
                std::cmp::Reverse(systems.iter().position(|x| x == s)),
            )
        })
    })?;
    let words = |s: &str| -> HashSet<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 1)
            .map(str::to_lowercase)
            .collect()
    };
    let want = words(&stem);
    let best = ms
        .iter()
        .filter(|m| m.game.system == system)
        .chain(ms.iter())
        .max_by_key(|m| {
            let overlap = words(&m.game.name).intersection(&want).count();
            (
                overlap,
                m.game.system == system,
                std::cmp::Reverse(m.key.components().count()),
                std::cmp::Reverse(m.key.to_path_buf()),
            )
        })?;
    Some(FolderGame {
        item: best.item,
        system,
        name: stem,
        key: best.key.to_path_buf(),
        crc: best.game.crc,
        rule: Rule::GameFolder,
    })
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
        let target = self
            .library
            .join(naming::sanitize_file_name(fg.system))
            .join(naming::sanitize_file_name(&fg.name));
        let key_rel = fg.key.strip_prefix(root).unwrap_or(&fg.key);
        self.lpl
            .entry(fg.system.to_owned())
            .or_default()
            .push(lpl::Entry {
                path: target.join(key_rel),
                label: fg.name.clone(),
                crc: fg.crc,
            });
        if root == target {
            self.plan.unchanged += 1;
            self.repair_launchers(root, &target, fg.system);
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
            .filter(|e| !super::frontend::is_metadata(&self.meta_roots, e.path()))
            .map(|e| {
                let rel = e.path().strip_prefix(root).unwrap_or(e.path());
                self.transfer(&it, e.path(), &target.join(rel))
            })
            .collect();
        self.why = Why::new(fg.rule, "");
        if self.commit(&it, ops) {
            self.plan.placed += 1;
            self.repair_launchers(root, &target, fg.system);
        }
    }

    /// Writes the id into broken ScummVM launcher files of a game folder (at its target).
    fn repair_launchers(&mut self, root: &Path, target: &Path, system: &str) {
        // only ScummVM reads launchers; walking every DOS folder costs seconds on a network
        if system != crate::retroarch::scummvm::SYSTEM {
            return;
        }
        let fixes: Vec<_> = walkdir::WalkDir::new(root)
            .sort_by_file_name()
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| {
                let id = crate::scummvm::repaired_id_of(e.path())?;
                let rel = e.path().strip_prefix(root).ok()?;
                Some((target.join(rel), id))
            })
            .collect();
        for (path, id) in fixes {
            self.why = Why::new(Rule::ScummvmLauncher, "");
            self.write(path, id);
        }
    }
}
