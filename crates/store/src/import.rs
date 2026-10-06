//! Incremental import of RetroArch RDB files into the `entry` table.

use crate::record::{Record, merge};
use crate::{Error, Result, Store};
use rayon::prelude::*;
use rombro_rdb::{Entry, RdbFile};
use rusqlite::params;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Outcome of [`Store::sync_rdbs`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SyncReport {
    pub imported: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub entries: usize,
    /// Metadata-only entries merged into a game entry.
    pub merged: usize,
    /// Metadata-only entries without a matching game entry (dropped).
    pub orphaned: usize,
}

struct SourceFile {
    path: PathBuf,
    system: String,
    mtime: i64,
    size: i64,
}

struct Parsed {
    records: Vec<Record>,
    merged: usize,
    orphaned: usize,
}

impl Store {
    /// Imports new or changed `.rdb` files from `dir`; drops sources that vanished.
    pub fn sync_rdbs(&mut self, dir: impl AsRef<Path>) -> Result<SyncReport> {
        self.sync_rdbs_progress(dir, &|_, _| {})
    }

    /// [`Self::sync_rdbs`], reporting (parsed, to parse) RDB files to `progress`.
    pub fn sync_rdbs_progress(
        &mut self,
        dir: impl AsRef<Path>,
        progress: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<SyncReport> {
        let files = scan_dir(dir.as_ref())?;
        let known: HashMap<String, (i64, i64, i64)> = {
            let mut st = self
                .conn
                .prepare("SELECT path, id, mtime, size FROM rdb_source")?;
            st.query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?, r.get(3)?))))?
                .collect::<rusqlite::Result<_>>()?
        };

        let mut report = SyncReport::default();
        let mut stale: Vec<i64> = Vec::new();
        let mut todo: Vec<&SourceFile> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for f in &files {
            let key = f.path.to_string_lossy().into_owned();
            match known.get(&key) {
                Some(&(_, m, s)) if m == f.mtime && s == f.size => report.unchanged += 1,
                Some(&(id, ..)) => {
                    stale.push(id);
                    todo.push(f);
                }
                None => todo.push(f),
            }
            seen.insert(key);
        }
        for (path, &(id, ..)) in &known {
            if !seen.contains(path) {
                stale.push(id);
                report.removed += 1;
            }
        }

        let done = std::sync::atomic::AtomicUsize::new(0);
        progress(0, todo.len());
        let parsed = todo
            .par_iter()
            .map(|f| {
                let p = parse_file(f);
                let n = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                progress(n, todo.len());
                p
            })
            .collect::<Result<Vec<_>>>()?;

        let tx = self.conn.transaction()?;
        // Rebuilding indexes once is far cheaper than maintaining them per insert.
        let bulk = todo.len() > 4;
        if bulk {
            tx.execute_batch(crate::schema::DROP_ENTRY_INDEXES)?;
        }
        for id in stale {
            tx.execute("DELETE FROM rdb_source WHERE id = ?1", [id])?;
        }
        {
            let mut src = tx.prepare(
                "INSERT INTO rdb_source (path, system, mtime, size) VALUES (?1, ?2, ?3, ?4)",
            )?;
            let mut ins = tx.prepare(
                "INSERT INTO entry (source_id, system, name, description, rom_name, size, crc, md5,
                 sha1, serial, region, genre, developer, publisher, franchise, release_year,
                 release_month, users)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
            )?;
            for (f, p) in todo.iter().zip(&parsed) {
                src.execute(params![f.path.to_string_lossy(), f.system, f.mtime, f.size])?;
                let sid = tx.last_insert_rowid();
                for r in &p.records {
                    ins.execute(params![
                        sid,
                        r.system,
                        r.name,
                        r.description,
                        r.rom_name,
                        r.size,
                        r.crc,
                        r.md5,
                        r.sha1,
                        r.serial,
                        r.region,
                        r.genre,
                        r.developer,
                        r.publisher,
                        r.franchise,
                        r.release_year,
                        r.release_month,
                        r.users
                    ])?;
                }
                report.entries += p.records.len();
                report.merged += p.merged;
                report.orphaned += p.orphaned;
            }
        }
        if bulk {
            tx.execute_batch(crate::schema::ENTRY_INDEXES)?;
        }
        tx.commit()?;
        report.imported = todo.len();
        Ok(report)
    }
}

fn scan_dir(dir: &Path) -> Result<Vec<SourceFile>> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir)? {
        let path = e?.path();
        if path.extension().is_none_or(|x| x != "rdb") {
            continue;
        }
        let md = std::fs::metadata(&path)?;
        let mtime = md
            .modified()?
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as i64);
        let system = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        out.push(SourceFile {
            path,
            system,
            mtime,
            size: md.len() as i64,
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn parse_file(f: &SourceFile) -> Result<Parsed> {
    let label = || f.path.display().to_string();
    let rdb = RdbFile::open(&f.path).map_err(|e| Error::Rdb(label(), e))?;
    let all = rdb
        .entries()
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::Rdb(label(), e))?;
    Ok(merge_entries(&f.system, all))
}

/// Merges metadata-only entries into game entries (by serial, then crc) and converts to records.
fn merge_entries(system: &str, all: Vec<Entry<'_>>) -> Parsed {
    let (games, metas): (Vec<_>, Vec<_>) = all.into_iter().partition(|e| !e.is_metadata_only());
    let mut by_serial: HashMap<&str, usize> = HashMap::new();
    let mut by_crc: HashMap<u32, usize> = HashMap::new();
    for (i, m) in metas.iter().enumerate() {
        if let Some(s) = m.serial {
            by_serial.entry(s).or_insert(i);
        }
        if let Some(c) = m.crc32() {
            by_crc.entry(c).or_insert(i);
        }
    }
    let mut used = vec![false; metas.len()];
    let records = games
        .into_iter()
        .map(|mut g| {
            let hit = g
                .serial
                .and_then(|s| by_serial.get(s))
                .or_else(|| g.crc32().and_then(|c| by_crc.get(&c)))
                .copied();
            if let Some(i) = hit {
                merge(&mut g, &metas[i]);
                used[i] = true;
            }
            to_record(system, &g)
        })
        .collect();
    let merged = used.iter().filter(|u| **u).count();
    Parsed {
        records,
        merged,
        orphaned: metas.len() - merged,
    }
}

fn to_record(system: &str, e: &Entry<'_>) -> Record {
    let s = |v: Option<&str>| v.map(str::to_owned);
    Record {
        id: 0,
        system: system.to_owned(),
        name: e.name.unwrap_or_default().to_owned(),
        description: s(e.description),
        rom_name: s(e.rom_name),
        size: e.size,
        crc: e.crc32(),
        md5: e.md5.map(<[u8]>::to_vec),
        sha1: e.sha1.map(<[u8]>::to_vec),
        serial: s(e.serial),
        region: s(e.region),
        genre: s(e.genre),
        developer: s(e.developer),
        publisher: s(e.publisher),
        franchise: s(e.franchise),
        release_year: e.release_year,
        release_month: e.release_month,
        users: e.users,
    }
}
