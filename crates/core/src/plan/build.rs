//! Building a plan: 1G1R per system, target paths, TBD queue, quarantine, playlists.

use super::{
    BIOS_DIR, Decision, Files, Game, Ident, Item, Mode, Op, Options, PLAYLIST_DIR, Plan,
    QUARANTINE_DIR, TRASH_DIR, Verdict, lpl,
};
use crate::rules::{Rule, Why};
use crate::{g1r, naming};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Folder in `_trash` for unknown files (when they don't go to `_quarantine`).
const UNKNOWN_TRASH: &str = "unknown";

mod archives;
mod daphne;
mod firmware;
mod folders;
mod frontend;
mod msu;
pub use daphne::SYSTEM as DAPHNE_SYSTEM;
pub use msu::SYSTEM as MSU1_SYSTEM;
mod named;
mod ports;
pub use folders::FOLDER_SYSTEMS;
pub use ports::PORTS;

/// Name-only identification ([`named`]) plus libretro ports ([`ports`]).
pub fn name_only(items: &[Item], rules: &crate::g1r::Rules) -> Vec<Item> {
    ports::apply(named::apply(items, rules))
}
mod place;

use place::quarantine_sources;

/// Plans placing `items` into `library`. Nothing is touched on disk (reads only).
/// Library items should come first so they win over identical inbox copies.
pub fn build(items: &[Item], library: &Path, opts: &Options) -> Plan {
    let items = &name_only(items, &opts.rules);
    let mut b = Builder {
        library,
        opts,
        plan: Plan::default(),
        claimed: HashMap::new(),
        lpl: BTreeMap::new(),
        why: Why::new(Rule::G1rPick, ""),
        members_done: HashMap::new(),
        identified: items
            .iter()
            .filter(|it| !matches!(it.ident, Ident::Unknown))
            .filter_map(|it| it.files.primary().parent().map(Path::to_path_buf))
            .collect(),
        meta_roots: HashSet::new(),
    };
    let managed = [QUARANTINE_DIR, PLAYLIST_DIR, TRASH_DIR, BIOS_DIR].map(|d| library.join(d));
    let quarantine = library.join(QUARANTINE_DIR);
    let items: Vec<&Item> = items
        .iter()
        // quarantined files stay put, except frontend metadata (below)
        .filter(|it| {
            let p = it.files.primary();
            p.starts_with(&quarantine) || !managed.iter().any(|m| p.starts_with(m))
        })
        .filter(|it| {
            let p = it.files.archive().unwrap_or(it.files.primary());
            !opts.ignore.iter().any(|i| p.starts_with(i))
        })
        .collect();

    // Frontend metadata (gamelist.xml, scraped media) goes to the trash before anything else,
    // so game folders move without it.
    if opts.rules.frontend_trash {
        b.meta_roots = frontend::roots(&items);
    }
    let (meta, items): (Vec<&Item>, Vec<&Item>) = items.into_iter().partition(|it| {
        matches!(it.ident, Ident::Unknown)
            && it.files.archive().is_none()
            && frontend::is_metadata(&b.meta_roots, it.files.primary())
    });
    b.why = Why::new(Rule::FrontendMeta, "");
    for it in meta {
        b.trash_meta(it);
    }
    let outer = |it: &Item| it.files.archive().unwrap_or(it.files.primary()).clone();
    let (quarantined, items): (Vec<&Item>, Vec<&Item>) = items
        .into_iter()
        .partition(|it| it.files.primary().starts_with(&quarantine));
    if opts.rules.unknown_to_trash {
        // the old quarantine is emptied into the trash; files identified meanwhile stay
        let mut still_unknown: BTreeMap<PathBuf, bool> = BTreeMap::new();
        for it in &quarantined {
            let unknown = matches!(it.ident, Ident::Unknown | Ident::Incomplete(_));
            *still_unknown.entry(outer(it)).or_insert(true) &= unknown;
        }
        b.why = Why::new(Rule::Quarantine, "old quarantine emptied into the trash");
        for (p, unknown) in still_unknown {
            if unknown {
                b.trash_quarantined(&p, &quarantine);
            }
        }
    }

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
    let mut folder_systems = opts.rules.folder_systems.clone();
    folder_systems.extend(ports::PORTS.iter().map(|(s, _)| (*s).to_owned()));
    let (mut folder_games, foreign) =
        folders::find(&items, library, opts.inbox.as_deref(), &folder_systems);
    let roots: Vec<&Path> = [Some(library), opts.inbox.as_deref()]
        .into_iter()
        .flatten()
        .collect();
    // MSU-1 folders win over chip dumps inside them that a database knows
    let msu = msu::find(&items, &roots);
    folder_games.retain(|root, _| {
        !msu.keys()
            .any(|m| root.starts_with(m) || m.starts_with(root))
    });
    folder_games.extend(msu);
    let daphne = daphne::find(&items, &roots, library);
    folder_games.retain(|root, _| !daphne.iter().any(|d| root.starts_with(d)));
    let items: Vec<&Item> = items
        .into_iter()
        .filter(|it| {
            let p = it.files.archive().unwrap_or(it.files.primary());
            !p.ancestors().any(|a| {
                folder_games.contains_key(a) || daphne.contains(a) || foreign.iter().any(|f| f == a)
            })
        })
        .collect();
    for root in &daphne {
        b.daphne(root);
    }
    for (root, fg) in &folder_games {
        b.folder_game(root, fg);
        if fg.rule == Rule::Msu1
            && let Some(rel) = msu::missing_marker(root, &fg.key)
        {
            let target = library
                .join(naming::sanitize_file_name(fg.system))
                .join(naming::sanitize_file_name(&fg.name))
                .join(rel);
            b.why = Why::new(Rule::Msu1, "missing .msu marker added");
            b.write(target, String::new());
        }
    }
    // all items, for trashing archives whose members were all handled
    let archived = items.clone();
    let disk_sets = archives::disk_sets(&items);
    for (archive, members) in &disk_sets {
        b.disk_set(archive, members);
    }
    // multi-disk games already in the library (`<Game>/<Game>.m3u`) are finished units:
    // their disks may carry inconsistent database names and must not be renamed apart
    let mut placed_sets: BTreeMap<&Path, &Game> = BTreeMap::new();
    let items: Vec<&Item> = items
        .into_iter()
        .filter(|it| {
            if it
                .files
                .archive()
                .is_some_and(|a| disk_sets.contains_key(a.as_path()))
            {
                return false;
            }
            match (&it.ident, it.files.primary().parent()) {
                (Ident::Known(g), Some(dir)) if it.in_library && has_own_m3u(dir) => {
                    placed_sets.entry(dir).or_insert(g);
                    false
                }
                _ => true,
            }
        })
        .collect();
    for (dir, g) in placed_sets {
        b.placed_set(dir, g);
    }
    // (system, identified by name only) – name-only releases never compete with verified dumps
    let mut known: BTreeMap<(&str, bool), Vec<(&Item, &Game)>> = BTreeMap::new();
    let mut arcade: Vec<(&Item, &Game)> = Vec::new();
    for &it in &items {
        match &it.ident {
            Ident::Known(g) if matches!(it.files, Files::Set { .. }) => arcade.push((it, g)),
            Ident::Known(g) => known.entry((&g.system, false)).or_default().push((it, g)),
            Ident::Named(g) => known.entry((&g.system, true)).or_default().push((it, g)),
            Ident::Bios(g) => {
                b.why = Why::new(Rule::Bios, "");
                b.bios(it, g);
            }
            Ident::Firmware(paths) => b.firmware(it, paths),
            Ident::Ambiguous(c) => b.plan.decisions.push(Decision::Ambiguous {
                path: it.files.primary().clone(),
                candidates: c.clone(),
            }),
            // Unknown members stay packed; the whole archive is quarantined at the end.
            Ident::Unknown if it.files.archive().is_some() => {}
            Ident::Unknown if b.left_alone(it.files.primary()) => {}
            Ident::Unknown => {
                b.why = Why::new(Rule::Quarantine, "no database match");
                b.quarantine(it)
            }
            Ident::Incomplete(reason) => {
                b.why = Why::new(Rule::ArcadeDat, reason.clone());
                b.quarantine(it)
            }
            Ident::Skip(reason) => b.plan.decisions.push(Decision::Skipped {
                path: it.files.primary().clone(),
                reason: reason.clone(),
            }),
        }
    }
    b.arcade(&arcade);
    for ((system, by_name), list) in &known {
        let rules = opts.rules.for_system(system);
        for gp in g1r::select(list, |(_, g)| &g.name, &rules) {
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
            b.why = Why::new(
                if *by_name {
                    Rule::NameOnly
                } else {
                    Rule::G1rPick
                },
                match (tie, picked.len()) {
                    (true, _) => "preferred by you".into(),
                    (_, n) if n > 1 => format!("{n} discs"),
                    _ => String::new(),
                },
            );
            b.release(system, &picked);
            let kept = picked.first().map(|(_, g)| g.name.clone());
            for ((it, g), reason) in &rejected {
                let key = ((*system).to_owned(), g.name.clone());
                match opts.verdicts.get(&key) {
                    // A duplicate would claim the pick's own target; only discarding makes sense.
                    Some(Verdict::Keep) if *reason != g1r::Reason::Duplicate => {
                        b.why = Why::new(Rule::Verdict, format!("kept ({reason:?})"));
                        b.release(system, &[&(*it, *g)]);
                        continue;
                    }
                    Some(Verdict::Discard) => {
                        b.why = Why::new(Rule::Verdict, format!("discarded ({reason:?})"));
                        b.discard(it);
                        continue;
                    }
                    _ => {}
                }
                // An archive member that duplicates the pick needs no decision: it is simply
                // not extracted, and the archive counts as handled.
                if *reason == g1r::Reason::Duplicate
                    && let Files::Member { archive, .. } = &it.files
                {
                    *b.members_done.entry(archive.clone()).or_default() += 1;
                    continue;
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
    b.finish_archives(&archived);
    b.playlists();
    b.plan
}

struct Builder<'a> {
    library: &'a Path,
    opts: &'a Options,
    plan: Plan,
    /// Targets claimed by earlier ops of this plan, with the file they come from.
    claimed: HashMap<PathBuf, Option<PathBuf>>,
    lpl: BTreeMap<String, Vec<lpl::Entry>>,
    /// Reason attached to the ops added next.
    why: Why,
    /// Members of multi-ROM archives handled so far (extracted, quarantined or discarded).
    members_done: HashMap<PathBuf, usize>,
    /// Folders holding at least one identified item; unknown files elsewhere are left alone.
    identified: HashSet<PathBuf>,
    /// Folders with frontend metadata to trash (see [`frontend`]); empty if the rule is off.
    meta_roots: HashSet<PathBuf>,
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
                self.why = Why::new(Rule::MultiDisc, "");
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

    /// Unknown files go to `_trash/unknown` (default) or `_quarantine`, keeping their path.
    pub(super) fn quarantine(&mut self, it: &Item) {
        let dir = if self.opts.rules.unknown_to_trash {
            self.library.join(TRASH_DIR).join(UNKNOWN_TRASH)
        } else {
            self.library.join(QUARANTINE_DIR)
        };
        let ops = quarantine_sources(&it.files)
            .iter()
            .map(|f| self.transfer(it, f, &dir.join(self.quarantine_rel(f))))
            .collect();
        if self.commit(it, ops) {
            self.plan.quarantined += 1;
        }
    }

    /// Unknown files in folders without any identified item (game installs, frontend media,
    /// unsupported formats) or in a BIOS folder stay where they are.
    fn left_alone(&self, p: &Path) -> bool {
        if !self.opts.rules.quarantine {
            return true;
        }
        let Some(dir) = p.parent() else { return false };
        let is_root = Some(dir) == self.opts.inbox.as_deref() || dir == self.library;
        // a frontend's BIOS folder (`bios`, `00bios`, `system`) is kept as it is
        let roots = [self.opts.inbox.as_deref(), Some(self.library)];
        let in_bios_dir = dir
            .ancestors()
            .take_while(|a| !roots.contains(&Some(*a)))
            .filter_map(Path::file_name)
            .any(|n| {
                let n = n.to_string_lossy().to_ascii_lowercase();
                matches!(n.as_str(), "bios" | "00bios" | "system")
            });
        in_bios_dir || !is_root && !self.identified.contains(dir)
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

    /// Arcade sets: 1G1R per game (see [`crate::arcade::g1r`]); rejected sets go to the
    /// decision queue like console releases. Library copies come first and win ties; sets
    /// with the fewest fallback systems pick their slot first, so versions listed in one
    /// system only still get it.
    fn arcade(&mut self, sets: &[(&Item, &Game)]) {
        fn alt(it: &Item) -> &[Game] {
            match &it.files {
                Files::Set { alt, .. } => alt,
                _ => &[],
            }
        }
        let mut order: Vec<&(&Item, &Game)> = sets.iter().collect();
        order.sort_by_key(|(it, _)| (!it.in_library, alt(it).len()));
        let names: Vec<Vec<&str>> = order
            .iter()
            .map(|(it, g)| {
                std::iter::once(g.name.as_str())
                    .chain(alt(it).iter().map(|a| a.name.as_str()))
                    .collect()
            })
            .collect();
        let picks = if self.opts.rules.arcade_g1r {
            crate::arcade::g1r::select(&names, &self.opts.rules.regions)
        } else {
            (0..names.len()).map(|i| (None, i)).collect()
        };
        for (&&(it, g), (reason, best)) in order.iter().zip(picks) {
            let place = |b: &mut Self, why: Why| {
                b.why = why;
                let g = b.free_arcade_slot(it, g);
                b.release(&g.system, &[&(it, g)]);
            };
            let Some(reason) = reason else {
                let note = match &it.files {
                    Files::Set { dat_note, .. } => dat_note.clone(),
                    _ => String::new(),
                };
                place(self, Why::new(Rule::ArcadeSet, note));
                continue;
            };
            match self.opts.verdicts.get(&(g.system.clone(), g.name.clone())) {
                Some(Verdict::Keep) if reason != g1r::Reason::Duplicate => {
                    place(self, Why::new(Rule::Verdict, format!("kept ({reason:?})")));
                }
                Some(Verdict::Discard) => {
                    self.why = Why::new(Rule::Verdict, format!("discarded ({reason:?})"));
                    self.discard(it);
                }
                _ => self.plan.decisions.push(Decision::Rejected {
                    path: it.files.primary().clone(),
                    system: g.system.clone(),
                    name: g.name.clone(),
                    kept: Some(order[best].1.name.clone()),
                    reason: format!("{reason:?}"),
                }),
            }
        }
    }

    /// A multi-disc game already in place: keeps its files, gets its playlist entry.
    fn placed_set(&mut self, dir: &Path, g: &Game) {
        let Some(name) = dir.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            return;
        };
        self.plan.unchanged += 1;
        self.lpl
            .entry(g.system.clone())
            .or_default()
            .push(lpl::Entry {
                path: dir.join(format!("{name}.m3u")),
                label: name,
                crc: g.crc,
            });
    }

    /// The best system whose slot for this set's short name is free: another version under
    /// the same name may already take it (e.g. `gradius3.zip`, Japan vs World).
    fn free_arcade_slot<'g>(&self, it: &'g Item, g: &'g Game) -> &'g Game {
        let Files::Set { archive, alt, .. } = &it.files else {
            return g;
        };
        let free = |s: &str| {
            let t = self.library.join(s).join(file_name(archive));
            !self.claimed.contains_key(&t) && (!t.exists() || t == *archive)
        };
        std::iter::once(g)
            .chain(alt)
            .find(|c| free(&c.system))
            .unwrap_or(g)
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

    /// A file of the old `_quarantine` to `_trash/unknown/<path in quarantine>`.
    fn trash_quarantined(&mut self, p: &Path, quarantine: &Path) {
        let rel = p.strip_prefix(quarantine).unwrap_or(p);
        let to = self.library.join(TRASH_DIR).join(UNKNOWN_TRASH).join(rel);
        let it = Item {
            files: Files::Single(p.to_path_buf()),
            ident: Ident::Unknown,
            in_library: true,
        };
        let op = Op::Move {
            from: p.to_path_buf(),
            to,
        };
        if self.commit(&it, vec![op]) {
            self.plan.quarantined += 1;
        }
    }

    /// Frontend metadata to `_trash/frontend/<path in library or inbox>` (always a move).
    fn trash_meta(&mut self, it: &Item) {
        let f = it.files.primary();
        let to = self
            .library
            .join(TRASH_DIR)
            .join("frontend")
            .join(self.quarantine_rel(f));
        let op = Op::Move {
            from: f.clone(),
            to,
        };
        if self.commit(it, vec![op]) {
            self.plan.discarded += 1;
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
        }
    }

    /// The game name if every clashing op copies a file bit-identical to the one already at
    /// (or planned for) its target – the item is then a duplicate, not a conflict.
    fn identical_copy(&self, it: &Item, ops: &[Op]) -> Option<String> {
        let g = match &it.ident {
            Ident::Known(g) | Ident::Bios(g) => g,
            Ident::Firmware(paths) => paths.first()?,
            _ => return None,
        };
        let own: Vec<&Path> = ops.iter().filter_map(Op::source).collect();
        let all_same = ops.iter().all(|op| {
            let t = op.target();
            let other = match self.claimed.get(t) {
                Some(src) => src.as_deref(),
                None if t.exists() && !own.contains(&t) => Some(t),
                None => return true,
            };
            match (op, other) {
                (Op::Extract { .. } | Op::Write { .. }, _) | (_, None) => false,
                (_, Some(other)) => op.source().is_some_and(|s| same_content(s, other)),
            }
        });
        all_same.then(|| g.name.clone())
    }

    /// A bit-identical second copy: 1G1R duplicate (decision queue, or trashed if you said so).
    fn duplicate(&mut self, it: &Item, n_ops: usize, kept: String) {
        let g = match &it.ident {
            Ident::Known(g) | Ident::Bios(g) => g,
            Ident::Firmware(paths) if !paths.is_empty() => &paths[0],
            _ => return,
        };
        let key = (g.system.clone(), g.name.clone());
        // single files only; a duplicate game folder is left for you to remove
        if n_ops == 1 && self.opts.verdicts.get(&key) == Some(&Verdict::Discard) {
            self.why = Why::new(Rule::Verdict, "discarded (Duplicate)");
            self.discard(it);
            return;
        }
        self.plan.decisions.push(Decision::Rejected {
            path: it.files.primary().clone(),
            system: key.0,
            name: key.1,
            kept: Some(kept),
            reason: format!("{:?}", g1r::Reason::Duplicate),
        });
    }

    /// Adds an item's ops unless a target exists or is already claimed (then: conflict).
    fn commit(&mut self, it: &Item, ops: Vec<Op>) -> bool {
        let own: Vec<&Path> = ops.iter().filter_map(Op::source).collect();
        let clash = ops.iter().find(|op| {
            let t = op.target();
            let rewrite = matches!(op, Op::Write { .. });
            self.claimed.contains_key(t) && !rewrite
                || (t.exists() && !own.contains(&t) && !rewrite)
        });
        if let Some(op) = clash {
            if let Some(kept) = self.identical_copy(it, &ops) {
                self.duplicate(it, ops.len(), kept);
                return false;
            }
            self.plan.decisions.push(Decision::Conflict {
                path: it.files.primary().clone(),
                target: op.target().to_path_buf(),
            });
            return false;
        }
        self.claimed.extend(ops.iter().map(|op| {
            (
                op.target().to_path_buf(),
                op.source().map(Path::to_path_buf),
            )
        }));
        if let Some(archive) = it.files.archive() {
            *self.members_done.entry(archive.clone()).or_default() += 1;
        }
        let mut why = self.why.clone();
        if it.in_library && matches!(why.rule, Rule::G1rPick | Rule::ArcadeSet) {
            why.detail = match why.detail.as_str() {
                "" => "rename to naming scheme".into(),
                d => format!("{d}, rename to naming scheme"),
            };
        }
        self.plan.why.extend(ops.iter().map(|_| why.clone()));
        self.plan.ops.extend(ops);
        true
    }

    /// Writes a text file unless it already has exactly this content.
    fn write(&mut self, path: PathBuf, contents: String) {
        if fs::read_to_string(&path).is_ok_and(|old| old == contents) {
            return;
        }
        self.claimed.insert(path.clone(), None);
        self.plan.why.push(self.why.clone());
        self.plan.ops.push(Op::Write { path, contents });
    }

    fn playlists(&mut self) {
        let Some(dir) = self.opts.playlists.clone() else {
            return;
        };
        self.why = Why::new(Rule::Playlist, "");
        let mut written = HashSet::new();
        for (system, entries) in std::mem::take(&mut self.lpl) {
            let path = dir.join(format!("{}.lpl", naming::sanitize_file_name(&system)));
            written.insert(path.clone());
            self.write(path, lpl::render(&system, &entries));
        }
        // playlists of systems without any game left (all of it trashed or moved away)
        let Ok(rd) = fs::read_dir(&dir) else { return };
        let mut stale: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "lpl") && !written.contains(p))
            .collect();
        stale.sort();
        self.why = Why::new(Rule::Playlist, "no games left – playlist to the trash");
        for p in stale {
            let to = self
                .library
                .join(TRASH_DIR)
                .join("playlists")
                .join(file_name(&p));
            let it = Item {
                files: Files::Single(p.clone()),
                ident: Ident::Unknown,
                in_library: true,
            };
            self.commit(&it, vec![Op::Move { from: p, to }]);
        }
    }

    fn skip(&mut self, it: &Item, reason: String) {
        self.plan.decisions.push(Decision::Skipped {
            path: it.files.primary().clone(),
            reason,
        });
    }
}

/// Whether `dir` holds the `.m3u` named after itself (a placed multi-disc game).
fn has_own_m3u(dir: &Path) -> bool {
    dir.file_name()
        .is_some_and(|n| dir.join(format!("{}.m3u", n.to_string_lossy())).is_file())
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

fn is_zip(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip"))
}

/// Whether two files have the same content: zips with the same members (name, CRC), else
/// the same bytes (size first, then streamed comparison).
pub(crate) fn same_content(a: &Path, b: &Path) -> bool {
    use std::io::Read;
    if a == b {
        return true;
    }
    if is_zip(a) && is_zip(b) {
        // the same set packed by another tool: equal members count, not equal bytes
        let members = |p: &Path| {
            crate::archive::members(p).ok().map(|mut m| {
                for (n, _) in &mut m {
                    *n = n.to_ascii_lowercase();
                }
                m.sort();
                m
            })
        };
        if let (Some(ma), Some(mb)) = (members(a), members(b)) {
            return ma == mb;
        }
    }
    let (Ok(ma), Ok(mb)) = (fs::metadata(a), fs::metadata(b)) else {
        return false;
    };
    if ma.len() != mb.len() || !ma.is_file() || !mb.is_file() {
        return false;
    }
    let (Ok(mut fa), Ok(mut fb)) = (fs::File::open(a), fs::File::open(b)) else {
        return false;
    };
    let (mut ba, mut bb) = (vec![0u8; 1 << 16], vec![0u8; 1 << 16]);
    loop {
        let Ok(n) = fa.read(&mut ba) else {
            return false;
        };
        if n == 0 {
            return true;
        }
        if fb.read_exact(&mut bb[..n]).is_err() || ba[..n] != bb[..n] {
            return false;
        }
    }
}
