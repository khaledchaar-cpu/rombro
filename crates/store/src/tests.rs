use crate::Store;
use std::path::{Path, PathBuf};

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
            variant: None,
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

#[test]
fn import_end_to_end_with_resolution_journal_and_undo() {
    use rombro_core::MultiHasher;
    use rombro_core::plan::{self, Decision, Ident, Mode, Options};
    let hash = |data: &[u8]| {
        let mut h = MultiHasher::new();
        h.update(data);
        h.finish()
    };
    let entry = |name: &'static str, h: &rombro_core::Hashes| {
        map(&[
            ("name", F::S(name)),
            ("size", F::U(h.size as u32)),
            ("crc", F::B(Box::leak(Box::new(h.crc.to_be_bytes())))),
            ("sha1", F::B(Box::leak(Box::new(h.sha1)))),
        ])
    };
    let (unique, shared) = (hash(b"unique rom"), hash(b"shared rom"));
    let tmp = tempfile::tempdir().unwrap();
    let (rdb, inbox, lib) = (
        tmp.path().join("rdb"),
        tmp.path().join("inbox"),
        tmp.path().join("lib"),
    );
    for d in [&rdb, &inbox] {
        std::fs::create_dir_all(d).unwrap();
    }
    write_rdb(
        &rdb.join("Nintendo - SNES.rdb"),
        &[
            entry("Foo (Europe)", &unique),
            entry("Bar (USA)", &shared),
            entry("Baz (USA)", &shared),
        ],
    );
    std::fs::write(inbox.join("foo.sfc"), b"unique rom").unwrap();
    std::fs::write(inbox.join("amb.sfc"), b"shared rom").unwrap();
    std::fs::write(inbox.join("junk.sfc"), b"junk").unwrap();

    let mut s = Store::open_in_memory().unwrap();
    s.sync_rdbs(&rdb).unwrap();
    let opts = Options {
        mode: Mode::Move,
        rules: Default::default(),
        playlists: None,
        verdicts: Default::default(),
        inbox: None,
        ignore: Vec::new(),
    };
    let items = s.items(&rombro_core::scan(&inbox), false).unwrap();
    let p = plan::build(&items, &lib, &opts);
    assert_eq!((p.placed, p.quarantined), (1, 1));
    assert!(
        matches!(&p.decisions[..], [Decision::Ambiguous { candidates, .. }] if candidates.len() == 2)
    );

    // The user's choice is remembered and applied on the next scan.
    s.set_resolution(&shared.sha1, "Nintendo - SNES", "Baz (USA)")
        .unwrap();
    let items = s.items(&rombro_core::scan(&inbox), false).unwrap();
    assert!(
        items
            .iter()
            .all(|i| !matches!(i.ident, Ident::Ambiguous(_)))
    );
    let p = plan::build(&items, &lib, &opts);
    assert_eq!((p.placed, p.quarantined, p.decisions.len()), (2, 1, 0));

    let ex = plan::execute(&p.ops);
    assert!(ex.error.is_none());
    assert!(lib.join("Nintendo - SNES/Baz (USA).sfc").is_file());
    let lib_str = lib.to_string_lossy();
    s.add_journal(0, &lib_str, &plan::journal_to_json(&ex.done))
        .unwrap();

    let j = s.last_journal().unwrap().unwrap();
    let done = plan::journal_from_json(&j.done).unwrap();
    assert!(plan::undo(&done).is_empty());
    s.mark_undone(j.id).unwrap();
    assert!(s.last_journal().unwrap().is_none());
    assert!(!lib.exists());
    assert_eq!(std::fs::read_dir(&inbox).unwrap().count(), 3);
}

#[test]
fn verdicts_roundtrip() {
    use rombro_core::plan::Verdict;
    let s = Store::open_in_memory().unwrap();
    s.set_verdict("Sys", "A", Some(Verdict::Keep)).unwrap();
    s.set_verdict("Sys", "B", Some(Verdict::Keep)).unwrap();
    s.set_verdict("Sys", "B", Some(Verdict::Discard)).unwrap();
    s.set_verdict("Sys", "C", Some(Verdict::Keep)).unwrap();
    s.set_verdict("Sys", "C", None).unwrap();
    let v = s.verdicts().unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v[&("Sys".into(), "A".into())], Verdict::Keep);
    assert_eq!(v[&("Sys".into(), "B".into())], Verdict::Discard);
}

