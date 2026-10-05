//! Building a plan: 1G1R per system, target paths, TBD queue, quarantine, playlists.

use super::{
    BIOS_DIR, Decision, Files, Game, Ident, Item, Mode, Op, Options, PLAYLIST_DIR, Plan,
    QUARANTINE_DIR, TRASH_DIR, Verdict, lpl,
};
use crate::{g1r, naming};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

mod archives;
mod place;

use place::quarantine_sources;

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
    let managed = [QUARANTINE_DIR, PLAYLIST_DIR, TRASH_DIR, BIOS_DIR].map(|d| library.join(d));
    let items: Vec<&Item> = items
        .iter()
        .filter(|it| !managed.iter().any(|m| it.files.primary().starts_with(m)))
        .collect();

    // Archives with any unknown member stay whole (e.g. multi-disk games where only some
    // disks match): nothing is extracted, the archive goes to quarantine as is.
    let mixed: HashSet<&Path> = items
        .iter()
        .filter(|it| matches!(it.ident, Ident::Unknown))
        .filter_map(|it| it.files.archive().map(PathBuf::as_path))
        .collect();
    let items: Vec<&Item> = items
        .into_iter()
        .filter(|it| {
            matches!(it.ident, Ident::Unknown)
                || !it
                    .files
                    .archive()
                    .is_some_and(|a| mixed.contains(a.as_path()))
        })
        .collect();
    let mut known: BTreeMap<&str, Vec<(&Item, &Game)>> = BTreeMap::new();
    let mut arcade_seen: HashSet<(&str, &str)> = HashSet::new();
    for &it in &items {
        match &it.ident {
            // Arcade sets are kept as-is: every exact match is its own release, no 1G1R.
            // A second copy of the same set is a duplicate (library copies come first and win).
            Ident::Known(g) if matches!(it.files, Files::Set { .. }) => {
                if arcade_seen.insert((g.system.as_str(), g.name.as_str())) {
                    b.why = "arcade romset".into();
                    b.release(&g.system, &[&(it, g)]);
                } else if opts.verdicts.get(&(g.system.clone(), g.name.clone()))
                    == Some(&Verdict::Discard)
                {
                    b.why = "discarded by you (duplicate set)".into();
                    b.discard(it);
                } else {
                    b.plan.decisions.push(Decision::Rejected {
                        path: it.files.primary().clone(),
                        system: g.system.clone(),
                        name: g.name.clone(),
                        kept: Some(g.name.clone()),
                        reason: format!("{:?}", g1r::Reason::Duplicate),
                    });
                }
            }
            Ident::Known(g) => known.entry(&g.system).or_default().push((it, g)),
            Ident::Bios(g) => {
                b.why = "BIOS".into();
                b.bios(it, g);
            }
            Ident::Ambiguous(c) => b.plan.decisions.push(Decision::Ambiguous {
                path: it.files.primary().clone(),
                candidates: c.clone(),
            }),
            // Unknown members stay packed; the whole archive is quarantined at the end.
            Ident::Unknown if it.files.archive().is_some() => {}
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
    b.finish_archives(&items);
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

    pub(super) fn quarantine(&mut self, it: &Item) {
        let dir = self.library.join(QUARANTINE_DIR);
        let ops = quarantine_sources(&it.files)
            .iter()
            .map(|f| self.transfer(it, f, &dir.join(self.quarantine_rel(f))))
            .collect();
        if self.commit(it, ops) {
            self.plan.quarantined += 1;
        }
    }

    /// Path below `_quarantine/`: relative to the inbox (or library) root, else the bare name.
    fn quarantine_rel(&self, f: &Path) -> PathBuf {
        [self.opts.inbox.as_deref(), Some(self.library)]
            .into_iter()
            .flatten()
            .find_map(|root| f.strip_prefix(root).ok())
            .filter(|rel| !rel.as_os_str().is_empty())
            .map_or_else(|| PathBuf::from(file_name(f)), Path::to_path_buf)
    }

    /// Puts a BIOS set where RetroArch's core looks for it.
    fn bios(&mut self, it: &Item, g: &Game) {
        let from = it.files.primary();
        let to = self
            .library
            .join(crate::arcade::bios_dir(&g.system))
            .join(file_name(from));
        let ops = vec![self.transfer(it, from, &to)];
        let ops = ops
            .into_iter()
            .filter(|op| op.source() != Some(op.target()))
            .collect::<Vec<_>>();
        if !ops.is_empty() {
            self.commit(it, ops);
        }
    }

    /// Moves an item to the trash folder (always a move, even from the inbox).
    fn discard(&mut self, it: &Item) {
        let dir = self.library.join(TRASH_DIR);
        // A member is discarded by not extracting it; the archive is trashed once emptied.
        let members = it.files.archive().is_some();
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
        if let Some(archive) = it.files.archive() {
            // `from` is the member name for archived items.
            return Op::Extract {
                archive: archive.clone(),
                member: from.to_string_lossy().into_owned(),
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
        if let Some(archive) = it.files.archive() {
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

    fn skip(&mut self, it: &Item, reason: String) {
        self.plan.decisions.push(Decision::Skipped {
            path: it.files.primary().clone(),
            reason,
        });
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
