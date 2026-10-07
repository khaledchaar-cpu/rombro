//! Firmware from the inbox recognised by `System.dat` (size + SHA1): imported to
//! `_bios/<path>` under the name the cores expect, as long as it is still missing there.

use super::Builder;
use crate::plan::{BIOS_DIR, Game, Item, Op};
use crate::rules::{Rule, Why};
use std::path::{Path, PathBuf};

impl Builder<'_> {
    /// Places `it` at every listed path that is still free (cores look for different names);
    /// if every path is taken, the first one decides (bit-identical → duplicate, else conflict).
    pub(super) fn firmware(&mut self, it: &Item, paths: &[Game]) {
        let bios = self.library.join(BIOS_DIR);
        let targets: Vec<(PathBuf, &Game)> = paths
            .iter()
            .map(|g| (bios.join(&g.name), g))
            .filter(|(t, _)| !blocked_by_file(&bios, t))
            .collect();
        let mut free: Vec<&(PathBuf, &Game)> = targets
            .iter()
            .filter(|(t, _)| !t.exists() && !self.claimed.contains_key(t))
            .collect();
        if free.is_empty() {
            free.extend(targets.first());
        }
        let Some((last, rest)) = free.split_last() else {
            return;
        };
        let from = it.files.primary();
        self.why = Why::new(Rule::Bios, format!("System.dat: {}", last.1.system));
        // copies first: the last op may move the source away
        let mut ops: Vec<Op> = rest
            .iter()
            .map(|(to, _)| Op::Copy {
                from: from.clone(),
                to: to.clone(),
            })
            .collect();
        ops.push(self.transfer(it, from, &last.0));
        if self.commit(it, ops) {
            self.plan.placed += 1;
        }
    }
}

/// A path below `bios` whose parent folder exists as a file (`SGB1.sfc/…` next to a `SGB1.sfc`).
fn blocked_by_file(bios: &Path, t: &Path) -> bool {
    t.ancestors()
        .skip(1)
        .take_while(|a| *a != bios)
        .any(Path::is_file)
}
