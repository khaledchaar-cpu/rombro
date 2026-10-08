use super::*;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const SYS: &str = "Nintendo - SNES";

fn game(name: &str) -> Ident {
    Ident::Known(Game {
        system: SYS.into(),
        name: name.into(),
        crc: Some(1),
    })
}

fn zip(path: &Path, members: &[&str]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut w = zip::ZipWriter::new(File::create(path).unwrap());
    for m in members {
        w.start_file(*m, zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(m.as_bytes()).unwrap();
    }
    w.finish().unwrap();
}

fn member(archive: &Path, name: &str, ident: Ident) -> Item {
    Item {
        files: Files::Member {
            archive: archive.to_path_buf(),
            member: name.into(),
        },
        ident,
        in_library: false,
    }
}

fn opts(mode: Mode) -> Options {
    Options {
        mode,
        // these tests cover the quarantine; `unknown_to_trash` has its own test
        rules: crate::g1r::Rules {
            unknown_to_trash: false,
            ..Default::default()
        },
        playlists: None,
        verdicts: Default::default(),
        inbox: None,
        ignore: Vec::new(),
    }
}

fn tree(root: &Path) -> Vec<String> {
    let mut v: Vec<String> = walkdir::WalkDir::new(root)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().strip_prefix(root).unwrap().display().to_string())
        .collect();
    v.sort();
    v
}

#[test]
fn archive_with_any_unknown_member_stays_whole() {
    // e.g. a multi-disk game where only disk 2 matches: extracting it would leave a broken game
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let set = inbox.join("set.zip");
    zip(&set, &["mario.sfc", "x.bin"]);
    let items = [
        member(&set, "mario.sfc", game("Mario (Europe)")),
        member(&set, "x.bin", Ident::Unknown),
    ];
    let plan = build(&items, &lib, &opts(Mode::Move));
    assert_eq!((plan.placed, plan.quarantined), (0, 1));
    let ex = execute(&plan.ops);
    assert!(ex.error.is_none());
    assert_eq!(tree(&lib), ["_quarantine/set.zip"]);
    assert!(undo(&ex.done).is_empty());
    assert_eq!(tree(&inbox), ["set.zip"]);
    assert!(tree(&lib).is_empty());
}

#[test]
fn quarantines_unknown_archive_whole_and_trashes_emptied_one() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let (arcade, snes) = (inbox.join("arcade.zip"), inbox.join("snes.zip"));
    zip(&arcade, &["a.prom", "b.rom"]);
    zip(&snes, &["mario.sfc"]);
    let items = [
        member(&arcade, "a.prom", Ident::Unknown),
        member(&arcade, "b.rom", Ident::Unknown),
        member(&snes, "mario.sfc", game("Mario (Europe)")),
    ];
    let plan = build(&items, &lib, &opts(Mode::Move));
    assert!(plan.decisions.is_empty());
    assert_eq!(plan.quarantined, 1);
    assert!(execute(&plan.ops).error.is_none());
    assert_eq!(
        tree(&lib),
        [
            "Nintendo - SNES/Mario (Europe).sfc",
            "_quarantine/arcade.zip",
            "_trash/snes.zip"
        ]
    );
}

#[test]
fn keeps_archive_with_open_decisions_or_in_copy_mode() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let set = inbox.join("set.zip");
    zip(&set, &["e.sfc", "u.sfc"]);
    let items = [
        member(&set, "e.sfc", game("Mario (Europe)")),
        member(&set, "u.sfc", game("Mario (USA)")),
    ];
    let plan = build(&items, &lib, &opts(Mode::Move));
    assert!(matches!(plan.decisions[..], [Decision::Rejected { .. }]));
    assert!(plan.ops.iter().all(|op| matches!(op, Op::Extract { .. })));

    let one = [member(&set, "e.sfc", game("Mario (Europe)"))];
    let plan = build(&one, &lib, &opts(Mode::Copy));
    assert_eq!(plan.ops.len(), 1);
}

