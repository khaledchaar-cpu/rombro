//! Typed view of one RDB entry.

use crate::Error;
use crate::msgpack::{Reader, Value};

/// One database entry. Strings and hashes borrow from the RDB buffer.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Entry<'a> {
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub rom_name: Option<&'a str>,
    pub serial: Option<&'a str>,
    pub region: Option<&'a str>,
    pub genre: Option<&'a str>,
    pub developer: Option<&'a str>,
    pub publisher: Option<&'a str>,
    pub franchise: Option<&'a str>,
    pub origin: Option<&'a str>,
    pub size: Option<u64>,
    pub release_year: Option<u64>,
    pub release_month: Option<u64>,
    pub users: Option<u64>,
    /// Raw 4-byte CRC32 (big-endian).
    pub crc: Option<&'a [u8]>,
    pub md5: Option<&'a [u8]>,
    pub sha1: Option<&'a [u8]>,
}

impl<'a> Entry<'a> {
    /// CRC32 as integer, if present and 4 bytes long.
    pub fn crc32(&self) -> Option<u32> {
        self.crc
            .and_then(|c| <[u8; 4]>::try_from(c).ok())
            .map(u32::from_be_bytes)
    }

    /// Entries without a name carry only supplementary metadata.
    pub fn is_metadata_only(&self) -> bool {
        self.name.is_none()
    }

    /// Parses the map body of `n` pairs at the reader's position.
    pub(crate) fn parse(r: &mut Reader<'a>, n: usize) -> Result<Self, Error> {
        let mut e = Entry::default();
        for _ in 0..n {
            let at = r.pos();
            let key = match r.next()? {
                Value::Str(k) => k,
                _ => return Err(Error::Malformed(at, "non-string key")),
            };
            let v = r.next()?;
            match key {
                b"name" => e.name = as_str(v),
                b"description" => e.description = as_str(v),
                b"rom_name" => e.rom_name = as_str(v),
                b"serial" => e.serial = as_str(v).or(as_bin(v).and_then(utf8)),
                b"region" => e.region = as_str(v),
                b"genre" => e.genre = as_str(v),
                b"developer" => e.developer = as_str(v),
                b"publisher" => e.publisher = as_str(v),
                b"franchise" => e.franchise = as_str(v),
                b"origin" => e.origin = as_str(v),
                b"size" => e.size = as_uint(v),
                b"releaseyear" => e.release_year = as_uint(v),
                b"releasemonth" => e.release_month = as_uint(v),
                b"users" => e.users = as_uint(v),
                b"crc" => e.crc = as_bin(v),
                b"md5" => e.md5 = as_bin(v),
                b"sha1" => e.sha1 = as_bin(v),
                _ => r.skip_body(v)?,
            }
        }
        Ok(e)
    }
}

fn utf8(b: &[u8]) -> Option<&str> {
    std::str::from_utf8(b).ok()
}

fn as_str(v: Value<'_>) -> Option<&str> {
    match v {
        Value::Str(s) => utf8(s),
        _ => None,
    }
}

fn as_bin(v: Value<'_>) -> Option<&[u8]> {
    match v {
        Value::Bin(b) => Some(b),
        _ => None,
    }
}

fn as_uint(v: Value<'_>) -> Option<u64> {
    match v {
        Value::UInt(u) => Some(u),
        Value::Int(i) => u64::try_from(i).ok(),
        _ => None,
    }
}
