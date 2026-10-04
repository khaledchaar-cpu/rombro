//! Journal of executed plans (for undo) and persisted user resolutions.

use crate::{Result, Store};
use rusqlite::{OptionalExtension, params};

/// A stored execution: the serialized completed operations of one plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    pub id: i64,
    /// Unix seconds.
    pub ts: i64,
    pub library: String,
    pub done: String,
}

impl Store {
    pub fn add_journal(&self, ts: i64, library: &str, done: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO journal (ts, library, done, state) VALUES (?1, ?2, ?3, 'done')",
            params![ts, library, done],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Most recent execution that has not been undone.
    pub fn last_journal(&self) -> Result<Option<JournalEntry>> {
        Ok(self
            .conn
            .query_row(
                "SELECT id, ts, library, done FROM journal WHERE state = 'done' ORDER BY id DESC LIMIT 1",
                [],
                |r| {
                    Ok(JournalEntry {
                        id: r.get(0)?,
                        ts: r.get(1)?,
                        library: r.get(2)?,
                        done: r.get(3)?,
                    })
                },
            )
            .optional()?)
    }

    pub fn mark_undone(&self, id: i64) -> Result<()> {
        self.conn
            .execute("UPDATE journal SET state = 'undone' WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Remembers the user's choice for a file whose match was ambiguous.
    pub fn set_resolution(&self, sha1: &[u8], system: &str, name: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO resolution (sha1, system, name) VALUES (?1, ?2, ?3)",
            params![sha1, system, name],
        )?;
        Ok(())
    }

    /// The stored choice (system, name) for a file, if any.
    pub fn resolution(&self, sha1: &[u8]) -> Result<Option<(String, String)>> {
        Ok(self
            .conn
            .query_row(
                "SELECT system, name FROM resolution WHERE sha1 = ?1",
                [sha1],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?)
    }
}
