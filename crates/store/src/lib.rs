//! rombro-store: SQLite persistence for the RDB cache and (later) library state.

mod catalog;
mod cheevos;
mod chipset;
mod dat;
mod dat_sync;
mod disc;
mod files;
mod firmware;
mod gamify;
mod identify;
mod import;
mod journal;
mod lookup;
mod play;
pub use play::{MIN_PLAY_SECS, PlayStats};
mod record;
mod schema;
mod snapshot;
#[cfg(test)]
mod tests;

pub use catalog::set_names;
pub use cheevos::{RaGame, RaSyncReport};
pub use dat::DatInfo;
pub use dat_sync::{DatSyncReport, http_download, http_get};
pub use disc::DiscMatch;
pub use gamify::Stats;
pub use identify::{Match, candidates};
pub use import::SyncReport;
pub use journal::{JournalEntry, VerdictRow};
pub use record::Record;
pub use snapshot::Snapshot;

use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Default database location (per OS, see [`rombro_core::paths::database`]).
pub fn default_path() -> Option<PathBuf> {
    rombro_core::paths::database()
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("rdb {0}: {1}")]
    Rdb(String, rombro_rdb::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("database schema version {0} is newer than supported ({1})")]
    SchemaTooNew(i64, i64),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Handle to the rombro SQLite database.
pub struct Store {
    conn: Connection,
    /// Parsed DAT sets by (system, name); `None` = not in that DAT. Emptied on DAT import.
    dat_cache: std::cell::RefCell<dat::DatCache>,
}

impl Store {
    /// Opens (or creates) the database file and runs pending migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        // the app opens several connections (plan, execute, library refresh); a writer waits
        // for another instead of failing with "database is locked"
        conn.busy_timeout(std::time::Duration::from_secs(30))?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        schema::migrate(&conn)?;
        Ok(Self {
            conn,
            dat_cache: Default::default(),
        })
    }
}
