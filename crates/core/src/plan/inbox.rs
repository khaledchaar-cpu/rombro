//! What a plan leaves in the inbox, and clearing it: leftovers move to
//! `<library>/_trash/inbox-<stamp>/` (undoable; only emptying the trash deletes them).

use super::{Op, Plan, TRASH_DIR};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Files under `inbox` (with sizes, sorted) that no operation of `plan` takes from there:
/// unknown files the planner leaves alone, launcher stubs, frontend media, empty files.
pub fn leftovers(inbox: &Path, plan: &Plan) -> io::Result<Vec<(PathBuf, u64)>> {
    let used: HashSet<&Path> = plan.ops.iter().filter_map(Op::source).collect();
    let mut all = Vec::new();
    walk(inbox, &mut all)?;
    all.retain(|(p, _)| !used.contains(p.as_path()));
    all.sort();
    Ok(all)
}

/// Moves of `files` (inside `inbox`) to `<library>/_trash/inbox-<stamp>/<path in inbox>`;
/// files gone since planning are skipped.
pub fn clear_ops(inbox: &Path, library: &Path, stamp: &str, files: &[PathBuf]) -> Vec<Op> {
    let dest = library.join(TRASH_DIR).join(format!("inbox-{stamp}"));
    files
        .iter()
        .filter(|f| f.is_file())
        .filter_map(|f| {
            let rel = f.strip_prefix(inbox).ok()?;
            Some(Op::Move {
                from: f.clone(),
                to: dest.join(rel),
            })
        })
        .collect()
}

/// Removes empty folders below `inbox` (never `inbox` itself); returns how many.
pub fn prune_empty_dirs(inbox: &Path) -> usize {
    fn prune(dir: &Path) -> usize {
        let Ok(rd) = fs::read_dir(dir) else {
            return 0;
        };
        let mut n = 0;
        for e in rd.flatten() {
            let p = e.path();
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                n += prune(&p);
                if fs::remove_dir(&p).is_ok() {
                    n += 1;
                }
            }
        }
        n
    }
    prune(inbox)
}

fn walk(dir: &Path, out: &mut Vec<(PathBuf, u64)>) -> io::Result<()> {
    for e in fs::read_dir(dir)? {
        let e = e?;
        let t = e.file_type()?;
        if t.is_dir() {
            walk(&e.path(), out)?;
        } else if t.is_file() {
            out.push((e.path(), e.metadata()?.len()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_leftovers_and_clears_them_undoably() {
        let tmp = tempfile::tempdir().unwrap();
        let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
        fs::create_dir_all(inbox.join("sdlpop/images")).unwrap();
        fs::write(inbox.join("game.sfc"), b"rom").unwrap();
        fs::write(inbox.join("sdlpop/PrinceOfPersia.sdlpop"), b"").unwrap();
        fs::write(inbox.join("sdlpop/images/box.png"), b"png").unwrap();
        let plan = Plan {
            ops: vec![Op::Move {
                from: inbox.join("game.sfc"),
                to: lib.join("SNES/game.sfc"),
            }],
            ..Plan::default()
        };
        let left = leftovers(&inbox, &plan).unwrap();
        let files: Vec<PathBuf> = left.iter().map(|(p, _)| p.clone()).collect();
        assert_eq!(
            files,
            [
                inbox.join("sdlpop/PrinceOfPersia.sdlpop"),
                inbox.join("sdlpop/images/box.png")
            ]
        );

        let ops = clear_ops(&inbox, &lib, "1", &files);
        let r = crate::plan::execute(&ops);
        assert!(r.error.is_none());
        assert_eq!(prune_empty_dirs(&inbox), 2);
        assert!(inbox.is_dir() && !inbox.join("sdlpop").exists());
        assert!(lib.join("_trash/inbox-1/sdlpop/images/box.png").is_file());

        assert!(crate::plan::undo(&r.done).is_empty());
        assert!(inbox.join("sdlpop/images/box.png").is_file());
    }

    #[test]
    fn prunes_only_folders_emptied_by_the_moves() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path().join("lib");
        let src = lib.join("Quake/quake/images/a.png");
        fs::create_dir_all(src.parent().unwrap()).unwrap();
        fs::create_dir_all(lib.join("Untouched/empty")).unwrap();
        fs::create_dir_all(lib.join("Quake/tyrquake")).unwrap();
        fs::write(&src, b"png").unwrap();
        let ops = [Op::Move {
            from: src.clone(),
            to: lib.join("_trash/a.png"),
        }];
        let r = crate::plan::execute(&ops);
        // images/ and quake/ emptied; Quake/ keeps tyrquake, Untouched/empty is not ours
        assert_eq!(crate::plan::prune_emptied(&r.done, &[lib.as_path()]), 2);
        assert!(!lib.join("Quake/quake").exists());
        assert!(lib.join("Quake/tyrquake").is_dir());
        assert!(lib.join("Untouched/empty").is_dir());
        assert!(crate::plan::undo(&r.done).is_empty());
        assert!(src.is_file());
    }
}
