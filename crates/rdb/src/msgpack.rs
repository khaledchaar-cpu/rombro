//! Minimal zero-copy MessagePack reader covering the subset used by libretrodb.

use crate::Error;

/// A decoded scalar or container header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value<'a> {
    Nil,
    Bool(bool),
    UInt(u64),
    Int(i64),
    Str(&'a [u8]),
    Bin(&'a [u8]),
    /// Map with `n` key/value pairs following.
    Map(usize),
    /// Array with `n` elements following.
    Array(usize),
}

/// Cursor over a byte slice.
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8], pos: usize) -> Self {
        Self { buf, pos }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::Truncated(self.pos))?;
        let s = self
            .buf
            .get(self.pos..end)
            .ok_or(Error::Truncated(self.pos))?;
        self.pos = end;
        Ok(s)
    }

    fn be<const N: usize>(&mut self) -> Result<u64, Error> {
        let mut v = 0u64;
        for &b in self.take(N)? {
            v = (v << 8) | u64::from(b);
        }
        Ok(v)
    }

    fn len<const N: usize>(&mut self) -> Result<usize, Error> {
        usize::try_from(self.be::<N>()?).map_err(|_| Error::Truncated(self.pos))
    }

    /// Reads the next value header (containers are not descended).
    pub fn next(&mut self) -> Result<Value<'a>, Error> {
        let at = self.pos;
        let tag = self.take(1)?[0];
        Ok(match tag {
            0x00..=0x7f => Value::UInt(u64::from(tag)),
            0x80..=0x8f => Value::Map(usize::from(tag & 0x0f)),
            0x90..=0x9f => Value::Array(usize::from(tag & 0x0f)),
            0xa0..=0xbf => Value::Str(self.take(usize::from(tag & 0x1f))?),
            0xc0 => Value::Nil,
            0xc2 => Value::Bool(false),
            0xc3 => Value::Bool(true),
            0xc4 => {
                let n = self.len::<1>()?;
                Value::Bin(self.take(n)?)
            }
            0xc5 => {
                let n = self.len::<2>()?;
                Value::Bin(self.take(n)?)
            }
            0xc6 => {
                let n = self.len::<4>()?;
                Value::Bin(self.take(n)?)
            }
            0xcc => Value::UInt(self.be::<1>()?),
            0xcd => Value::UInt(self.be::<2>()?),
            0xce => Value::UInt(self.be::<4>()?),
            0xcf => Value::UInt(self.be::<8>()?),
            0xd0 => Value::Int(i64::from(self.be::<1>()? as u8 as i8)),
            0xd1 => Value::Int(i64::from(self.be::<2>()? as u16 as i16)),
            0xd2 => Value::Int(i64::from(self.be::<4>()? as u32 as i32)),
            0xd3 => Value::Int(self.be::<8>()? as i64),
            0xd9 => {
                let n = self.len::<1>()?;
                Value::Str(self.take(n)?)
            }
            0xda => {
                let n = self.len::<2>()?;
                Value::Str(self.take(n)?)
            }
            0xdb => {
                let n = self.len::<4>()?;
                Value::Str(self.take(n)?)
            }
            0xdc => Value::Array(self.len::<2>()?),
            0xdd => Value::Array(self.len::<4>()?),
            0xde => Value::Map(self.len::<2>()?),
            0xdf => Value::Map(self.len::<4>()?),
            0xe0..=0xff => Value::Int(i64::from(tag as i8)),
            _ => return Err(Error::UnsupportedTag { tag, offset: at }),
        })
    }

    /// Skips a value whose header was already read, including nested contents.
    pub fn skip_body(&mut self, v: Value<'a>) -> Result<(), Error> {
        let n = match v {
            Value::Map(n) => n.checked_mul(2).ok_or(Error::Truncated(self.pos))?,
            Value::Array(n) => n,
            _ => return Ok(()),
        };
        for _ in 0..n {
            let inner = self.next()?;
            self.skip_body(inner)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_scalars() {
        let buf = [
            0x05, 0xcd, 0x1e, 0x52, 0xa2, b'h', b'i', 0xc4, 2, 0xab, 0xcd, 0xff, 0xc0,
        ];
        let mut r = Reader::new(&buf, 0);
        assert_eq!(r.next().unwrap(), Value::UInt(5));
        assert_eq!(r.next().unwrap(), Value::UInt(7762));
        assert_eq!(r.next().unwrap(), Value::Str(b"hi"));
        assert_eq!(r.next().unwrap(), Value::Bin(&[0xab, 0xcd]));
        assert_eq!(r.next().unwrap(), Value::Int(-1));
        assert_eq!(r.next().unwrap(), Value::Nil);
        assert!(matches!(r.next(), Err(Error::Truncated(_))));
    }

    #[test]
    fn skips_nested() {
        // {"a": [1, {"b": 2}]}, 7
        let buf = [0x81, 0xa1, b'a', 0x92, 1, 0x81, 0xa1, b'b', 2, 7];
        let mut r = Reader::new(&buf, 0);
        let v = r.next().unwrap();
        r.skip_body(v).unwrap();
        assert_eq!(r.next().unwrap(), Value::UInt(7));
    }

    #[test]
    fn truncated_str() {
        let mut r = Reader::new(&[0xa5, b'x'], 0);
        assert!(matches!(r.next(), Err(Error::Truncated(_))));
    }
}
