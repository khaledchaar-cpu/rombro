//! rombro-rdb: reader for RetroArch `libretrodb` (`.rdb`) files.
//!
//! Layout: 16-byte header (`RARCHDB\0` + big-endian u64 metadata offset),
//! then MessagePack maps (one per entry) terminated by `nil`, then a
//! metadata map `{"count": n}`.

mod entry;
mod msgpack;

pub use entry::Entry;

use msgpack::{Reader, Value};
use std::path::Path;

const MAGIC: &[u8; 8] = b"RARCHDB\0";
const HEADER_LEN: usize = 16;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),
    #[error("not an RDB file (bad magic)")]
    BadMagic,
    #[error("unexpected end of data at offset {0}")]
    Truncated(usize),
    #[error("unsupported MessagePack tag {tag:#04x} at offset {offset}")]
    UnsupportedTag { tag: u8, offset: usize },
    #[error("malformed data at offset {0}: {1}")]
    Malformed(usize, &'static str),
}

/// An RDB file loaded into memory.
#[derive(Debug, Clone)]
pub struct RdbFile {
    data: Vec<u8>,
}

impl RdbFile {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        Self::from_bytes(std::fs::read(path)?)
    }

    pub fn from_bytes(data: Vec<u8>) -> Result<Self, Error> {
        if data.len() < HEADER_LEN || &data[..8] != MAGIC {
            return Err(Error::BadMagic);
        }
        Ok(Self { data })
    }

    /// Iterates over all entries in file order.
    pub fn entries(&self) -> Entries<'_> {
        Entries {
            reader: Reader::new(&self.data, HEADER_LEN),
            done: false,
        }
    }

    /// Entry count from the trailing metadata block, if present.
    pub fn declared_count(&self) -> Option<u64> {
        let off = u64::from_be_bytes(self.data[8..16].try_into().ok()?);
        let mut r = Reader::new(&self.data, usize::try_from(off).ok()?);
        let Value::Map(n) = r.next().ok()? else {
            return None;
        };
        for _ in 0..n {
            let k = r.next().ok()?;
            let v = r.next().ok()?;
            if k == Value::Str(b"count") {
                if let Value::UInt(c) = v {
                    return Some(c);
                }
            }
            r.skip_body(v).ok()?;
        }
        None
    }
}

/// Iterator over entries; stops at the `nil` terminator or the first error.
pub struct Entries<'a> {
    reader: Reader<'a>,
    done: bool,
}

impl<'a> Iterator for Entries<'a> {
    type Item = Result<Entry<'a>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let at = self.reader.pos();
        let res = match self.reader.next() {
            Ok(Value::Nil) => {
                self.done = true;
                return None;
            }
            Ok(Value::Map(n)) => Entry::parse(&mut self.reader, n),
            Ok(_) => Err(Error::Malformed(at, "entry is not a map")),
            Err(e) => Err(e),
        };
        if res.is_err() {
            self.done = true;
        }
        Some(res)
    }
}

#[cfg(test)]
mod tests;
