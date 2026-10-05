//! Multi-ROM archives: once every member is extracted, the archive itself goes to the trash.

use super::{Builder, file_name};
use crate::plan::{Files, Ident, Item, Mode, Op, TRASH_DIR};
use std::collections::BTreeMap;
use std::path::Path;

impl Builder<'_> {
    /// Moves multi-ROM archives to the trash once every member was handled, unless the
    /// inbox is only copied/linked from.
    pub(super) fn trash_emptied_archives(&mut self, items: &[&Item]) {
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
}