#[test]
fn journals_newest_first_with_state() {
    let s = Store::open_in_memory().unwrap();
    let a = s.add_journal(1, "/lib", "[]").unwrap();
    let b = s.add_journal(2, "/lib", "[]").unwrap();
    s.mark_undone(b).unwrap();
    let j = s.journals(10).unwrap();
    assert_eq!(j.len(), 2);
    assert_eq!((j[0].0.id, j[0].1.as_str()), (b, "undone"));
    assert_eq!((j[1].0.id, j[1].1.as_str()), (a, "done"));
}

#[test]
fn file_index_follows_scan_execute_and_undo() {
    use rombro_core::plan::{Op, execute, undo};
    let dir = tempfile::tempdir().unwrap();
    let (inbox, lib) = (dir.path().join("in"), dir.path().join("lib"));
    std::fs::create_dir_all(&inbox).unwrap();
    std::fs::create_dir_all(&lib).unwrap();
    let src = inbox.join("a.gb");
    std::fs::write(&src, [7u8; 512]).unwrap();
    let store = Store::open_in_memory().unwrap();
    store.set_library(&lib).unwrap();
    assert_eq!(store.library().unwrap().as_deref(), Some(lib.as_path()));

    store.save_scan(&inbox, &rombro_core::scan(&inbox)).unwrap();
    assert!(store.hash_cache(&inbox).unwrap().get(&src).is_some());
    assert!(store.hash_cache(&lib).unwrap().0.is_empty());

    let to = lib.join("GB/a.gb");
    let ex = execute(&[Op::Move {
        from: src.clone(),
        to: to.clone(),
    }]);
    store.index_executed(&ex.done).unwrap();
    let hit = store.hash_cache(&lib).unwrap().get(&to).unwrap();
    assert_eq!(hit[0].path, to);
    let added = store.added_times(&lib).unwrap()[&to];
    assert!(added > 0);
    // a rename inside the library keeps the time; a rescan does too
    let renamed = lib.join("GB/b.gb");
    store
        .conn
        .execute(
            "UPDATE file SET added = 1 WHERE path = ?1",
            [to.to_str().unwrap()],
        )
        .unwrap();
    let ex2 = execute(&[Op::Move {
        from: to.clone(),
        to: renamed.clone(),
    }]);
    store.index_executed(&ex2.done).unwrap();
    store.save_scan(&lib, &rombro_core::scan(&lib)).unwrap();
    assert_eq!(store.added_times(&lib).unwrap()[&renamed], 1);
    assert!(undo(&ex2.done).is_empty());
    store.index_undone(&ex2.done).unwrap();
    assert!(store.hash_cache(&inbox).unwrap().0.is_empty());

    assert!(undo(&ex.done).is_empty());
    store.index_undone(&ex.done).unwrap();
    assert!(store.hash_cache(&inbox).unwrap().get(&src).is_some());

    std::fs::remove_file(&src).unwrap();
    store.save_scan(&inbox, &rombro_core::scan(&inbox)).unwrap();
    assert!(store.hash_cache(&inbox).unwrap().0.is_empty());
}

#[test]
fn gamify_completeness_meta_and_persisted_unlocks() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let mut s = Store::open_in_memory().unwrap();
    s.sync_rdbs(dir.path()).unwrap();
    let owned = [("Nintendo - SNES".to_string(), "Foo (USA)".to_string())];
    let st = s.gamify(&owned, 0, 0, 86_400 * 10).unwrap();
    let snes = &st.kpis.systems[0];
    assert_eq!((snes.owned, snes.total), (1, 2));
    assert_eq!(st.kpis.genres["Action"], 1);
    assert!(st.new.contains(&"games-1".to_string()));
    assert!(st.new.contains(&"clean-sweep".to_string()));
    // unlocks persist even when the condition no longer holds
    let st = s.gamify(&owned, 3, 0, 86_400 * 11).unwrap();
    assert!(st.new.is_empty());
    let clean = st
        .achievements
        .iter()
        .find(|(a, _)| a.id == "clean-sweep")
        .unwrap();
    assert_eq!((clean.0.unlocked, clean.1), (true, Some(86_400 * 10)));
}