#[test]
fn extracts_archived_disc_with_renamed_tracks() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let set = inbox.join("disc.zip");
    let cue = "FILE \"g 1.bin\" BINARY\n  TRACK 01 MODE2/2352\nFILE \"g 2.bin\" BINARY\n  TRACK 02 AUDIO\n";
    fs::create_dir_all(&inbox).unwrap();
    let mut w = zip::ZipWriter::new(File::create(&set).unwrap());
    for (name, data) in [("d/g.cue", cue), ("d/g 1.bin", "one"), ("d/g 2.bin", "two")] {
        w.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(data.as_bytes()).unwrap();
    }
    w.finish().unwrap();
    let items = [Item {
        files: Files::ArchivedSheet {
            archive: set.clone(),
            sheet: "d/g.cue".into(),
            tracks: vec!["d/g 1.bin".into(), "d/g 2.bin".into()],
        },
        ident: game("Game (Europe)"),
        in_library: false,
    }];
    let ex = execute(&build(&items, &lib, &opts(Mode::Move)).ops);
    assert!(ex.error.is_none(), "{:?}", ex.error);
    assert_eq!(
        tree(&lib),
        [
            "Nintendo - SNES/Game (Europe) (Track 1).bin",
            "Nintendo - SNES/Game (Europe) (Track 2).bin",
            "Nintendo - SNES/Game (Europe).cue",
            "_trash/disc.zip"
        ]
    );
    let new_cue = fs::read_to_string(lib.join("Nintendo - SNES/Game (Europe).cue")).unwrap();
    assert!(new_cue.contains("FILE \"Game (Europe) (Track 2).bin\""));
    assert!(undo(&ex.done).is_empty());
    assert!(tree(&lib).is_empty());
    assert_eq!(tree(&inbox), ["disc.zip"]);
}

fn arcade(system: &str, name: &str) -> Game {
    Game {
        system: system.into(),
        name: name.into(),
        crc: Some(1),
    }
}

#[test]
fn places_romsets_by_short_name_with_chds_and_bios_apart() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let fb = "FBNeo - Arcade Games";
    let set = |name: &str, chds: Vec<PathBuf>| Files::Set {
        archive: inbox.join(name),
        chds,
        alt: vec![],
        dat_note: String::new(),
    };
    zip(&inbox.join("burningf.zip"), &["a"]);
    zip(&inbox.join("burningfh.zip"), &["b"]);
    zip(&inbox.join("kinst.zip"), &["c"]);
    zip(&inbox.join("neogeo.zip"), &["d"]);
    fs::create_dir_all(inbox.join("kinst")).unwrap();
    fs::write(inbox.join("kinst/kinst.chd"), b"hd").unwrap();
    let items = [
        Item {
            files: set("burningf.zip", vec![]),
            ident: Ident::Known(arcade(fb, "Burning Fight (NGM-018 ~ NGH-018)")),
            in_library: false,
        },
        Item {
            files: set("burningfh.zip", vec![]),
            ident: Ident::Known(arcade(fb, "Burning Fight Special (NGH-018, US)")),
            in_library: false,
        },
        Item {
            files: set("kinst.zip", vec![inbox.join("kinst/kinst.chd")]),
            ident: Ident::Known(arcade("MAME", "Killer Instinct (v1.5d)")),
            in_library: false,
        },
        Item {
            files: Files::Set {
                archive: inbox.join("neogeo.zip"),
                chds: vec![],
                alt: vec![arcade("MAME", "neogeo"), arcade("MAME 2010", "neogeo")],
                dat_note: String::new(),
            },
            ident: Ident::Bios(arcade(fb, "Neo Geo")),
            in_library: false,
        },
    ];
    let plan = build(&items, &lib, &opts(Mode::Move));
    assert!(plan.decisions.is_empty(), "{:?}", plan.decisions);
    assert!(execute(&plan.ops).error.is_none());
    let files: Vec<String> = tree(&lib)
        .into_iter()
        .filter(|f| !f.starts_with("_playlists"))
        .collect();
    assert_eq!(
        files,
        [
            "FBNeo - Arcade Games/burningf.zip",
            "FBNeo - Arcade Games/burningfh.zip",
            "MAME/kinst.zip",
            "MAME/kinst/kinst.chd",
            "MAME/neogeo.zip",
            "_bios/fbneo/neogeo.zip"
        ]
    );
    // re-planning the library changes nothing
    let lib_items: Vec<Item> = items
        .iter()
        .map(|it| {
            let mut it = it.clone();
            let rel = |p: &Path| p.strip_prefix(&inbox).unwrap().to_path_buf();
            it.files = match &it.files {
                Files::Set {
                    archive,
                    chds,
                    alt,
                    dat_note,
                } => Files::Set {
                    alt: alt.clone(),
                    dat_note: dat_note.clone(),
                    archive: lib.join(match &it.ident {
                        Ident::Bios(_) => Path::new("_bios/fbneo").join(rel(archive)),
                        Ident::Known(g) => Path::new(&g.system).join(rel(archive)),
                        _ => unreachable!(),
                    }),
                    chds: chds.iter().map(|c| lib.join("MAME").join(rel(c))).collect(),
                },
                f => f.clone(),
            };
            it.in_library = true;
            it
        })
        .collect();
    let mut lib_items = lib_items;
    let mut copy = lib_items.last().unwrap().clone();
    if let Files::Set { archive, .. } = &mut copy.files {
        *archive = lib.join("MAME/neogeo.zip");
    }
    lib_items.push(copy);
    let again = build(&lib_items, &lib, &opts(Mode::Move));
    assert!(
        again.ops.iter().all(|op| matches!(op, Op::Write { .. })),
        "{:?}",
        again.ops
    );
}

