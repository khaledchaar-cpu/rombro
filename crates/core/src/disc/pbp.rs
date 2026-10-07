//! PSP `EBOOT.PBP` files, also used for PS1 classics (PSX2PSP, PS Store). The disc inside is
//! compressed, so no database hash exists; the `PARAM.SFO` in the header names the serial.

use super::serial::{DiscId, Platform};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const MAGIC: &[u8; 4] = b"\0PBP";
/// `PARAM.SFO` value format of UTF-8 strings (`utf8` and `utf8-s`).
const UTF8: [u16; 2] = [0x0204, 0x0004];

pub fn is_pbp(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pbp"))
}

/// Platform and serial from the `PARAM.SFO` of a PBP file.
pub fn identify(path: &Path) -> io::Result<Option<DiscId>> {
    let mut head = vec![0u8; 0x1000];
    let n = File::open(path)?.take(head.len() as u64).read(&mut head)?;
    head.truncate(n);
    Ok(parse(&head))
}

/// Parses the PBP header and its `PARAM.SFO` (which follows the header directly).
pub fn parse(head: &[u8]) -> Option<DiscId> {
    let le32 = |b: &[u8], o: usize| Some(u32::from_le_bytes(b.get(o..o + 4)?.try_into().ok()?));
    let le16 = |b: &[u8], o: usize| Some(u16::from_le_bytes(b.get(o..o + 2)?.try_into().ok()?));
    if head.get(..4)? != MAGIC {
        return None;
    }
    let (start, end) = (le32(head, 8)? as usize, le32(head, 12)? as usize);
    let sfo = head.get(start..end.min(head.len()))?;
    if sfo.get(..4)? != b"\0PSF" {
        return None;
    }
    let (keys, data, count) = (
        le32(sfo, 8)? as usize,
        le32(sfo, 12)? as usize,
        le32(sfo, 16)?,
    );
    let (mut serial, mut category) = (None, None);
    for i in 0..count as usize {
        let e = 20 + i * 16;
        let key_at = keys + le16(sfo, e)? as usize;
        let key_len = sfo.get(key_at..)?.iter().position(|&b| b == 0)?;
        let key = sfo.get(key_at..key_at + key_len)?;
        if !UTF8.contains(&le16(sfo, e + 2)?) {
            continue;
        }
        let (len, at) = (
            le32(sfo, e + 4)? as usize,
            data + le32(sfo, e + 12)? as usize,
        );
        let val = std::str::from_utf8(sfo.get(at..at + len)?).ok()?;
        let val = val.trim_end_matches('\0').trim();
        match key {
            b"DISC_ID" => serial = Some(val.to_owned()),
            b"CATEGORY" => category = Some(val.to_owned()),
            _ => {}
        }
    }
    let serial = serial.filter(|s| !s.is_empty())?;
    // `ME`: PS1 classic; everything else is a PSP title
    let platform = if category.as_deref() == Some("ME") {
        Platform::Ps1
    } else {
        Platform::Psp
    };
    Some(DiscId {
        platform,
        serial,
        variant: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PBP header with a `PARAM.SFO` holding `entries` (all UTF-8 strings).
    pub(crate) fn pbp(entries: &[(&str, &str)]) -> Vec<u8> {
        let (mut keys, mut data, mut index) = (Vec::new(), Vec::new(), Vec::new());
        for (k, v) in entries {
            let mut val = v.as_bytes().to_vec();
            val.push(0);
            index.extend((keys.len() as u16).to_le_bytes());
            index.extend(0x0204u16.to_le_bytes());
            index.extend((val.len() as u32).to_le_bytes());
            index.extend((val.len() as u32).to_le_bytes());
            index.extend((data.len() as u32).to_le_bytes());
            keys.extend(k.as_bytes());
            keys.push(0);
            data.extend(val);
        }
        let key_off = 20 + index.len();
        let data_off = key_off + keys.len();
        let mut sfo = b"\0PSF".to_vec();
        sfo.extend(0x0101u32.to_le_bytes());
        sfo.extend((key_off as u32).to_le_bytes());
        sfo.extend((data_off as u32).to_le_bytes());
        sfo.extend((entries.len() as u32).to_le_bytes());
        sfo.extend(index);
        sfo.extend(keys);
        sfo.extend(data);
        let mut out = MAGIC.to_vec();
        out.extend(0x0001_0000u32.to_le_bytes());
        let start = 40u32;
        let end = start + sfo.len() as u32;
        out.extend(start.to_le_bytes());
        for _ in 0..7 {
            out.extend(end.to_le_bytes());
        }
        out.extend(sfo);
        out
    }

    #[test]
    fn ps1_classic_and_psp_title() {
        let id = parse(&pbp(&[("CATEGORY", "ME"), ("DISC_ID", "SLUS00877")])).unwrap();
        assert_eq!(
            (id.platform, id.serial.as_str()),
            (Platform::Ps1, "SLUS00877")
        );
        assert!(id.lookup_keys().contains(&"SLUS-00877".to_owned()));
        let psp = parse(&pbp(&[("DISC_ID", "ULUS10041"), ("CATEGORY", "EG")])).unwrap();
        assert_eq!(psp.platform, Platform::Psp);
        assert!(parse(&pbp(&[("CATEGORY", "ME")])).is_none());
        assert!(parse(b"\0PBX").is_none());
    }
}
