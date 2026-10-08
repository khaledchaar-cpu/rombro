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
    CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    // v2: executed plans (undo) and user decisions for ambiguous matches
    "CREATE TABLE journal (
        id      INTEGER PRIMARY KEY,
        ts      INTEGER NOT NULL,
        library TEXT NOT NULL,
        done    TEXT NOT NULL,
        state   TEXT NOT NULL
    );
    CREATE TABLE resolution (
        sha1   BLOB PRIMARY KEY,
        system TEXT NOT NULL,
        name   TEXT NOT NULL
    );",
    "CREATE TABLE verdict (
        system  TEXT NOT NULL,
        name    TEXT NOT NULL,
        verdict TEXT NOT NULL,
        PRIMARY KEY (system, name)
    );",
    // v4: library index / hash cache (one row per file; ROM list as JSON)
    "CREATE TABLE file (
        path  TEXT PRIMARY KEY,
        size  INTEGER NOT NULL,
        mtime INTEGER NOT NULL,
        roms  TEXT NOT NULL
    );",
    // v5: when a file entered its place (unix seconds); NULL for rows indexed before v5
    "ALTER TABLE file ADD COLUMN added INTEGER;",
    // v6: unlocked achievements (unix seconds)
    "CREATE TABLE achievement (id TEXT PRIMARY KEY, unlocked_at INTEGER NOT NULL);",
    // v7: arcade DATs (member lists per core and set; ROMs as JSON)
    "CREATE TABLE dat_source (
        system  TEXT PRIMARY KEY,
        version TEXT NOT NULL,
        fetched INTEGER NOT NULL
    );
    CREATE TABLE dat_set (
        system TEXT NOT NULL,
        name   TEXT NOT NULL,
        romof  TEXT,
        bios   INTEGER NOT NULL,
        roms   TEXT NOT NULL,
        PRIMARY KEY (system, name)
    ) WITHOUT ROWID;",
    // v8: why and when a verdict was given
    "ALTER TABLE verdict ADD COLUMN reason TEXT NOT NULL DEFAULT '';
     ALTER TABLE verdict ADD COLUMN decided INTEGER;",
    // v9: rehash SNES files that may be combined Sufami Turbo images (keeps `added`)
    "UPDATE file SET mtime = -1
     WHERE size BETWEEN 1572864 AND 3146240
       AND lower(substr(path, -4)) IN ('.smc', '.sfc', '.swc', '.fig');",
    // v10: DATs now record disks (CHDs); drop them so the next sync imports them again
    "DELETE FROM dat_set; DELETE FROM dat_source;",
    // v11: driver status (sets the emulator can't run); drop DATs again for a fresh import
    "ALTER TABLE dat_set ADD COLUMN working INTEGER NOT NULL DEFAULT 1;
     DELETE FROM dat_set; DELETE FROM dat_source;",
    // v12: CHD data tracks now hash with their pregap (Redump layout); rehash CHDs
    "UPDATE file SET mtime = -1 WHERE lower(substr(path, -4)) = '.chd';",
    // v13: identified library per root (gzip JSON); dropped whenever identification may change
    "CREATE TABLE snapshot (root TEXT PRIMARY KEY, data BLOB NOT NULL);
     CREATE TRIGGER snap_rdb_i AFTER INSERT ON rdb_source BEGIN DELETE FROM snapshot; END;
     CREATE TRIGGER snap_rdb_u AFTER UPDATE ON rdb_source BEGIN DELETE FROM snapshot; END;
     CREATE TRIGGER snap_rdb_d AFTER DELETE ON rdb_source BEGIN DELETE FROM snapshot; END;
     CREATE TRIGGER snap_dat_i AFTER INSERT ON dat_source BEGIN DELETE FROM snapshot; END;
     CREATE TRIGGER snap_dat_u AFTER UPDATE ON dat_source BEGIN DELETE FROM snapshot; END;
     CREATE TRIGGER snap_dat_d AFTER DELETE ON dat_source BEGIN DELETE FROM snapshot; END;
     CREATE TRIGGER snap_res_i AFTER INSERT ON resolution BEGIN DELETE FROM snapshot; END;
     CREATE TRIGGER snap_res_u AFTER UPDATE ON resolution BEGIN DELETE FROM snapshot; END;
     CREATE TRIGGER snap_res_d AFTER DELETE ON resolution BEGIN DELETE FROM snapshot; END;",
    // v14: files changed since a snapshot was taken, re-identified on its next load
    "CREATE TABLE snapshot_dirty (root TEXT NOT NULL, path TEXT NOT NULL, PRIMARY KEY (root, path));
     CREATE TRIGGER snap_dirty_d AFTER DELETE ON snapshot
       BEGIN DELETE FROM snapshot_dirty WHERE root = OLD.root; END;",
    // v15: DAT lookups by set name alone (re-packed romsets: which cores know the set)
    "CREATE INDEX dat_set_name ON dat_set(name);",
    // v16: launcher – play time and favorites per game (`sha1:<hex>`, or `path:<abs>` unhashed)
    "CREATE TABLE play_stats (
        game    TEXT PRIMARY KEY,
        plays   INTEGER NOT NULL,
        seconds INTEGER NOT NULL,
        last    INTEGER NOT NULL
    );
    CREATE TABLE favorite (game TEXT PRIMARY KEY, added INTEGER NOT NULL);",
    // v17: RetroAchievements – games and hashes per console (from the Web API), RA hash per file
    "CREATE TABLE ra_console (id INTEGER PRIMARY KEY, synced INTEGER NOT NULL);
    CREATE TABLE ra_game (
        id           INTEGER PRIMARY KEY,
        console      INTEGER NOT NULL,
        title        TEXT NOT NULL,
        achievements INTEGER NOT NULL,
        points       INTEGER NOT NULL
    );
    CREATE TABLE ra_hash (hash TEXT PRIMARY KEY, game INTEGER NOT NULL);
    CREATE TABLE ra_file (
        path  TEXT PRIMARY KEY,
        size  INTEGER NOT NULL,
        mtime INTEGER NOT NULL,
        hash  TEXT
    );",
    // v18: RetroAchievements – the logged-in user's unlocks per game
    "CREATE TABLE ra_progress (
        game     INTEGER PRIMARY KEY,
        awarded  INTEGER NOT NULL,
        hardcore INTEGER NOT NULL,
        total    INTEGER NOT NULL,
        award    TEXT
    );",
];