#[test]
fn second_copy_of_a_romset_is_a_duplicate_not_a_conflict() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    zip(&inbox.join("720.zip"), &["a"]);
    zip(&inbox.join("old/720.zip"), &["a"]);
    let items: Vec<Item> = ["720.zip", "old/720.zip"]
        .iter()
        .map(|p| Item {
            files: Files::Set {
                archive: inbox.join(p),
                chds: vec![],
                alt: vec![],
                dat_note: String::new(),
            },
            ident: Ident::Known(arcade("MAME", "720 Degrees (rev 4)")),
            in_library: false,
        })
        .collect();
    let plan = build(&items, &lib, &opts(Mode::Move));
    assert_eq!(plan.placed, 1);
    assert!(
        matches!(&plan.decisions[..], [Decision::Rejected { path, reason, .. }]
            if path == &inbox.join("old/720.zip") && reason == "Duplicate")
    );
}

#[test]
fn second_version_under_the_same_short_name_falls_back_to_the_next_system() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    zip(&inbox.join("a/gradius3.zip"), &["japan"]);
    zip(&inbox.join("b/gradius3.zip"), &["world"]);
    let fb = "FBNeo - Arcade Games";
    let items = [
        Item {
            files: Files::Set {
                archive: inbox.join("a/gradius3.zip"),
                chds: vec![],
                alt: vec![arcade("MAME 2003-Plus", "Other Game (Japan)")],
                dat_note: String::new(),
            },
            ident: Ident::Known(arcade(fb, "Other Game (Japan, version 3)")),
            in_library: false,
        },
        Item {
            files: Files::Set {
                archive: inbox.join("b/gradius3.zip"),
                chds: vec![],
                alt: vec![arcade("MAME 2015", "Gradius III (World)")],
                dat_note: String::new(),
            },
            ident: Ident::Known(arcade(fb, "Gradius III (World, version R)")),
            in_library: false,
        },
    ];
    let plan = build(&items, &lib, &opts(Mode::Move));
    assert!(plan.decisions.is_empty(), "{:?}", plan.decisions);
    assert!(execute(&plan.ops).error.is_none());
    assert_eq!(
        tree(&lib),
        [
            "FBNeo - Arcade Games/gradius3.zip",
            "MAME 2015/gradius3.zip"
        ]
    );
}

#[test]
fn version_listed_in_one_system_only_gets_that_slot() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    zip(&inbox.join("a/pururun.zip"), &["set2"]);
    zip(&inbox.join("b/pururun.zip"), &["set1"]);
    let fb = "FBNeo - Arcade Games";
    let set = |p: &str, alt| Files::Set {
        archive: inbox.join(p),
        chds: vec![],
        alt,
        dat_note: String::new(),
    };
    let items = [
        // listed first, but it could also go to MAME 2003-Plus
        Item {
            files: set(
                "a/pururun.zip",
                vec![arcade("MAME 2003-Plus", "Pururun Two")],
            ),
            ident: Ident::Known(arcade(fb, "Pururun Two (set 2)")),
            in_library: false,
        },
        Item {
            files: set("b/pururun.zip", vec![]),
            ident: Ident::Known(arcade(fb, "Pururun (set 1)")),
            in_library: false,
        },
    ];
    let plan = build(&items, &lib, &opts(Mode::Move));
    assert!(plan.decisions.is_empty(), "{:?}", plan.decisions);
    assert!(plan.ops.iter().any(
        |op| op.source() == Some(inbox.join("b/pururun.zip").as_path())
            && op.target() == lib.join("FBNeo - Arcade Games/pururun.zip")
    ));
    assert!(execute(&plan.ops).error.is_none());
    assert_eq!(
        tree(&lib),
        [
            "FBNeo - Arcade Games/pururun.zip",
            "MAME 2003-Plus/pururun.zip"
        ]
    );
}

