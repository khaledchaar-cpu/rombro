//! The `_trash` folder: releases the user discarded. Emptying it is the only irreversible action.

use super::TRASH_DIR;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Files in `<library>/_trash` with their sizes, sorted by path (empty if the folder is missing).
pub fn list(library: &Path) -> io::Result<Vec<(PathBuf, u64)>> {
    let mut out = Vec::new();
    let dir = library.join(TRASH_DIR);
    if dir.is_dir() {
        walk(&dir, &mut out)?;
    }
    out.sort();
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<(PathBuf, u64)>) -> io::Result<()> {
    for e in fs::read_dir(dir)? {
        let e = e?;
        let meta = e.metadata()?;
        if meta.is_dir() {
            walk(&e.path(), out)?;
        } else {
            out.push((e.path(), meta.len()));
        }
    }
    Ok(())
}

/// Permanently deletes `<library>/_trash`; returns the number of deleted files.
pub fn empty(library: &Path) -> io::Result<usize> {
    let n = list(library)?.len();
    let dir = library.join(TRASH_DIR);
    if dir.is_dir() {
        fs::remove_dir_all(dir)?;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_and_empties_trash() {
        let tmp = tempfile::TempDir::new().unwrap();
        let lib = tmp.path();
        assert!(list(lib).unwrap().is_empty());
        assert_eq!(empty(lib).unwrap(), 0);
        fs::create_dir_all(lib.join("_trash/sub")).unwrap();
        fs::write(lib.join("_trash/a.sfc"), "abc").unwrap();
        fs::write(lib.join("_trash/sub/b.bin"), "x").unwrap();
        fs::write(lib.join("keep.sfc"), "k").unwrap();
        let l = list(lib).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l[0], (lib.join("_trash/a.sfc"), 3));
        assert_eq!(empty(lib).unwrap(), 2);
        assert!(!lib.join("_trash").exists());
        assert!(lib.join("keep.sfc").exists());
    }
}