#[test]
fn arcade_chip_inside_unknown_zip_is_not_identified() {
    use rombro_core::MultiHasher;
    use rombro_core::plan::Ident;
    use std::io::Write;
    let mut h = MultiHasher::new();
    h.update(b"prom chip");
    let chip = h.finish();
    let tmp = tempfile::tempdir().unwrap();
    let (rdb, inbox) = (tmp.path().join("rdb"), tmp.path().join("inbox"));
    for d in [&rdb, &inbox] {
        std::fs::create_dir_all(d).unwrap();
    }
    write_rdb(
        &rdb.join("MAME.rdb"),
        &[map(&[
            ("name", F::S("Get Star (bootleg set 1)")),
            ("crc", F::B(Box::leak(Box::new(chip.crc.to_be_bytes())))),
            ("sha1", F::B(Box::leak(Box::new(chip.sha1)))),
        ])],
    );
    let mut w = zip::ZipWriter::new(std::fs::File::create(inbox.join("alcon.zip")).unwrap());
    for (name, data) in [("a.8b", &b"prom chip"[..]), ("b.6g", b"other chip")] {
        w.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap();

    let mut s = Store::open_in_memory().unwrap();
    s.sync_rdbs(&rdb).unwrap();
    let items = s.items(&rombro_core::scan(&inbox), false).unwrap();
    assert!(!items.is_empty());
    assert!(items.iter().all(|it| matches!(it.ident, Ident::Unknown)));
}

#[test]
fn exceptions_roundtrip() {
    let s = Store::open_in_memory().unwrap();
    assert!(s.ignored().unwrap().is_empty());
    s.set_ignored(&[PathBuf::from("/roms/keep")]).unwrap();
    assert_eq!(s.ignored().unwrap(), [PathBuf::from("/roms/keep")]);
    s.set_resolution(&[1, 2], "Sys", "Game").unwrap();
    assert_eq!(s.resolutions().unwrap().len(), 1);
    s.clear_resolution(&[1, 2]).unwrap();
    assert!(s.resolutions().unwrap().is_empty());
}

#[test]
fn arcade_set_goes_to_first_core_whose_dat_it_completes() {
    use rombro_core::arcade::dat;
    use rombro_core::plan::Ident;
    use std::io::Write;
    let tmp = tempfile::tempdir().unwrap();
    let (rdb, inbox) = (tmp.path().join("rdb"), tmp.path().join("inbox"));
    for d in [&rdb, &inbox] {
        std::fs::create_dir_all(d).unwrap();
    }
    let zip = inbox.join("1943.zip");
    let mut w = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
    for (name, data) in [("a.bin", &b"chip a"[..]), ("b.bin", b"chip b")] {
        w.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap();
    let mut h = rombro_core::MultiHasher::new();
    h.update(&std::fs::read(&zip).unwrap());
    let whole = h.finish();
    for db in ["MAME", "MAME 2003-Plus"] {
        write_rdb(
            &rdb.join(format!("{db}.rdb")),
            &[map(&[
                ("name", F::S("1943: The Battle of Midway (Euro)")),
                ("rom_name", F::S("1943.zip")),
                ("crc", F::B(Box::leak(Box::new(whole.crc.to_be_bytes())))),
                ("sha1", F::B(Box::leak(Box::new(whole.sha1)))),
            ])],
        );
    }
    let crc = |d: &[u8]| format!("{:08x}", crc32fast::hash(d));
    let dat = |roms: &[(&str, &[u8])]| {
        let r: String = roms
            .iter()
            .map(|(n, d)| format!(r#"<rom name="{n}" size="6" crc="{}"/>"#, crc(d)))
            .collect();
        dat::parse(format!(r#"<mame><machine name="1943">{r}</machine></mame>"#).as_bytes())
            .unwrap()
    };
    let mut s = Store::open_in_memory().unwrap();
    s.sync_rdbs(&rdb).unwrap();
    let ident = |s: &Store| {
        s.items(&rombro_core::scan(&inbox), false).unwrap()[0]
            .ident
            .clone()
    };
    // without DATs: database order (MAME first)
    assert!(matches!(ident(&s), Ident::Known(g) if g.system == "MAME"));

    let old: &[(&str, &[u8])] = &[("a.bin", b"chip a"), ("b.bin", b"chip b")];
    s.import_dat(
        "MAME",
        "0.289",
        0,
        &dat(&[("a.bin", b"chip a"), ("c.bin", b"new")]),
    )
    .unwrap();
    s.import_dat("MAME 2003-Plus", "x", 0, &dat(old)).unwrap();
    assert!(matches!(ident(&s), Ident::Known(g) if g.system == "MAME 2003-Plus"));

    s.import_dat("MAME 2003-Plus", "x", 0, &dat(&[("a.bin", b"chip b")]))
        .unwrap();
    let Ident::Incomplete(why) = ident(&s) else {
        panic!()
    };
    assert_eq!(
        why,
        "MAME: 1 missing (c.bin); MAME 2003-Plus: 1 misnamed (a.bin)"
    );
}
