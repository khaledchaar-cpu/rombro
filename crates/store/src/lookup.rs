//! Hash/serial lookups against the entry index.

use crate::record::{COLUMNS, Record};
use crate::{Result, Store};
use rusqlite::ToSql;

impl Store {
    /// Entries with this CRC32; if `size` is given, entries with a different known size are excluded.
    pub fn by_crc(&self, crc: u32, size: Option<u64>) -> Result<Vec<Record>> {
        self.query(
            "crc = ?1 AND (?2 IS NULL OR size IS NULL OR size = ?2)",
            &[&crc, &size],
        )
    }

    pub fn by_sha1(&self, sha1: &[u8]) -> Result<Vec<Record>> {
        self.query("sha1 = ?1", &[&sha1])
    }

    pub fn by_md5(&self, md5: &[u8]) -> Result<Vec<Record>> {
        self.query("md5 = ?1", &[&md5])
    }

    /// Entries with this serial, optionally restricted to one system.
    pub fn by_serial(&self, serial: &str, system: Option<&str>) -> Result<Vec<Record>> {
        self.query(
            "serial = ?1 AND (?2 IS NULL OR system = ?2)",
            &[&serial, &system],
        )
    }

    /// Entries whose serial starts with `prefix`, optionally restricted to one system.
    pub fn by_serial_prefix(&self, prefix: &str, system: Option<&str>) -> Result<Vec<Record>> {
        // Range scan (uses the serial index, unlike LIKE); serials are printable ASCII.
        self.query(
            "serial >= ?1 AND serial < ?1 || '~' AND (?2 IS NULL OR system = ?2)",
            &[&prefix, &system],
        )
    }

    /// Entries of `system` whose romset file is `rom_name` (`1942.zip`).
    pub fn by_rom_name(&self, system: &str, rom_name: &str) -> Result<Vec<Record>> {
        // `+system`: the rom_name index is far more selective than the system one
        self.query("+system = ?1 AND rom_name = ?2", &[&system, &rom_name])
    }

    /// All entries of one system.
    pub fn by_system(&self, system: &str) -> Result<Vec<Record>> {
        self.query("system = ?1", &[&system])
    }

    /// Entry count per system, sorted by system name.
    pub fn system_counts(&self) -> Result<Vec<(String, u64)>> {
        let mut st = self
            .conn
            .prepare_cached("SELECT system, COUNT(*) FROM entry GROUP BY system ORDER BY system")?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    fn query(&self, filter: &str, args: &[&dyn ToSql]) -> Result<Vec<Record>> {
        let sql = format!("SELECT {COLUMNS} FROM entry WHERE {filter}");
        let mut st = self.conn.prepare_cached(&sql)?;
        let rows = st.query_map(args, Record::from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}
