//! Journal of executed plans (for undo) and persisted user resolutions.

use crate::{Result, Store};
use rombro_core::plan::Verdict;
use rusqlite::{OptionalExtension, params};
use std::collections::HashMap;

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

    /// The `limit` most recent executions, newest first, with their state (`done` or `undone`).
    pub fn journals(&self, limit: usize) -> Result<Vec<(JournalEntry, String)>> {
        let mut st = self.conn.prepare(
            "SELECT id, ts, library, done, state FROM journal ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = st.query_map([limit as i64], |r| {
            Ok((
                JournalEntry {
                    id: r.get(0)?,
                    ts: r.get(1)?,
                    library: r.get(2)?,
                    done: r.get(3)?,
                },
                r.get(4)?,
            ))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
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

    /// Stores (or with `None` clears) the user's verdict on a release 1G1R rejected.
    pub fn set_verdict(&self, system: &str, name: &str, v: Option<Verdict>) -> Result<()> {
        match v {
            Some(v) => {
                let v = match v {
                    Verdict::Keep => "keep",
                    Verdict::Discard => "discard",
                    Verdict::Prefer => "prefer",
                };
                self.conn.execute(
                    "INSERT OR REPLACE INTO verdict (system, name, verdict) VALUES (?1, ?2, ?3)",
                    params![system, name, v],
                )?;
            }
            None => {
                self.conn.execute(
                    "DELETE FROM verdict WHERE system = ?1 AND name = ?2",
                    params![system, name],
                )?;
            }
        }
        Ok(())
    }

    /// All stored verdicts, ready for `plan::Options::verdicts`.
    pub fn verdicts(&self) -> Result<HashMap<(String, String), Verdict>> {
        let mut st = self
            .conn
            .prepare("SELECT system, name, verdict FROM verdict")?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut out = HashMap::new();
        for row in rows {
            let (system, name, v) = row?;
            let v = if v == "keep" {
                Verdict::Keep
            } else {
                Verdict::Discard
            };
            out.insert((system, name), v);
        }
        Ok(out)
    }
}