pub fn migrate(conn: &Connection) -> Result<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let target = MIGRATIONS.len() as i64;
    if current > target {
        return Err(Error::SchemaTooNew(current, target));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx =
            rusqlite::Transaction::new_unchecked(conn, rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as i64 + 1)?;
        tx.commit()?;
    }
    conn.execute_batch(ENTRY_INDEXES)?;
    Ok(())
}

/// Lookup indexes on `entry`; dropped during bulk imports and rebuilt afterwards.
pub const ENTRY_INDEXES: &str = "
    CREATE INDEX IF NOT EXISTS entry_crc    ON entry(crc) WHERE crc IS NOT NULL;
    CREATE INDEX IF NOT EXISTS entry_sha1   ON entry(sha1) WHERE sha1 IS NOT NULL;
    CREATE INDEX IF NOT EXISTS entry_md5    ON entry(md5) WHERE md5 IS NOT NULL;
    CREATE INDEX IF NOT EXISTS entry_serial ON entry(serial) WHERE serial IS NOT NULL;
    CREATE INDEX IF NOT EXISTS entry_source ON entry(source_id);
    CREATE INDEX IF NOT EXISTS entry_system ON entry(system);
    CREATE INDEX IF NOT EXISTS entry_rom_name ON entry(rom_name) WHERE rom_name IS NOT NULL;";

pub const DROP_ENTRY_INDEXES: &str = "
    DROP INDEX IF EXISTS entry_crc; DROP INDEX IF EXISTS entry_sha1;
    DROP INDEX IF EXISTS entry_md5; DROP INDEX IF EXISTS entry_serial;
    DROP INDEX IF EXISTS entry_system; DROP INDEX IF EXISTS entry_rom_name;";
