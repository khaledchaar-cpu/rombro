use crate::Store;
use std::path::Path;

/// Minimal MessagePack encoders for synthetic RDB fixtures.
fn str_(s: &str) -> Vec<u8> {
    let mut v = vec![0xd9, s.len() as u8];
    v.extend_from_slice(s.as_bytes());
    v
}
fn bin(b: &[u8]) -> Vec<u8> {
    let mut v = vec![0xc4, b.len() as u8];
    v.extend_from_slice(b);
    v
}
fn uint(n: u32) -> Vec<u8> {
    let mut v = vec![0xce];
    v.extend_from_slice(&n.to_be_bytes());
    v
}

enum F<'a> {
    S(&'a str),
    B(&'a [u8]),
    U(u32),
}

fn map(fields: &[(&str, F<'_>)]) -> Vec<u8> {
    let mut v = vec![0x80 | fields.len() as u8];
    for (k, f) in fields {
        v.extend(str_(k));
        v.extend(match f {
            F::S(s) => str_(s),
            F::B(b) => bin(b),
            F::U(n) => uint(*n),
        });
    }
    v
}

fn write_rdb(path: &Path, entries: &[Vec<u8>]) {
    let mut body: Vec<u8> = entries.concat();
    body.push(0xc0);
    let meta_off = 16 + body.len() as u64;
    let mut data = b"RARCHDB\0".to_vec();
    data.extend_from_slice(&meta_off.to_be_bytes());
    data.extend(body);
    data.extend(map(&[("count", F::U(entries.len() as u32))]));
    std::fs::write(path, data).unwrap();
}

fn fixture(dir: &Path) {
    write_rdb(
        &dir.join("Nintendo - SNES.rdb"),
        &[
            map(&[
                ("name", F::S("Foo (USA)")),
                ("size", F::U(1024)),
                ("crc", F::B(&[0xde, 0xad, 0xbe, 0xef])),
                ("sha1", F::B(&[1; 20])),
                ("md5", F::B(&[2; 16])),
            ]),
            map(&[("name", F::S("Bar (Europe)")), ("crc", F::B(&[0, 0, 0, 1]))]),
            // metadata-only, merges into Foo by crc
            map(&[
                ("crc", F::B(&[0xde, 0xad, 0xbe, 0xef])),
                ("genre", F::S("Action")),
            ]),
            // orphan
            map(&[("crc", F::B(&[9, 9, 9, 9])), ("genre", F::S("X"))]),
        ],
    );
    write_rdb(
        &dir.join("Sony - PlayStation.rdb"),
        &[
            map(&[
                ("name", F::S("Baz (Japan)")),
                ("serial", F::S("SLPS-00001")),
            ]),
            map(&[("serial", F::S("SLPS-00001")), ("releaseyear", F::U(1995))]),
        ],
    );
}

#[test]
fn migrate_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");
    drop(Store::open(&db).unwrap());
    Store::open(&db).unwrap();
}

#[test]
fn sync_imports_merges_and_looks_up() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let mut s = Store::open_in_memory().unwrap();
    let r = s.sync_rdbs(dir.path()).unwrap();
    assert_eq!((r.imported, r.entries, r.merged, r.orphaned), (2, 3, 2, 1));

    let foo = s.by_crc(0xdeadbeef, Some(1024)).unwrap();
    assert_eq!(foo.len(), 1);
    assert_eq!(foo[0].name, "Foo (USA)");
    assert_eq!(foo[0].system, "Nintendo - SNES");
    assert_eq!(foo[0].genre.as_deref(), Some("Action"));
    assert!(s.by_crc(0xdeadbeef, Some(999)).unwrap().is_empty());
    assert_eq!(
        s.by_crc(1, Some(5)).unwrap().len(),
        1,
        "unknown size matches"
    );
    assert_eq!(s.by_sha1(&[1; 20]).unwrap()[0].name, "Foo (USA)");
    assert_eq!(s.by_md5(&[2; 16]).unwrap()[0].name, "Foo (USA)");

    let baz = s
        .by_serial("SLPS-00001", Some("Sony - PlayStation"))
        .unwrap();
    assert_eq!(baz[0].release_year, Some(1995));
    assert!(
        s.by_serial("SLPS-00001", Some("Nintendo - SNES"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(s.system_counts().unwrap().len(), 2);
}

#[test]
fn sync_is_incremental() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let mut s = Store::open_in_memory().unwrap();
    s.sync_rdbs(dir.path()).unwrap();

    let r = s.sync_rdbs(dir.path()).unwrap();
    assert_eq!((r.imported, r.unchanged, r.removed), (0, 2, 0));

    std::fs::remove_file(dir.path().join("Sony - PlayStation.rdb")).unwrap();
    write_rdb(
        &dir.path().join("Nintendo - SNES.rdb"),
        &[map(&[("name", F::S("New (USA)"))])],
    );
    let r = s.sync_rdbs(dir.path()).unwrap();
    assert_eq!((r.imported, r.removed), (1, 1));
    assert_eq!(
        s.system_counts().unwrap(),
        vec![("Nintendo - SNES".to_owned(), 1)]
    );
}

#[test]
fn identify_confirms_by_sha1() {
    use crate::Match;
    use rombro_core::Hashes;
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let mut s = Store::open_in_memory().unwrap();
    s.sync_rdbs(dir.path()).unwrap();
    let foo = Hashes {
        size: 1024,
        crc: 0xdead_beef,
        sha1: [1; 20],
        md5: Some([2; 16]),
    };
    assert!(matches!(s.identify(&foo).unwrap(), Match::Verified(v) if v[0].name == "Foo (USA)"));
    let bad = Hashes {
        sha1: [7; 20],
        ..foo
    };
    assert_eq!(s.identify(&bad).unwrap(), Match::Unknown);
    let wrong_size = Hashes { size: 2048, ..foo };
    assert_eq!(s.identify(&wrong_size).unwrap(), Match::Unknown);
    let bar = Hashes {
        size: 99,
        crc: 1,
        sha1: [0; 20],
        md5: None,
    };
    assert!(matches!(s.identify(&bar).unwrap(), Match::CrcOnly(v) if v[0].name == "Bar (Europe)"));
}

#[test]
fn identify_disc_by_hash_then_serial() {
    use crate::{DiscMatch, Match};
    use rombro_core::disc::{DiscId, DiscKind, Platform};
    use rombro_core::{Hashes, ScannedDisc, ScannedRom};
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    write_rdb(
        &dir.path().join("Sony - PlayStation 2.rdb"),
        &[map(&[
            ("name", F::S("Multi (USA) (Disc 1)")),
            ("serial", F::S("SLUS-20001-0")),
        ])],
    );
    let mut s = Store::open_in_memory().unwrap();
    s.sync_rdbs(dir.path()).unwrap();
    let track = |crc, sha1| ScannedRom {
        path: "t.bin".into(),
        member: None,
        hashes: Hashes {
            size: 1024,
            crc,
            sha1,
            md5: None,
        },
        header: None,
        headerless: None,
    };
    let disc = |tracks, id: Option<(Platform, &str)>| ScannedDisc {
        path: "d.cue".into(),
        kind: DiscKind::Cue,
        id: id.map(|(platform, s)| DiscId {
            platform,
            serial: s.into(),
        }),
        tracks,
        missing: vec![],
    };
    // Audio track unknown, data track verified.
    let d = disc(
        vec![track(5, [0; 20]), track(0xdead_beef, [1; 20])],
        Some((Platform::Ps1, "SLPS-00001")),
    );
    assert!(
        matches!(s.identify_disc(&d).unwrap(), DiscMatch::Hash(Match::Verified(v)) if v[0].name == "Foo (USA)")
    );
    // No hash match: serial wins, restricted to the platform's system.
    let d = disc(vec![track(5, [0; 20])], Some((Platform::Ps1, "SLPS-00001")));
    assert!(
        matches!(s.identify_disc(&d).unwrap(), DiscMatch::Serial(v) if v[0].name == "Baz (Japan)")
    );
    let d = disc(vec![], Some((Platform::Ps2, "SLPS-00001")));
    assert_eq!(s.identify_disc(&d).unwrap(), DiscMatch::Unknown);
    // Multi-disc suffix fallback.
    let d = disc(vec![], Some((Platform::Ps2, "SLUS-20001")));
    assert!(
        matches!(s.identify_disc(&d).unwrap(), DiscMatch::Serial(v) if v[0].name == "Multi (USA) (Disc 1)")
    );
    assert_eq!(
        s.identify_disc(&disc(vec![], None)).unwrap(),
        DiscMatch::Unknown
    );
}
