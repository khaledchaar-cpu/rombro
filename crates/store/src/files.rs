//! Persistent file index: hash cache for incremental scans of the library (and inbox),
//! kept up to date by execute/undo so moved files are never rehashed.

use crate::{Result, Store};
use rombro_core::plan::{Done, Op};
use rombro_core::{CachedDisc, CachedRom, HashCache, ScanReport, ScannedRom, Stamp};
use rusqlite::{OptionalExtension, params};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const LIBRARY_KEY: &str = "library";

pub(crate) fn key(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// `root` as a path prefix ending in a separator.
pub(crate) fn prefix(root: &Path) -> String {
    let mut s = key(root);
    if !s.ends_with(std::path::MAIN_SEPARATOR) {
        s.push(std::path::MAIN_SEPARATOR);
    }
    s
}

const RULES_KEY: &str = "rules";
const IGNORE_KEY: &str = "ignore";
const HITS_KEY: &str = "rule_hits";

pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
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

    /// Stored 1G1R rules (setting `rules`, JSON), or the defaults when none or unreadable.
    pub fn rules(&self) -> Result<rombro_core::g1r::Rules> {
        Ok(self
            .setting(RULES_KEY)?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default())
    }

    pub fn set_rules(&self, rules: &rombro_core::g1r::Rules) -> Result<()> {
        let json = serde_json::to_string(rules).unwrap_or_default();
        self.set_setting(RULES_KEY, &json)
    }

    /// Paths the user excluded from planning (setting `ignore`, JSON list).
    pub fn ignored(&self) -> Result<Vec<PathBuf>> {
        Ok(self
            .setting(IGNORE_KEY)?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default())
    }

    pub fn set_ignored(&self, paths: &[PathBuf]) -> Result<()> {
        let json = serde_json::to_string(paths).unwrap_or_default();
        self.set_setting(IGNORE_KEY, &json)
    }

    /// Planned ops per rule id from the last plan (setting `rule_hits`).
    pub fn rule_hits(&self) -> Result<std::collections::BTreeMap<String, usize>> {
        Ok(self
            .setting(HITS_KEY)?
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default())
    }

    pub fn set_rule_hits(&self, why: &[rombro_core::rules::Why]) -> Result<()> {
        let json = serde_json::to_string(&rombro_core::rules::hits(why)).unwrap_or_default();
        self.set_setting(HITS_KEY, &json)
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
        Ok(HashCache::new(map))
    }

    /// Replaces the index below `root` with the files of `report` (vanished files drop out).
    /// With `trusted` (the scan used a trusted cache) rows whose hashes are unchanged are
    /// kept as they are instead of re-reading each file's stamp.
    pub fn save_scan(&self, root: &Path, report: &ScanReport, trusted: bool) -> Result<()> {
        let mut files: Vec<(&Path, Vec<CachedRom>)> = Vec::new();
        let whole: HashMap<&Path, &ScannedRom> = report
            .archives
            .iter()
            .map(|a| (a.path.as_path(), a))
            .collect();
        for group in report.roms.chunk_by(|a, b| a.path == b.path) {
            // Archives are kept with their whole-file hash so the cache can serve both.
            let own = whole.get(group[0].path.as_path()).copied();
            files.push((
                &group[0].path,
                own.into_iter()
                    .chain(group)
                    .map(CachedRom::from_rom)
                    .collect(),
            ));
        }
        for d in &report.discs {
            for (i, t) in d.tracks.iter().enumerate() {
                let mut c = CachedRom::from_rom(t);
                if i == 0 {
                    c.disc = Some(CachedDisc { id: d.id.clone() });
                }
                files.push((&t.path, vec![c]));
            }
        }
        let pre = prefix(root);
        let tx = rusqlite::Transaction::new_unchecked(
            &self.conn,
            rusqlite::TransactionBehavior::Immediate,
        )?;
        let old: HashMap<String, String> = tx
            .prepare_cached("SELECT path, roms FROM file WHERE substr(path, 1, ?2) = ?1")?
            .query_map(params![pre, pre.chars().count() as i64], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let now = now();
        let mut seen = std::collections::HashSet::new();
        let mut changed = false;
        {
            // existing rows keep their `added` time
            let mut up = tx.prepare_cached(
                "INSERT INTO file (path, size, mtime, roms, added) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(path) DO UPDATE SET size = ?2, mtime = ?3, roms = ?4",
            )?;
            for (path, roms) in files {
                let json = serde_json::to_string(&roms)?;
                let k = key(path);
                if trusted && old.get(&k) == Some(&json) {
                    seen.insert(k);
                    continue;
                }
                let Ok(st) = Stamp::of(path) else { continue };
                if old.get(&k) != Some(&json) {
                    changed = true;
                }
                up.execute(params![k, st.size as i64, st.mtime, json, now])?;
                seen.insert(k);
            }
            let mut del = tx.prepare_cached("DELETE FROM file WHERE path = ?1")?;
            for p in old.keys().filter(|p| !seen.contains(*p)) {
                changed = true;
                del.execute([p])?;
            }
        }
        if changed {
            self.drop_snapshots(root)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// `added` time per indexed file below `root` (rows from before v5 have none).
    pub fn added_times(&self, root: &Path) -> Result<HashMap<PathBuf, i64>> {
        let pre = prefix(root);
        let mut stmt = self.conn.prepare_cached(
            "SELECT path, added FROM file WHERE added IS NOT NULL AND substr(path, 1, ?2) = ?1",
        )?;
        let rows = stmt.query_map(params![pre, pre.chars().count() as i64], |r| {
            Ok((PathBuf::from(r.get::<_, String>(0)?), r.get(1)?))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Carries index rows along with executed file operations.
    pub fn index_executed(&self, done: &[Done]) -> Result<()> {
        let tx = rusqlite::Transaction::new_unchecked(
            &self.conn,
            rusqlite::TransactionBehavior::Immediate,
        )?;
        for d in done {
            match &d.op {
                Op::Move { from, to } => self.relocate(from, to, true)?,
                Op::Copy { from, to } => self.relocate(from, to, false)?,
                Op::Extract {
                    archive,
                    member,
                    to,
                } => self.index_member(archive, member, to)?,
                Op::Write { .. } => {}
            }
            self.mark_touched(&d.op)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Marks the paths an executed (or undone) op touched as dirty in their snapshots.
    fn mark_touched(&self, op: &Op) -> Result<()> {
        let paths: Vec<&Path> = match op {
            Op::Move { from, to } | Op::Copy { from, to } => vec![from, to],
            Op::Extract { archive, to, .. } => vec![archive, to],
            Op::Write { path, .. } => vec![path],
        };
        for p in paths {
            self.mark_dirty(p)?;
        }
        Ok(())
    }

    /// Indexes a file extracted from an archive with the member's cached hashes.
    fn index_member(&self, archive: &Path, member: &str, to: &Path) -> Result<()> {
        let roms: Option<String> = self
            .conn
            .query_row(
                "SELECT roms FROM file WHERE path = ?1",
                [key(archive)],
                |r| r.get(0),
            )
            .optional()?;
        let Some(roms) = roms else { return Ok(()) };
        let roms: Vec<CachedRom> = serde_json::from_str(&roms)?;
        let (Some(mut rom), Ok(st)) = (
            roms.into_iter()
                .find(|r| r.member.as_deref() == Some(member)),
            Stamp::of(to),
        ) else {
            return Ok(());
        };
        rom.member = None;
        self.conn.execute(
            "INSERT OR REPLACE INTO file (path, size, mtime, roms, added)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                key(to),
                st.size as i64,
                st.mtime,
                serde_json::to_string(&[rom])?,
                Some(now())
            ],
        )?;
        Ok(())
    }

    /// Reverts [`Store::index_executed`] after an undo.
    pub fn index_undone(&self, done: &[Done]) -> Result<()> {
        let tx = rusqlite::Transaction::new_unchecked(
            &self.conn,
            rusqlite::TransactionBehavior::Immediate,
        )?;
        for d in done.iter().rev() {
            match &d.op {
                Op::Move { from, to } => self.relocate(to, from, true)?,
                Op::Copy { to, .. } | Op::Extract { to, .. } => {
                    self.conn
                        .execute("DELETE FROM file WHERE path = ?1", [key(to)])?;
                }
                Op::Write { .. } => {}
            }
            self.mark_touched(&d.op)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Re-keys (or copies) the row of `from` to `to` with `to`'s current stamp.
    /// Keeps `added` for moves inside the library; arrivals from elsewhere are added now.
    fn relocate(&self, from: &Path, to: &Path, remove: bool) -> Result<()> {
        let row: Option<(String, Option<i64>)> = self
            .conn
            .query_row(
                "SELECT roms, added FROM file WHERE path = ?1",
                [key(from)],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let inside = self
            .library()?
            .is_some_and(|l| from.starts_with(&l) && to.starts_with(&l));
        if remove {
            self.conn
                .execute("DELETE FROM file WHERE path = ?1", [key(from)])?;
        }
        if let (Some((roms, added)), Ok(st)) = (row, Stamp::of(to)) {
            let added = if inside { added } else { Some(now()) };
            self.conn.execute(
                "INSERT OR REPLACE INTO file (path, size, mtime, roms, added)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![key(to), st.size as i64, st.mtime, roms, added],
            )?;
        }
        Ok(())
    }
}
