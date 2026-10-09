//! Game folders holding a title another folder already has (e.g. `Quake/quake/` and
//! `Quake/tyrquake/`): the extra folder waits for your decision instead of being placed twice.

use super::folders::FolderGame;
use super::{Builder, Decision, Ident, Op, TRASH_DIR, Verdict};
use crate::g1r;
use crate::rules::{Rule, Why};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// Maps each duplicate folder to the folder kept for its title. Kept: already in the library,
/// then the folder with more files (add-ons, extras), then the first path.
pub(super) fn find(
    games: &BTreeMap<PathBuf, FolderGame<'_>>,
    library: &Path,
) -> HashMap<PathBuf, PathBuf> {
    let mut by_title: HashMap<(&str, &str), Vec<&Path>> = HashMap::new();
    for (root, fg) in games {
        if fg.rule != Rule::GameFolder {
            continue;
        }
        if let Ident::Known(g) = &fg.item.ident {
            by_title
                .entry((fg.system, g.name.as_str()))
                .or_default()
                .push(root);
        }
    }
    let mut dups = HashMap::new();
    for (_, mut roots) in by_title.into_iter().filter(|(_, r)| r.len() > 1) {
        roots.sort_by_cached_key(|r| {
            (
                !r.starts_with(library),
                std::cmp::Reverse(count_files(r)),
                r.to_path_buf(),
            )
        });
        let kept = roots[0].to_path_buf();
        for r in &roots[1..] {
            dups.insert(r.to_path_buf(), kept.clone());
        }
    }
    dups
}

fn count_files(root: &Path) -> usize {
    walkdir::WalkDir::new(root)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .count()
}

impl Builder<'_> {
    /// A folder whose title is already kept elsewhere: decision queue unless you decided.
    pub(super) fn duplicate_folder(&mut self, root: &Path, fg: &FolderGame<'_>, kept: &Path) {
        let Ident::Known(g) = &fg.item.ident else {
            return;
        };
        let key = (fg.system.to_owned(), g.name.clone());
        match self.opts.verdicts.get(&key) {
            Some(Verdict::Keep) => self.folder_game(root, fg),
            Some(Verdict::Discard) => self.trash_folder(root, fg),
            _ => {
                let kept = kept.strip_prefix(self.library).unwrap_or(kept);
                self.plan.decisions.push(Decision::Rejected {
                    path: root.to_path_buf(),
                    system: key.0,
                    name: key.1,
                    kept: Some(kept.display().to_string()),
                    reason: format!("{:?}", g1r::Reason::Duplicate),
                });
            }
        }
    }

    /// Moves the whole folder to `_trash/<folder>/`, keeping its layout (undo restores it).
    fn trash_folder(&mut self, root: &Path, fg: &FolderGame<'_>) {
        let Some(name) = root.file_name() else {
            return;
        };
        let target = self.library.join(TRASH_DIR).join(name);
        let ops = walkdir::WalkDir::new(root)
            .sort_by_file_name()
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                let rel = e.path().strip_prefix(root).unwrap_or(e.path());
                Op::Move {
                    from: e.path().to_path_buf(),
                    to: target.join(rel),
                }
            })
            .collect();
        let it = super::Item {
            files: super::Files::Single(root.to_path_buf()),
            ident: fg.item.ident.clone(),
            in_library: fg.item.in_library,
        };
        self.why = Why::new(Rule::Verdict, "discarded (Duplicate)");
        if self.commit(&it, ops) {
            self.plan.discarded += 1;
        }
    }
}
