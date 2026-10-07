//! Identified library per root, so planning and the library view need neither a folder walk
//! nor a database lookup while nothing changed. Dropped (by triggers) when the game
//! databases or resolutions change, and by every change to the file index below its root.

use crate::{Result, Store};
use rombro_core::plan::Item;
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
        self.conn.execute(
            "INSERT OR REPLACE INTO snapshot (root, data) VALUES (?1, ?2)",
            params![root.to_string_lossy(), data],
        )?;
        Ok(())
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
    use rombro_core::plan::{Done, Files, Ident, Op};

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
    fn executed_ops_drop_only_touched_libraries() {
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
        assert!(s.snapshot(Path::new("/lib")).unwrap().is_none());
        assert!(s.snapshot(Path::new("/other")).unwrap().is_some());
    }
}
