//! Building a plan: 1G1R per system, target paths, trash, quarantine, playlists.

use super::{
    Decision, Files, Game, Ident, Item, Mode, Op, Options, PLAYLIST_DIR, Plan, QUARANTINE_DIR,
    TRASH_DIR, lpl,
};
use crate::{disc, g1r, naming};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Plans placing `items` into `library`. Nothing is touched on disk (reads only).
/// Library items should come first so they win over identical inbox copies.
pub fn build(items: &[Item], library: &Path, opts: &Options) -> Plan {
    let mut b = Builder {
        library,
        opts,
        plan: Plan::default(),
        claimed: HashSet::new(),
        lpl: BTreeMap::new(),
    };
    let managed = [TRASH_DIR, QUARANTINE_DIR, PLAYLIST_DIR].map(|d| library.join(d));
    let items: Vec<&Item> = items
        .iter()
        .filter(|it| !managed.iter().any(|m| it.files.primary().starts_with(m)))
        .collect();

    let mut known: BTreeMap<&str, Vec<(&Item, &Game)>> = BTreeMap::new();
    for &it in &items {
        match &it.ident {
            Ident::Known(g) => known.entry(&g.system).or_default().push((it, g)),
            Ident::Ambiguous(c) => b.plan.decisions.push(Decision::Ambiguous {
                path: it.files.primary().clone(),
                candidates: c.clone(),
            }),
            Ident::Unknown => b.quarantine(it),
            Ident::Skip(reason) => b.plan.decisions.push(Decision::Skipped {
                path: it.files.primary().clone(),
                reason: reason.clone(),
            }),
        }
    }
    for (system, list) in &known {
        for gp in g1r::select(list, |(_, g)| &g.name, &opts.rules) {
            if gp.needs_decision {
                let mut releases: Vec<String> =
                    gp.picked.iter().map(|(_, g)| g.name.clone()).collect();
                releases.extend(gp.rejected.iter().map(|((_, g), _)| g.name.clone()));
                b.plan.decisions.push(Decision::Tie {
                    system: (*system).to_owned(),
                    releases,
                });
                continue;
            }
            b.release(system, &gp.picked);
            for ((it, _), _) in &gp.rejected {
                b.trash(it);
            }
        }
    }
    b.playlists();
    b.plan
}

struct Builder<'a> {
    library: &'a Path,
    opts: &'a Options,
    plan: Plan,
    /// Targets claimed by earlier ops of this plan.
    claimed: HashSet<PathBuf>,
    lpl: BTreeMap<String, Vec<lpl::Entry>>,
}