#[test]
fn multi_disk_archive_becomes_one_game_folder_with_m3u() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let a = inbox.join("Afterburner (Europe).zip");
    let (d1, d2) = (
        "Afterburner (Europe) (Disk A).ipf",
        "Afterburner (Europe) (Disk B).ipf",
    );
    zip(&a, &[d1, d2]);
    // the database names the two disks inconsistently
    let items = [
        member(&a, d1, game("Afterburner (Europe) (Disk 1)")),
        member(&a, d2, game("Afterburner (Europe)")),
    ];
    let mut o = opts(Mode::Move);
    o.playlists = Some(lib.join("_playlists"));
    let plan = build(&items, &lib, &o);
    assert!(plan.decisions.is_empty(), "{:?}", plan.decisions);
    assert_eq!(plan.placed, 1);
    assert!(execute(&plan.ops).error.is_none());
    let dir = format!("{SYS}/Afterburner (Europe)");
    assert_eq!(
        tree(&lib),
        [
            format!("{dir}/Afterburner (Europe) (Disk A).ipf"),
            format!("{dir}/Afterburner (Europe) (Disk B).ipf"),
            format!("{dir}/Afterburner (Europe).m3u"),
            format!("_playlists/{SYS}.lpl"),
            "_trash/Afterburner (Europe).zip".to_owned(),
        ]
    );
    assert_eq!(
        fs::read_to_string(lib.join(&dir).join("Afterburner (Europe).m3u")).unwrap(),
        format!("{d1}\n{d2}\n")
    );

    // re-planning the library keeps the folder as it is
    let lib_items = [
        Item {
            files: Files::Single(lib.join(&dir).join(d1)),
            ident: game("Afterburner (Europe) (Disk 1)"),
            in_library: true,
        },
        Item {
            files: Files::Single(lib.join(&dir).join(d2)),
            ident: game("Afterburner (Europe)"),
            in_library: true,
        },
    ];
    let again = build(&lib_items, &lib, &o);
    assert!(again.decisions.is_empty());
    assert_eq!(again.unchanged, 1);
    assert!(again.ops.iter().all(|op| matches!(op, Op::Write { .. })));
}

#[test]
fn arcade_1g1r_keeps_one_set_per_game() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    zip(&inbox.join("a/gradius3.zip"), &["japan"]);
    zip(&inbox.join("b/gradius3.zip"), &["world"]);
    let fb = "FBNeo - Arcade Games";
    let set = |p: &str, alt| Files::Set {
        archive: inbox.join(p),
        chds: vec![],
        alt,
        dat_note: String::new(),
    };
    let items = [
        Item {
            files: set(
                "a/gradius3.zip",
                vec![arcade("MAME 2003", "Gradius III (Japan)")],
            ),
            ident: Ident::Known(arcade(fb, "Gradius III: Densetsu kara Shinwa e (Japan)")),
            in_library: false,
        },
        Item {
            files: set("b/gradius3.zip", vec![]),
            ident: Ident::Known(arcade(fb, "Gradius III (World, version R)")),
            in_library: false,
        },
    ];
    let plan = build(&items, &lib, &opts(Mode::Move));
    assert_eq!(plan.placed, 1);
    assert!(matches!(&plan.decisions[..],
        [Decision::Rejected { path, kept: Some(k), reason, .. }]
            if path == &inbox.join("a/gradius3.zip") && k == "Gradius III (World, version R)" && reason == "Region"));
}

#[test]
fn duplicate_members_need_no_decision() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let (a, b) = (inbox.join("a.zip"), inbox.join("b.zip"));
    zip(&a, &["bios.sfc", "x.sfc"]);
    zip(&b, &["bios.sfc", "y.sfc"]);
    let items = [
        member(&a, "bios.sfc", game("Bios (Japan)")),
        member(&a, "x.sfc", game("X (Japan)")),
        member(&b, "bios.sfc", game("Bios (Japan)")),
        member(&b, "y.sfc", game("Y (Japan)")),
    ];
    let plan = build(&items, &lib, &opts(Mode::Move));
    assert!(plan.decisions.is_empty());
    assert!(execute(&plan.ops).error.is_none());
    assert_eq!(
        tree(&lib),
        [
            "Nintendo - SNES/Bios (Japan).sfc",
            "Nintendo - SNES/X (Japan).sfc",
            "Nintendo - SNES/Y (Japan).sfc",
            "_trash/a.zip",
            "_trash/b.zip"
        ]
    );
}

