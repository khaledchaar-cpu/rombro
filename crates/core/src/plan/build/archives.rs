//! Multi-ROM archives: once every member is handled, the archive goes to the trash, or as a
//! whole into quarantine if it holds unknown members (e.g. arcade sets that belong together).

use super::{Builder, file_name};
use crate::plan::{Files, Ident, Item, Mode, Op, TRASH_DIR};
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
                self.why = format!(
                    "unknown: {} archive member(s) without database match",
                    c.unknown
                );
                self.quarantine(&it);
            } else if c.in_library || self.opts.mode == Mode::Move {
                self.why = "archive fully extracted".into();
                let op = Op::Move {
                    from: archive.to_path_buf(),
                    to: self.library.join(TRASH_DIR).join(file_name(archive)),
                };
                self.commit(&it, vec![op]);
            }
        }
    }
}