impl Builder<'_> {
    /// Places all media of one picked release; multi-disc sets get a folder and an `.m3u`.
    fn release(&mut self, system: &str, media: &[&(&Item, &Game)]) {
        let multi = media.len() > 1;
        let mut placed = Vec::new();
        for (it, g) in media {
            if let Some(p) = self.place(it, g, multi) {
                placed.push((p, g));
            }
        }
        let Some((first, g)) = placed.first() else {
            return;
        };
        let entry = if multi {
            let m3u = self.library.join(naming::playlist_path(system, &g.name));
            let mut text = String::new();
            for (p, _) in &placed {
                text.push_str(&file_name(p));
                text.push('\n');
            }
            if placed.len() == media.len() {
                self.write(m3u.clone(), text);
            }
            m3u
        } else {
            first.clone()
        };
        self.lpl
            .entry(system.to_owned())
            .or_default()
            .push(lpl::Entry {
                path: entry,
                label: naming::release_name(&g.name),
                crc: g.crc,
            });
    }

    /// Ops moving one item to its library path; returns the new primary path.
    fn place(&mut self, it: &Item, g: &Game, multi: bool) -> Option<PathBuf> {
        let target = |src: &Path| {
            let rel = naming::target_path(&g.system, &g.name, &ext_of(src), multi);
            self.library.join(rel)
        };
        let primary = target(it.files.primary());
        let mut ops = vec![self.transfer(it, it.files.primary(), &primary)];
        if let Files::Sheet { sheet, tracks } = &it.files {
            let base = naming::sanitize_file_name(&g.name);
            let width = if tracks.len() > 9 { 2 } else { 1 };
            let mut renames = Vec::new();
            for (i, t) in tracks.iter().enumerate() {
                let name = if tracks.len() == 1 {
                    format!("{base}.{}", ext_of(t))
                } else {
                    format!("{base} (Track {:0width$}).{}", i + 1, ext_of(t))
                };
                renames.push((file_name(t), name.clone()));
                ops.push(self.transfer(it, t, &primary.with_file_name(name)));
            }
            let text = match disc::read_text(sheet) {
                Ok(t) => t,
                Err(e) => {
                    self.skip(it, format!("cannot read sheet: {e}"));
                    return None;
                }
            };
            let new = rewrite_sheet(&text, &renames);
            if new != text {
                ops.push(Op::Write {
                    path: primary.clone(),
                    contents: new,
                });
            }
        }
        ops.retain(|op| op.source() != Some(op.target()));
        if ops.is_empty() {
            self.plan.unchanged += 1;
        } else if self.commit(it, ops) {
            self.plan.placed += 1;
        } else {
            return None;
        }
        Some(primary)
    }

    fn trash(&mut self, it: &Item) {
        if !it.in_library && self.opts.mode != Mode::Move {
            return; // rejected inbox copies simply stay where they are
        }
        let dir = self.library.join(TRASH_DIR).join(&self.opts.stamp);
        let ops = it
            .files
            .all()
            .into_iter()
            .map(|f| {
                let rel = match f.strip_prefix(self.library) {
                    Ok(r) if it.in_library => r.to_path_buf(),
                    _ => PathBuf::from(file_name(f)),
                };
                Op::Move {
                    from: f.clone(),
                    to: dir.join(rel),
                }
            })
            .collect();
        if self.commit(it, ops) {
            self.plan.trashed += 1;
        }
    }

    fn quarantine(&mut self, it: &Item) {
        let dir = self.library.join(QUARANTINE_DIR);
        let ops = it
            .files
            .all()
            .into_iter()
            .map(|f| self.transfer(it, f, &dir.join(file_name(f))))
            .collect();
        if self.commit(it, ops) {
            self.plan.quarantined += 1;
        }
    }

    fn transfer(&self, it: &Item, from: &Path, to: &Path) -> Op {
        let (from, to) = (from.to_path_buf(), to.to_path_buf());
        match (it.in_library, self.opts.mode) {
            (true, _) | (_, Mode::Move) => Op::Move { from, to },
            (_, Mode::Copy) => Op::Copy { from, to },
            (_, Mode::Hardlink) => Op::Hardlink { from, to },
        }
    }

    /// Adds an item's ops unless a target exists or is already claimed (then: conflict).
    fn commit(&mut self, it: &Item, ops: Vec<Op>) -> bool {
        let own: Vec<&Path> = ops.iter().filter_map(Op::source).collect();
        let clash = ops.iter().find(|op| {
            let t = op.target();
            let rewrite = matches!(op, Op::Write { .. });
            self.claimed.contains(t) && !rewrite || (t.exists() && !own.contains(&t) && !rewrite)
        });
        if let Some(op) = clash {
            self.plan.decisions.push(Decision::Conflict {
                path: it.files.primary().clone(),
                target: op.target().to_path_buf(),
            });
            return false;
        }
        self.claimed
            .extend(ops.iter().map(|op| op.target().to_path_buf()));
        self.plan.ops.extend(ops);
        true
    }

    /// Writes a text file unless it already has exactly this content.
    fn write(&mut self, path: PathBuf, contents: String) {
        if fs::read_to_string(&path).is_ok_and(|old| old == contents) {
            return;
        }
        self.claimed.insert(path.clone());
        self.plan.ops.push(Op::Write { path, contents });
    }

    fn playlists(&mut self) {
        let Some(dir) = self.opts.playlists.clone() else {
            return;
        };
        for (system, entries) in std::mem::take(&mut self.lpl) {
            let path = dir.join(format!("{}.lpl", naming::sanitize_file_name(&system)));
            self.write(path, lpl::render(&system, &entries));
        }
    }

    fn skip(&mut self, it: &Item, reason: String) {
        self.plan.decisions.push(Decision::Skipped {
            path: it.files.primary().clone(),
            reason,
        });
    }
}

/// Replaces track file names in a sheet (case-insensitive, once per line).
fn rewrite_sheet(text: &str, renames: &[(String, String)]) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let lower = line.to_lowercase();
        let hit = renames.iter().find_map(|(old, new)| {
            lower
                .find(&old.to_lowercase())
                .filter(|_| lower.len() == line.len())
                .map(|i| (i, old.len(), new))
        });
        match hit {
            Some((i, len, new)) => {
                out.push_str(&line[..i]);
                out.push_str(new);
                out.push_str(&line[i + len..]);
            }
            None => out.push_str(line),
        }
    }
    out
}

fn ext_of(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_cue_file_lines() {
        let cue = "FILE \"game (track 1).BIN\" BINARY\n  TRACK 01 MODE2/2352\nFILE \"Game (Track 2).bin\" BINARY\n";
        let r = [
            ("Game (Track 1).bin".into(), "New (Track 1).bin".into()),
            ("Game (Track 2).bin".into(), "New (Track 2).bin".into()),
        ];
        assert_eq!(
            rewrite_sheet(cue, &r),
            "FILE \"New (Track 1).bin\" BINARY\n  TRACK 01 MODE2/2352\nFILE \"New (Track 2).bin\" BINARY\n"
        );
    }
}
