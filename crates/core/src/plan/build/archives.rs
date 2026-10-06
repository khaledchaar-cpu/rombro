//! Multi-ROM archives: once every member is handled, the archive goes to the trash, or as a
//! whole into quarantine if it holds unknown members (e.g. arcade sets that belong together).

use super::{Builder, file_name};
use crate::naming;
use crate::plan::{Files, Game, Ident, Item, Mode, Op, TRASH_DIR, lpl};
use crate::rules::{Rule, Why};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Default)]
struct Count {
    members: usize,
    unknown: usize,
    in_library: bool,
}

impl Builder<'_> {
    /// Trashes fully extracted archives (unless the inbox is only copied/linked from) and
    /// quarantines archives whose remaining members are all unknown.
    pub(super) fn finish_archives(&mut self, items: &[&Item]) {
        let mut counts: BTreeMap<&Path, Count> = BTreeMap::new();
        for it in items {
            if let Some(archive) = it.files.archive() {
                let c = counts.entry(archive).or_default();
                c.members += 1;
                c.unknown += usize::from(matches!(it.ident, Ident::Unknown));
                c.in_library = it.in_library;
            }
        }
        for (archive, c) in counts {
            let done = self.members_done.get(archive).copied().unwrap_or(0);
            if done + c.unknown < c.members {
                continue;
            }
            let it = Item {
                files: Files::Single(archive.to_path_buf()),
                ident: Ident::Unknown,
                in_library: c.in_library,
            };
            if c.unknown > 0 && self.left_alone(archive) {
                continue;
            }
            if c.unknown > 0 {
                self.why = Why::new(
                    Rule::Quarantine,
                    format!("{} archive member(s) without database match", c.unknown),
                );
                self.quarantine(&it);
            } else if c.in_library || self.opts.mode == Mode::Move {
                self.why = Why::new(Rule::ArchiveExtracted, "");
                let op = Op::Move {
                    from: archive.to_path_buf(),
                    to: self.library.join(TRASH_DIR).join(file_name(archive)),
                };
                self.commit(&it, vec![op]);
            }
        }
    }
}

/// Whether an archive member is one disk/side of a multi-disk game (`(Disk 2)`, `(Side B)`).
fn is_disk_member(member: &str) -> bool {
    let n = member.to_ascii_lowercase();
    n.contains("(disk ") || n.contains("(side ")
}

/// Archives holding the disks of one game: every member identified, one system, at least
/// two members, all tagged as disks. The databases often name such disks inconsistently,
/// so they stay together under the archive's name instead of going through 1G1R one by one.
pub(super) fn disk_sets<'a>(items: &[&'a Item]) -> BTreeMap<&'a Path, Vec<(&'a Item, &'a Game)>> {
    let mut sets: BTreeMap<&Path, Vec<(&Item, Option<&Game>)>> = BTreeMap::new();
    for &it in items {
        if let Files::Member { archive, .. } = &it.files {
            let g = match &it.ident {
                Ident::Known(g) => Some(g),
                _ => None,
            };
            sets.entry(archive.as_path()).or_default().push((it, g));
        }
    }
    sets.into_iter()
        .filter_map(|(archive, ms)| {
            let ms: Vec<(&Item, &Game)> = ms
                .into_iter()
                .map(|(it, g)| g.map(|g| (it, g)))
                .collect::<Option<_>>()?;
            let one_system = ms.iter().all(|(_, g)| g.system == ms[0].1.system);
            let disks = ms.iter().all(|(it, _)| match &it.files {
                Files::Member { member, .. } => is_disk_member(member),
                _ => false,
            });
            (ms.len() >= 2 && one_system && disks).then_some((archive, ms))
        })
        .collect()
}

impl Builder<'_> {
    /// Extracts a multi-disk archive into `<System>/<archive name>/` (member names kept) with
    /// an `.m3u`; RetroArch lists the game once and swaps disks via the playlist.
    pub(super) fn disk_set(&mut self, archive: &Path, members: &[(&Item, &Game)]) {
        let Some(stem) = archive
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
        else {
            return;
        };
        let system = &members[0].1.system;
        let m3u = self.library.join(naming::playlist_path(system, &stem));
        let Some(dir) = m3u.parent().map(Path::to_path_buf) else {
            return;
        };
        self.why = Why::new(Rule::MultiDiskArchive, "");
        let mut text = String::new();
        let mut all = true;
        for (it, _) in members {
            let Files::Member { member, .. } = &it.files else {
                continue;
            };
            let name = naming::sanitize_file_name(&file_name(Path::new(member)));
            let op = self.transfer(it, Path::new(member), &dir.join(&name));
            all &= self.commit(it, vec![op]);
            text.push_str(&name);
            text.push('\n');
        }
        if all {
            self.write(m3u.clone(), text);
            self.plan.placed += 1;
        }
        self.lpl
            .entry(system.clone())
            .or_default()
            .push(lpl::Entry {
                path: m3u,
                label: naming::release_name(&stem),
                crc: members[0].1.crc,
            });
    }
}
