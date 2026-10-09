//! Identified library per root, so planning and the library view need neither a folder walk
//! nor a database lookup while nothing changed. Dropped (by triggers) when the game
//! databases or resolutions change and when a full scan finds changes; files that Romburak's own
//! runs move are only marked dirty and re-identified one by one on the next load.

use crate::{Result, Store};
use romburak_core::plan::Item;
use rusqlite::{OptionalExtension, params};
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    /// Library items as [`Store::items`] returns them.
    pub items: Vec<Item>,
    /// Arcade set names in the library ([`crate::set_names`] of its scan).
    pub sets: Vec<String>,
}

impl Store {
    /// The stored snapshot of `root`, if it is still current.
    pub fn snapshot(&self, root: &Path) -> Result<Option<Snapshot>> {
        let data: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT data FROM snapshot WHERE root = ?1",
                [root.to_string_lossy()],
                |r| r.get(0),
            )
            .optional()?;
        let Some(data) = data else { return Ok(None) };
        let mut json = Vec::new();
        flate2::read::GzDecoder::new(data.as_slice()).read_to_end(&mut json)?;
        // an unreadable snapshot (older format) is simply rebuilt
        Ok(serde_json::from_slice(&json).ok())
    }

    pub fn save_snapshot(&self, root: &Path, snap: &Snapshot) -> Result<()> {
        // serialize first: many tiny writes straight into the encoder are slow
        let json = serde_json::to_vec(snap)?;
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(&json)?;
        let data = gz.finish()?;
        // replacing the row fires the delete trigger: the dirty list goes with the old data
        self.conn.execute(
            "INSERT OR REPLACE INTO snapshot (root, data) VALUES (?1, ?2)",
            params![root.to_string_lossy(), data],
        )?;
        self.conn.execute(
            "DELETE FROM snapshot_dirty WHERE root = ?1",
            [root.to_string_lossy()],
        )?;
        Ok(())
    }

    /// Files below `root` changed since its snapshot was taken (see [`Store::mark_dirty`]).
    pub fn snapshot_dirty(&self, root: &Path) -> Result<Vec<std::path::PathBuf>> {
        let rows = self
            .conn
            .prepare_cached("SELECT path FROM snapshot_dirty WHERE root = ?1")?
            .query_map([root.to_string_lossy()], |r| r.get::<_, String>(0))?
            .map(|p| p.map(std::path::PathBuf::from))
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// Marks `path` as changed in every snapshot below whose root it lies; a snapshot whose
    /// root lies inside `path` is dropped.
    pub(crate) fn mark_dirty(&self, path: &Path) -> Result<()> {
        for root in self.snapshot_roots()? {
            let r = Path::new(&root);
            if path.starts_with(r) {
                self.conn.execute(
                    "INSERT OR IGNORE INTO snapshot_dirty (root, path) VALUES (?1, ?2)",
                    params![root, path.to_string_lossy()],
                )?;
            } else if r.starts_with(path) {
                self.conn
                    .execute("DELETE FROM snapshot WHERE root = ?1", [&root])?;
            }
        }
        Ok(())
    }

    fn snapshot_roots(&self) -> Result<Vec<String>> {
        Ok(self
            .conn
            .prepare_cached("SELECT root FROM snapshot")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    }

    /// Drops every snapshot whose root contains `path` or lies inside it.
    pub(crate) fn drop_snapshots(&self, path: &Path) -> Result<()> {
        let roots: Vec<String> = self
            .conn
            .prepare_cached("SELECT root FROM snapshot")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        for root in roots {
            let r = Path::new(&root);
            if path.starts_with(r) || r.starts_with(path) {
                self.conn
                    .execute("DELETE FROM snapshot WHERE root = ?1", [&root])?;
            }
        }
        Ok(())
    }

    /// Drops all snapshots (e.g. after a full rescan was requested).
    pub fn clear_snapshots(&self) -> Result<()> {
        self.conn.execute("DELETE FROM snapshot", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use romburak_core::plan::{Done, Files, Ident, Op};

    fn snap() -> Snapshot {
        Snapshot {
            items: vec![Item {
                files: Files::Single("/lib/a.gb".into()),
                ident: Ident::Unknown,
                in_library: true,
            }],
            sets: vec!["neogeo".into()],
        }
    }

    #[test]
    fn roundtrip_and_resolution_drops_it() {
        let s = Store::open_in_memory().unwrap();
        let lib = Path::new("/lib");
        assert!(s.snapshot(lib).unwrap().is_none());
        s.save_snapshot(lib, &snap()).unwrap();
        let back = s.snapshot(lib).unwrap().unwrap();
        assert_eq!(back.items, snap().items);
        assert_eq!(back.sets, snap().sets);
        s.set_resolution(&[1, 2], "Sys", "Game").unwrap();
        assert!(s.snapshot(lib).unwrap().is_none());
    }

    #[test]
    fn executed_ops_mark_only_touched_libraries_dirty() {
        let s = Store::open_in_memory().unwrap();
        s.save_snapshot(Path::new("/lib"), &snap()).unwrap();
        s.save_snapshot(Path::new("/other"), &snap()).unwrap();
        let done = [Done {
            op: Op::Move {
                from: "/inbox/a.gb".into(),
                to: "/lib/GB/a.gb".into(),
            },
            created_dirs: Vec::new(),
            replaced: None,
        }];
        s.index_executed(&done).unwrap();
        // the snapshot stays, with the changed path to re-identify
        assert!(s.snapshot(Path::new("/lib")).unwrap().is_some());
        assert_eq!(
            s.snapshot_dirty(Path::new("/lib")).unwrap(),
            [Path::new("/lib/GB/a.gb")]
        );
        assert!(s.snapshot_dirty(Path::new("/other")).unwrap().is_empty());
        // saving the refreshed snapshot clears the list; dropping it does too
        s.save_snapshot(Path::new("/lib"), &snap()).unwrap();
        assert!(s.snapshot_dirty(Path::new("/lib")).unwrap().is_empty());
        s.index_executed(&done).unwrap();
        s.clear_snapshots().unwrap();
        assert!(s.snapshot_dirty(Path::new("/lib")).unwrap().is_empty());
    }
}
