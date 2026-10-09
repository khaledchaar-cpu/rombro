//! Placing one picked item at its library path (incl. disc sheets and archived discs).

use super::{Builder, TRASH_DIR, ext_of, file_name};
use crate::plan::sheet::rewrite_sheet;
use crate::plan::{Files, Game, Item, Mode, Op};
use crate::{archive, disc, naming};
use std::path::{Path, PathBuf};

impl Builder<'_> {
    /// Ops moving one item to its library path; returns the new primary path.
    pub(super) fn place(&mut self, it: &Item, g: &Game, multi: bool) -> Option<PathBuf> {
        let target = |src: &Path| {
            if matches!(it.files, Files::Set { .. }) {
                // Emulators find romsets only by their short name.
                return self.library.join(&g.system).join(file_name(src));
            }
            let rel = naming::target_path(&g.system, &g.name, &ext_of(src), multi);
            self.library.join(rel)
        };
        // Cores open `.m3u` entries themselves, without RetroArch's archive support.
        let zipped = multi.then(|| single_zip_member(&it.files)).flatten();
        let primary = match &zipped {
            Some(m) => target(Path::new(m)),
            None => target(&name_source(&it.files)),
        };
        let (mut ops, sheet) = match &it.files {
            Files::Sheet { sheet, tracks } => (
                vec![self.transfer(it, sheet, &primary)],
                Some((disc::read_text(sheet), tracks.clone())),
            ),
            Files::ArchivedSheet {
                archive,
                sheet,
                tracks,
            } => (
                Vec::new(),
                Some((
                    archive::read_member(archive, sheet)
                        .map(|b| String::from_utf8_lossy(&b).into_owned()),
                    tracks.iter().map(PathBuf::from).collect(),
                )),
            ),
            Files::Set { archive, chds, .. } => {
                let dir = primary.with_extension("");
                let mut ops = vec![self.transfer(it, archive, &primary)];
                ops.extend(
                    chds.iter()
                        .map(|c| self.transfer(it, c, &dir.join(file_name(c)))),
                );
                (ops, None)
            }
            Files::Single(zip) if zipped.is_some() => {
                let mut ops = vec![Op::Extract {
                    archive: zip.clone(),
                    member: zipped.clone().unwrap_or_default(),
                    to: primary.clone(),
                }];
                if it.in_library || self.opts.mode == Mode::Move {
                    ops.push(Op::Move {
                        from: zip.clone(),
                        to: self.free_trash_target(&self.library.join(TRASH_DIR), zip),
                    });
                }
                (ops, None)
            }
            f => (vec![self.transfer(it, &name_source(f), &primary)], None),
        };
        if let Some((text, tracks)) = sheet {
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
            let text = match text {
                Ok(t) => t,
                Err(e) => {
                    self.skip(it, format!("cannot read sheet: {e}"));
                    return None;
                }
            };
            let new = rewrite_sheet(&text, &renames);
            if new != text || it.files.archive().is_some() {
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
}

/// The path whose name/extension the library file takes (a member's own name for archives).
fn name_source(files: &Files) -> PathBuf {
    match files {
        Files::Member { member, .. } | Files::ArchivedSheet { sheet: member, .. } => {
            PathBuf::from(member)
        }
        f => f.primary().clone(),
    }
}

/// The only member of a single-ROM `.zip` (`None` for anything else).
fn single_zip_member(files: &Files) -> Option<String> {
    let Files::Single(p) = files else {
        return None;
    };
    if !ext_of(p).eq_ignore_ascii_case("zip") {
        return None;
    }
    match archive::members(p).ok()?.as_slice() {
        [(m, _)] => Some(m.clone()),
        _ => None,
    }
}

/// Files to put into quarantine (member names for archived items).
pub(super) fn quarantine_sources(files: &Files) -> Vec<PathBuf> {
    match files {
        Files::Member { member, .. } => vec![PathBuf::from(member)],
        Files::ArchivedSheet { sheet, tracks, .. } => std::iter::once(sheet)
            .chain(tracks)
            .map(PathBuf::from)
            .collect(),
        f => f.all().into_iter().cloned().collect(),
    }
}
