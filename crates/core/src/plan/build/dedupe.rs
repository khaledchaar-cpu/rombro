//! Library maintenance: bit-identical copies of a game are trashed, one copy stays.

use super::{Builder, file_name};
use crate::arcade;
use crate::plan::{Files, Ident, Item, TRASH_DIR};
use crate::rules::{Rule, Why};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// Trashes bit-identical copies among identified library files lying directly in a system
/// (or arcade core) folder, across systems. Kept is a copy whose folder's database lists it
/// under its file name (a core finds sets only by their short name), among those the best
/// arcade core ([`arcade::PRIORITY`]), then the first path. Game folders,
/// multi-disc folders and `_bios` are left alone (shared files there are intended).
/// Returns the items still to plan.
pub(super) fn run<'a>(b: &mut Builder<'_>, items: Vec<&'a Item>) -> Vec<&'a Item> {
    if b.opts.hashes.is_empty() {
        return items;
    }
    let mut groups: BTreeMap<[u8; 20], BTreeMap<PathBuf, Vec<&str>>> = BTreeMap::new();
    for it in &items {
        let Some(outer) = candidate(b.library, it) else {
            continue;
        };
        let Some(sha1) = b.opts.hashes.get(outer) else {
            continue;
        };
        groups
            .entry(*sha1)
            .or_default()
            .entry(outer.clone())
            .or_insert_with(|| systems(it));
    }
    let mut gone: HashSet<PathBuf> = HashSet::new();
    for copies in groups.into_values().filter(|c| c.len() > 1) {
        let Some(kept) = copies
            .iter()
            .min_by_key(|(p, systems)| {
                // a copy under a name its folder's core doesn't know is never the one kept
                let folder = folder(p);
                let fits = systems.contains(&folder.as_str()) && !b.opts.misnamed.contains(*p);
                (!fits, arcade::rank(&folder), p.to_path_buf())
            })
            .map(|(p, _)| p.clone())
        else {
            continue;
        };
        let rel = kept
            .strip_prefix(b.library)
            .unwrap_or(&kept)
            .display()
            .to_string();
        for p in copies.into_keys().filter(|p| *p != kept) {
            b.why = Why::new(Rule::LibraryDuplicate, format!("bit-identical to {rel}"));
            let dir = b.library.join(TRASH_DIR);
            let to = b.free_trash_target(&dir, &p);
            let it = Item {
                files: Files::Single(p.clone()),
                ident: Ident::Unknown,
                in_library: true,
            };
            if b.commit(
                &it,
                vec![crate::plan::Op::Move {
                    from: p.clone(),
                    to,
                }],
            ) {
                b.plan.discarded += 1;
            }
            gone.insert(p);
        }
    }
    items
        .into_iter()
        .filter(|it| !gone.contains(outer(it)))
        .collect()
}

fn outer(it: &Item) -> &PathBuf {
    it.files.archive().unwrap_or(it.files.primary())
}

/// The whole file to compare, if the item is an identified game file directly in a
/// non-managed top folder of the library.
fn candidate<'a>(library: &Path, it: &'a Item) -> Option<&'a PathBuf> {
    if !it.in_library {
        return None;
    }
    match (&it.ident, &it.files) {
        (Ident::Unknown | Ident::Incomplete(_) | Ident::Skip(_), _) => return None,
        (_, Files::Set { chds, .. }) if !chds.is_empty() => return None,
        (_, Files::Single(_) | Files::Set { .. }) => {}
        _ => return None,
    }
    let p = outer(it);
    let top = p.parent()?;
    (top.parent()? == library && !file_name(top).starts_with('_')).then_some(p)
}

/// Systems whose database lists the file under this very name (own system, for arcade
/// sets also the other cores listing the set).
fn systems(it: &Item) -> Vec<&str> {
    let own = match &it.ident {
        Ident::Known(g) | Ident::Bios(g) => Some(g.system.as_str()),
        _ => None,
    };
    let alt = match &it.files {
        Files::Set { alt, .. } => alt.iter().map(|g| g.system.as_str()).collect(),
        _ => Vec::new(),
    };
    own.into_iter().chain(alt).collect()
}

fn folder(p: &Path) -> String {
    p.parent().map(file_name).unwrap_or_default()
}
