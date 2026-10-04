//! Schema migrations, tracked via `PRAGMA user_version`.

use crate::{Error, Result};
use rusqlite::Connection;

const MIGRATIONS: &[&str] = &[
    // v1: RDB cache
    "CREATE TABLE rdb_source (
        id     INTEGER PRIMARY KEY,
        path   TEXT NOT NULL UNIQUE,
        system TEXT NOT NULL,
        mtime  INTEGER NOT NULL,
        size   INTEGER NOT NULL
    );
    CREATE TABLE entry (
        id            INTEGER PRIMARY KEY,
        source_id     INTEGER NOT NULL REFERENCES rdb_source(id) ON DELETE CASCADE,
        system        TEXT NOT NULL,
        name          TEXT NOT NULL,
        description   TEXT,
        rom_name      TEXT,
        size          INTEGER,
        crc           INTEGER,
        md5           BLOB,
        sha1          BLOB,
        serial        TEXT,
        region        TEXT,
        genre         TEXT,
        developer     TEXT,
        publisher     TEXT,
        franchise     TEXT,
        release_year  INTEGER,
        release_month INTEGER,
        users         INTEGER
    );
    CREATE INDEX entry_crc    ON entry(crc) WHERE crc IS NOT NULL;
    CREATE INDEX entry_sha1   ON entry(sha1) WHERE sha1 IS NOT NULL;
    CREATE INDEX entry_md5    ON entry(md5) WHERE md5 IS NOT NULL;
    CREATE INDEX entry_serial ON entry(serial) WHERE serial IS NOT NULL;
    CREATE INDEX entry_source ON entry(source_id);
    CREATE INDEX entry_system ON entry(system);
    CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
];

pub fn migrate(conn: &Connection) -> Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let target = MIGRATIONS.len() as i64;
    if current > target {
        return Err(Error::SchemaTooNew(current, target));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as i64 + 1)?;
        tx.commit()?;
    }
    Ok(())
}
