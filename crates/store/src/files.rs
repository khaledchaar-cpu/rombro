//! Persistent file index: hash cache for incremental scans of the library (and inbox),
//! kept up to date by execute/undo so moved files are never rehashed.

use crate::{Result, Store};
use rombro_core::plan::{Done, Op};
use rombro_core::{CachedRom, HashCache, ScanReport, Stamp};
use rusqlite::{OptionalExtension, params};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const LIBRARY_KEY: &str = "library";

fn key(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// `root` as a path prefix ending in a separator.
fn prefix(root: &Path) -> String {
    let mut s = key(root);
    if !s.ends_with(std::path::MAIN_SEPARATOR) {
        s.push(std::path::MAIN_SEPARATOR);
    }
    s
}

impl Store {
    pub fn setting(&self, name: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [name], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn set_setting(&self, name: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![name, value],
        )?;
        Ok(())
    }

    pub fn library(&self) -> Result<Option<PathBuf>> {
        Ok(self.setting(LIBRARY_KEY)?.map(PathBuf::from))
    }

    pub fn set_library(&self, path: &Path) -> Result<()> {
        self.set_setting(LIBRARY_KEY, &key(path))
    }

    /// Cached hashes of all indexed files below `root`.
    pub fn hash_cache(&self, root: &Path) -> Result<HashCache> {
        let pre = prefix(root);
        let mut stmt = self.conn.prepare_cached(
            "SELECT path, size, mtime, roms FROM file WHERE substr(path, 1, ?2) = ?1",
        )?;
        let rows = stmt.query_map(params![pre, pre.chars().count() as i64], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        let mut map = HashMap::new();
        for row in rows {
            let (path, size, mtime, roms) = row?;
            let roms: Vec<CachedRom> = serde_json::from_str(&roms)?;
            let stamp = Stamp {
                size: size as u64,
                mtime,
            };
            map.insert(PathBuf::from(path), (stamp, roms));
        }
        Ok(HashCache(map))
    }

    /// Replaces the index below `root` with the files of `report` (vanished files drop out).
    pub fn save_scan(&self, root: &Path, report: &ScanReport) -> Result<()> {
        let mut files: Vec<(&Path, Vec<CachedRom>)> = Vec::new();
        for group in report.roms.chunk_by(|a, b| a.path == b.path) {
            files.push((
                &group[0].path,
                group.iter().map(CachedRom::from_rom).collect(),
            ));
        }
        for t in report.discs.iter().flat_map(|d| &d.tracks) {
            files.push((&t.path, vec![CachedRom::from_rom(t)]));
        }
        let tx = self.conn.unchecked_transaction()?;
        let pre = prefix(root);
        tx.execute(
            "DELETE FROM file WHERE substr(path, 1, ?2) = ?1",
            params![pre, pre.chars().count() as i64],
        )?;
        {
            let mut ins = tx.prepare_cached(
                "INSERT OR REPLACE INTO file (path, size, mtime, roms) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for (path, roms) in files {
                let Ok(st) = Stamp::of(path) else { continue };
                let json = serde_json::to_string(&roms)?;
                ins.execute(params![key(path), st.size as i64, st.mtime, json])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Carries index rows along with executed file operations.
    pub fn index_executed(&self, done: &[Done]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for d in done {
            match &d.op {
                Op::Move { from, to } => self.relocate(from, to, true)?,
                Op::Copy { from, to } | Op::Hardlink { from, to } => {
                    self.relocate(from, to, false)?
                }
                Op::Write { .. } => {}
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Reverts [`Store::index_executed`] after an undo.
    pub fn index_undone(&self, done: &[Done]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for d in done.iter().rev() {
            match &d.op {
                Op::Move { from, to } => self.relocate(to, from, true)?,
                Op::Copy { to, .. } | Op::Hardlink { to, .. } => {
                    self.conn
                        .execute("DELETE FROM file WHERE path = ?1", [key(to)])?;
                }
                Op::Write { .. } => {}
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Re-keys (or copies) the row of `from` to `to` with `to`'s current stamp.
    fn relocate(&self, from: &Path, to: &Path, remove: bool) -> Result<()> {
        let roms: Option<String> = self
            .conn
            .query_row("SELECT roms FROM file WHERE path = ?1", [key(from)], |r| {
                r.get(0)
            })
            .optional()?;
        if remove {
            self.conn
                .execute("DELETE FROM file WHERE path = ?1", [key(from)])?;
        }
        if let (Some(roms), Ok(st)) = (roms, Stamp::of(to)) {
            self.conn.execute(
                "INSERT OR REPLACE INTO file (path, size, mtime, roms) VALUES (?1, ?2, ?3, ?4)",
                params![key(to), st.size as i64, st.mtime, roms],
            )?;
        }
        Ok(())
    }
}
