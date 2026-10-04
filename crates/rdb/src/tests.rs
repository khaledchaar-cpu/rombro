use super::*;

/// Builds a synthetic RDB: entries given as pre-encoded maps.
fn build(entries: &[Vec<u8>], count: u8) -> Vec<u8> {
    let mut body = Vec::new();
    for e in entries {
        body.extend_from_slice(e);
    }
    body.push(0xc0);
    let off = (HEADER_LEN + body.len()) as u64;
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(&off.to_be_bytes());
    out.extend(body);
    out.extend_from_slice(&[0x81, 0xa5]);
    out.extend_from_slice(b"count");
    out.push(count);
    out
}

fn str_(s: &str) -> Vec<u8> {
    let mut v = vec![0xd9, s.len() as u8];
    v.extend_from_slice(s.as_bytes());
    v
}

fn game(name: &str, crc: [u8; 4]) -> Vec<u8> {
    let mut m = vec![0x84];
    m.extend(str_("name"));
    m.extend(str_(name));
    m.extend(str_("crc"));
    m.extend([0xc4, 4]);
    m.extend(crc);
    m.extend(str_("releaseyear"));
    m.extend([0xcd, 0x07, 0xc6]);
    m.extend(str_("analog"));
    m.extend([0x92, 1, 0xa1, b'x']); // unknown key, nested value
    m
}

#[test]
fn reads_entries_and_count() {
    let meta = {
        let mut m = vec![0x81];
        m.extend(str_("serial"));
        m.extend(str_("SLUS-00001"));
        m
    };
    let data = build(
        &[
            game("Foo (USA)", [0xde, 0xad, 0xbe, 0xef]),
            meta,
            game("Bar (Japan)", [0, 0, 0, 1]),
        ],
        3,
    );
    let rdb = RdbFile::from_bytes(data).unwrap();
    let all: Vec<_> = rdb.entries().collect::<Result<_, _>>().unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].name, Some("Foo (USA)"));
    assert_eq!(all[0].crc32(), Some(0xdead_beef));
    assert_eq!(all[0].release_year, Some(1990));
    assert!(all[1].is_metadata_only());
    assert_eq!(all[1].serial, Some("SLUS-00001"));
    assert_eq!(all[2].crc32(), Some(1));
    assert_eq!(rdb.declared_count(), Some(3));
}

#[test]
fn rejects_bad_magic() {
    assert!(matches!(
        RdbFile::from_bytes(b"NOTANRDB00000000".to_vec()),
        Err(Error::BadMagic)
    ));
}

#[test]
fn truncated_entry_yields_error_then_stops() {
    let mut data = build(&[game("Foo", [1, 2, 3, 4])], 1);
    data.truncate(HEADER_LEN + 10);
    let rdb = RdbFile::from_bytes(data).unwrap();
    let mut it = rdb.entries();
    assert!(matches!(it.next(), Some(Err(Error::Truncated(_)))));
    assert!(it.next().is_none());
}

/// Parses every real RDB if present. Run with `cargo test -- --ignored`.
#[test]
#[ignore]
fn real_rdbs() {
    let Some(home) = std::env::var_os("HOME") else {
        return;
    };
    let dir = Path::new(&home).join(".config/retroarch/database/rdb");
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for f in rd.flatten() {
        let rdb = RdbFile::open(f.path()).unwrap();
        let n = rdb.entries().inspect(|e| assert!(e.is_ok())).count() as u64;
        assert_eq!(Some(n), rdb.declared_count(), "{:?}", f.path());
    }
}