#[test]
fn repacked_zips_with_equal_members_are_the_same() {
    let tmp = TempDir::new().unwrap();
    let a = tmp.path().join("a/1944.zip");
    zip(&a, &["x.bin", "y.bin"]);
    // other order, stored instead of deflated: other bytes, same set
    let b = tmp.path().join("b/1944.zip");
    fs::create_dir_all(b.parent().unwrap()).unwrap();
    let mut w = zip::ZipWriter::new(File::create(&b).unwrap());
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for m in ["y.bin", "x.bin"] {
        w.start_file(m, stored).unwrap();
        w.write_all(m.as_bytes()).unwrap();
    }
    w.finish().unwrap();
    assert_ne!(fs::read(&a).unwrap(), fs::read(&b).unwrap());
    assert!(build::same_content(&a, &b));
    let c = tmp.path().join("c/1944.zip");
    zip(&c, &["x.bin"]);
    assert!(!build::same_content(&a, &c));
}

#[test]
fn zipped_disks_of_a_multi_disk_release_are_extracted_for_the_m3u() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let disk = |n: u8| {
        let a = inbox.join(format!("Tenshi (Japan) (Disk {n}).zip"));
        zip(&a, &[&format!("Tenshi (Japan) (Disk {n}).fds")]);
        Item {
            files: Files::Single(a),
            ident: game(&format!("Tenshi (Japan) (Disk {n})")),
            in_library: false,
        }
    };
    let plan = build(&[disk(1), disk(2)], &lib, &opts(Mode::Move));
    assert!(execute(&plan.ops).error.is_none());
    let dir = format!("{SYS}/Tenshi (Japan)");
    assert_eq!(
        tree(&lib),
        [
            format!("{dir}/Tenshi (Japan) (Disk 1).fds"),
            format!("{dir}/Tenshi (Japan) (Disk 2).fds"),
            format!("{dir}/Tenshi (Japan).m3u"),
            "_trash/Tenshi (Japan) (Disk 1).zip".to_owned(),
            "_trash/Tenshi (Japan) (Disk 2).zip".to_owned(),
        ]
    );
    assert_eq!(
        fs::read_to_string(lib.join(&dir).join("Tenshi (Japan).m3u")).unwrap(),
        "Tenshi (Japan) (Disk 1).fds\nTenshi (Japan) (Disk 2).fds\n"
    );
}

#[test]
fn placed_m3u_of_zips_is_repaired_unless_the_core_reads_zips() {
    for (system, fixed) in [
        ("Nintendo - Family Computer Disk System", true),
        ("Atari - ST", false),
    ] {
        let tmp = TempDir::new().unwrap();
        let lib = tmp.path().join("lib");
        let dir = lib.join(system).join("T (Japan)");
        let items: Vec<Item> = (1..=2)
            .map(|n| {
                let a = dir.join(format!("T (Japan) (Disk {n}).zip"));
                zip(&a, &[&format!("T (Japan) (Disk {n}).fds")]);
                Item {
                    files: Files::Single(a),
                    ident: Ident::Known(Game {
                        system: system.into(),
                        name: format!("T (Japan) (Disk {n})"),
                        crc: Some(1),
                    }),
                    in_library: true,
                }
            })
            .collect();
        let m3u = dir.join("T (Japan).m3u");
        fs::write(&m3u, "T (Japan) (Disk 1).zip\nT (Japan) (Disk 2).zip\n").unwrap();
        let mut o = opts(Mode::Move);
        o.playlists = Some(lib.join("_playlists"));
        let plan = build(&items, &lib, &o);
        assert!(execute(&plan.ops).error.is_none());
        // FCEUmm cannot read an m3u: its playlist starts disk 1
        let lpl = fs::read_to_string(lib.join(format!("_playlists/{system}.lpl"))).unwrap();
        assert_eq!(lpl.contains("(Disk 1).fds"), fixed, "{lpl}");
        let text = fs::read_to_string(&m3u).unwrap();
        assert_eq!(text.contains(".fds"), fixed, "{system}: {text}");
        assert_eq!(dir.join("T (Japan) (Disk 1).zip").exists(), !fixed);
    }
}
