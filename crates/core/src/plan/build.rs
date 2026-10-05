//! Building a plan: 1G1R per system, target paths, TBD queue, quarantine, playlists.

use super::sheet::rewrite_sheet;
use super::{
    Decision, Files, Game, Ident, Item, Mode, Op, Options, PLAYLIST_DIR, Plan, QUARANTINE_DIR,
    TRASH_DIR, Verdict, lpl,
};
use crate::{disc, g1r, naming};
use std::collections::{BTreeMap, HashMap, HashSet};
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
        why: String::new(),
        members_done: HashMap::new(),
    };
    let managed = [QUARANTINE_DIR, PLAYLIST_DIR, TRASH_DIR].map(|d| library.join(d));
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
            Ident::Unknown => {
                b.why = "unknown: no database match".into();
                b.quarantine(it)
            }
            Ident::Skip(reason) => b.plan.decisions.push(Decision::Skipped {
                path: it.files.primary().clone(),
                reason: reason.clone(),
            }),
        }
    }
    for (system, list) in &known {
        for gp in g1r::select(list, |(_, g)| &g.name, &opts.rules) {
            let tie = gp.needs_decision;
            let (mut picked, mut rejected) = (gp.picked, gp.rejected);
            if tie {
                let rel = |g: &Game| naming::release_name(&g.name);
                let mut releases: Vec<String> = Vec::new();
                for g in picked
                    .iter()
                    .map(|m| m.1)
                    .chain(rejected.iter().map(|(m, _)| m.1))
                {
                    let r = rel(g);
                    if !releases.contains(&r) {
                        releases.push(r);
                    }
                }
                let preferred = releases.iter().find(|r| {
                    opts.verdicts.get(&((*system).to_owned(), (*r).clone()))
                        == Some(&Verdict::Prefer)
                });
                let Some(preferred) = preferred.cloned() else {
                    b.plan.decisions.push(Decision::Tie {
                        system: (*system).to_owned(),
                        releases,
                    });
                    continue;
                };
                let all: Vec<_> = picked
                    .drain(..)
                    .map(|m| (m, g1r::Reason::TieBreak))
                    .chain(rejected.drain(..))
                    .collect();
                for (m, reason) in all {
                    if rel(m.1) == preferred {
                        picked.push(m);
                    } else {
                        rejected.push((m, reason));
                    }
                }
            }
            b.why = match (tie, picked.len()) {
                (true, _) => "1G1R pick (preferred by you)".into(),
                (_, n) if n > 1 => format!("1G1R pick, {n} discs"),
                _ => "1G1R pick".into(),
            };
            b.release(system, &picked);
            let kept = picked.first().map(|(_, g)| g.name.clone());
            for ((it, g), reason) in &rejected {
                let key = ((*system).to_owned(), g.name.clone());
                match opts.verdicts.get(&key) {
                    // A duplicate would claim the pick's own target; only discarding makes sense.
                    Some(Verdict::Keep) if *reason != g1r::Reason::Duplicate => {
                        b.why = format!("kept by you (1G1R: {reason:?})");
                        b.release(system, &[&(*it, *g)]);
                        continue;
                    }
                    Some(Verdict::Discard) => {
                        b.why = format!("discarded by you (1G1R: {reason:?})");
                        b.discard(it);
                        continue;
                    }
                    _ => {}
                }
                b.plan.decisions.push(Decision::Rejected {
                    path: it.files.primary().clone(),
                    system: (*system).to_owned(),
                    name: g.name.clone(),
                    kept: kept.clone(),
                    reason: format!("{reason:?}"),
                });
            }
        }
    }
    b.trash_emptied_archives(&items);
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
    /// Reason attached to the ops added next.
    why: String,
    /// Members of multi-ROM archives handled so far (extracted, quarantined or discarded).
    members_done: HashMap<PathBuf, usize>,
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
                self.why = "multi-disc playlist".into();
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
        let primary = target(&name_source(&it.files));
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

    fn quarantine(&mut self, it: &Item) {
        let dir = self.library.join(QUARANTINE_DIR);
        let ops = it
            .files
            .all()
            .into_iter()
            .map(|f| self.transfer(it, f, &dir.join(file_name(&name_source(&it.files)))))
            .collect();
        if self.commit(it, ops) {
            self.plan.quarantined += 1;
        }
    }

    /// Moves an item to the trash folder (always a move, even from the inbox).
    fn discard(&mut self, it: &Item) {
        let dir = self.library.join(TRASH_DIR);
        // A member is discarded by not extracting it; the archive is trashed once emptied.
        let members = matches!(it.files, Files::Member { .. });
        let ops = it
            .files
            .all()
            .into_iter()
            .map(|f| Op::Move {
                from: f.clone(),
                to: dir.join(file_name(f)),
            })
            .filter(|_| !members)
            .collect();
        if self.commit(it, ops) {
            self.plan.discarded += 1;
        }
    }

    fn transfer(&self, it: &Item, from: &Path, to: &Path) -> Op {
        let (from, to) = (from.to_path_buf(), to.to_path_buf());
        if let Files::Member { member, .. } = &it.files {
            return Op::Extract {
                archive: from,
                member: member.clone(),
                to,
            };
        }
        match (it.in_library, self.opts.mode) {
            (true, _) | (_, Mode::Move) => Op::Move { from, to },
            (_, Mode::Copy) => Op::Copy { from, to },
            (_, Mode::Hardlink) => Op::Hardlink { from, to },
            (_, Mode::Reflink) => Op::Reflink { from, to },
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
        if let Files::Member { archive, .. } = &it.files {
            *self.members_done.entry(archive.clone()).or_default() += 1;
        }
        let why = if it.in_library && self.why.starts_with("1G1R") {
            format!("{} – rename to naming scheme", self.why)
        } else {
            self.why.clone()
        };
        self.plan.why.extend(ops.iter().map(|_| why.clone()));
        self.plan.ops.extend(ops);
        true
    }

    /// Writes a text file unless it already has exactly this content.
    fn write(&mut self, path: PathBuf, contents: String) {
        if fs::read_to_string(&path).is_ok_and(|old| old == contents) {
            return;
        }
        self.claimed.insert(path.clone());
        self.plan.why.push(self.why.clone());
        self.plan.ops.push(Op::Write { path, contents });
    }

    fn playlists(&mut self) {
        let Some(dir) = self.opts.playlists.clone() else {
            return;
        };
        self.why = "RetroArch playlist".into();
        for (system, entries) in std::mem::take(&mut self.lpl) {
            let path = dir.join(format!("{}.lpl", naming::sanitize_file_name(&system)));
            self.write(path, lpl::render(&system, &entries));
        }
    }

    /// Moves multi-ROM archives to the trash once every member was handled, unless the
    /// inbox is only copied/linked from.
    fn trash_emptied_archives(&mut self, items: &[&Item]) {
        let mut total: BTreeMap<&Path, (usize, bool)> = BTreeMap::new();
        for it in items {
            if let Files::Member { archive, .. } = &it.files {
                total.entry(archive).or_default().0 += 1;
                total.entry(archive).or_default().1 = it.in_library;
            }
        }
        self.why = "archive fully extracted".into();
        for (archive, (n, in_library)) in total {
            let done = self.members_done.get(archive).copied().unwrap_or(0);
            if done < n || !(in_library || self.opts.mode == Mode::Move) {
                continue;
            }
            let it = Item {
                files: Files::Single(archive.to_path_buf()),
                ident: Ident::Unknown,
                in_library,
            };
            let op = Op::Move {
                from: archive.to_path_buf(),
                to: self.library.join(TRASH_DIR).join(file_name(archive)),
            };
            self.commit(&it, vec![op]);
        }
    }

    fn skip(&mut self, it: &Item, reason: String) {
        self.plan.decisions.push(Decision::Skipped {
            path: it.files.primary().clone(),
            reason,
        });
    }
}

/// The path whose name/extension the library file takes (a member's own name for archives).
fn name_source(files: &Files) -> PathBuf {
    match files {
        Files::Member { member, .. } => PathBuf::from(member),
        f => f.primary().clone(),
    }
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
