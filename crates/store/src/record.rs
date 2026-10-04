//! Owned database entry, as stored in and returned from SQLite.

use rombro_rdb::Entry;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Record {
    pub id: i64,
    pub system: String,
    pub name: String,
    pub description: Option<String>,
    pub rom_name: Option<String>,
    pub size: Option<u64>,
    pub crc: Option<u32>,
    pub md5: Option<Vec<u8>>,
    pub sha1: Option<Vec<u8>>,
    pub serial: Option<String>,
    pub region: Option<String>,
    pub genre: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub franchise: Option<String>,
    pub release_year: Option<u64>,
    pub release_month: Option<u64>,
    pub users: Option<u64>,
}

pub(crate) const COLUMNS: &str = "id, system, name, description, rom_name, size, crc, md5, sha1, serial, \
     region, genre, developer, publisher, franchise, release_year, release_month, users";

impl Record {
    pub(crate) fn from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: r.get(0)?,
            system: r.get(1)?,
            name: r.get(2)?,
            description: r.get(3)?,
            rom_name: r.get(4)?,
            size: r.get(5)?,
            crc: r.get(6)?,
            md5: r.get(7)?,
            sha1: r.get(8)?,
            serial: r.get(9)?,
            region: r.get(10)?,
            genre: r.get(11)?,
            developer: r.get(12)?,
            publisher: r.get(13)?,
            franchise: r.get(14)?,
            release_year: r.get(15)?,
            release_month: r.get(16)?,
            users: r.get(17)?,
        })
    }
}

/// Fills fields missing in `game` from a metadata-only entry.
pub(crate) fn merge<'a>(game: &mut Entry<'a>, meta: &Entry<'a>) {
    macro_rules! fill {
        ($($f:ident),*) => { $( if game.$f.is_none() { game.$f = meta.$f; } )* };
    }
    fill!(
        description,
        rom_name,
        serial,
        region,
        genre,
        developer,
        publisher,
        franchise,
        origin,
        size,
        release_year,
        release_month,
        users,
        crc,
        md5,
        sha1
    );
}
