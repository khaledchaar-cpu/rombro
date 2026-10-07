use super::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const SYS: &str = "Nintendo - SNES";

fn game(name: &str) -> Ident {
    Ident::Known(Game {
        system: SYS.into(),
        name: name.into(),
        crc: Some(1),
    })
}

fn file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let p = dir.join(name);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, content).unwrap();
    p
}

fn item(path: PathBuf, ident: Ident, in_library: bool) -> Item {
    Item {
        files: Files::Single(path),
        ident,
        in_library,
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

/// Sorted relative paths of all files below `root`.
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
fn import_quarantine_queue_and_undo() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let items = [
        item(file(&inbox, "a.sfc", "a"), game("Mario (USA)"), false),
        item(
            file(&inbox, "sub/b.sfc", "b"),
            game("Mario (Europe)"),
            false,
        ),
        item(file(&inbox, "x.bin", "x"), Ident::Unknown, false),
    ];
    let before = tree(tmp.path());
    let plan = build(&items, &lib, &opts(Mode::Move));
    assert_eq!((plan.placed, plan.quarantined), (1, 1));
    assert_eq!(plan.why.len(), plan.ops.len());
    assert!(
        plan.why
            .iter()
            .any(|w| w.rule == crate::rules::Rule::G1rPick)
    );
    assert!(
        plan.why
            .iter()
            .any(|w| w.rule == crate::rules::Rule::Quarantine)
    );
    assert!(
        matches!(&plan.decisions[..], [Decision::Rejected { name, kept: Some(k), .. }] if name == "Mario (USA)" && k == "Mario (Europe)")
    );
    assert_eq!(tree(tmp.path()), before, "planning must not touch the disk");

    let ex = execute(&plan.ops);
    assert!(ex.error.is_none());
    assert_eq!(
        tree(&lib),
        ["Nintendo - SNES/Mario (Europe).sfc", "_quarantine/x.bin"]
    );
    assert!(undo(&ex.done).is_empty());
    assert_eq!(tree(tmp.path()), before);
    assert!(!lib.exists(), "created directories are removed on undo");
}

#[test]
fn copy_mode_leaves_inbox_and_rejects_alone() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let items = [
        item(file(&inbox, "a.sfc", "a"), game("Mario (USA)"), false),
        item(file(&inbox, "b.sfc", "b"), game("Mario (Europe)"), false),
    ];
    let ex = execute(&build(&items, &lib, &opts(Mode::Copy)).ops);
    assert!(ex.error.is_none());
    assert_eq!(tree(&inbox), ["a.sfc", "b.sfc"]);
    assert_eq!(tree(&lib), ["Nintendo - SNES/Mario (Europe).sfc"]);
    assert!(undo(&ex.done).is_empty());
}

#[test]
fn audit_moves_misnamed_and_keeps_correct_files() {
    let tmp = TempDir::new().unwrap();
    let lib = tmp.path().to_path_buf();
    let items = [
        item(
            file(&lib, "Nintendo - SNES/Zelda (Europe).sfc", "z"),
            game("Zelda (Europe)"),
            true,
        ),
        item(
            file(&lib, "misc/mario.SFC", "m"),
            game("Mario (Europe)"),
            true,
        ),
        item(file(&lib, "_quarantine/old.bin", "o"), Ident::Unknown, true),
    ];
    let plan = build(&items, &lib, &opts(Mode::Copy));
    assert_eq!((plan.unchanged, plan.placed, plan.quarantined), (1, 1, 0));
    assert!(
        matches!(plan.ops[..], [Op::Move { .. }]),
        "library files are always moved"
    );
}

#[test]
fn conflicts_and_ambiguous_become_decisions() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    file(&lib, "Nintendo - SNES/Mario (Europe).sfc", "other");
    let amb = Ident::Ambiguous(vec![]);
    let items = [
        item(file(&inbox, "m.sfc", "m"), game("Mario (Europe)"), false),
        item(file(&inbox, "a.sfc", "a"), amb, false),
    ];
    let plan = build(&items[..1], &lib, &opts(Mode::Move));
    assert!(plan.ops.is_empty());
    assert!(matches!(plan.decisions[..], [Decision::Conflict { .. }]));
    let plan = build(&items[1..], &lib, &opts(Mode::Move));
    assert!(matches!(plan.decisions[..], [Decision::Ambiguous { .. }]));
    assert!(plan.ops.is_empty());
}

#[test]
fn tie_is_left_untouched() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let items = [
        item(
            file(&inbox, "1.sfc", "1"),
            game("Game (USA) (Capcom Town)"),
            false,
        ),
        item(file(&inbox, "2.sfc", "2"), game("Game (USA)"), false),
    ];
    let plan = build(&items, &lib, &opts(Mode::Move));
    assert!(matches!(plan.decisions[..], [Decision::Tie { .. }]));
    assert!(plan.ops.is_empty());
}

#[test]
fn multi_disc_cue_sets_get_folder_m3u_and_rewritten_sheets() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let disc = |n: u8| {
        let cue = file(
            &inbox,
            &format!("ff{n}.cue"),
            &format!("FILE \"ff{n}.bin\" BINARY\n  TRACK 01 MODE2/2352\n"),
        );
        Item {
            files: Files::Sheet {
                sheet: cue,
                tracks: vec![file(&inbox, &format!("ff{n}.bin"), "d")],
            },
            ident: Ident::Known(Game {
                system: "Sony - PlayStation".into(),
                name: format!("FF (USA) (Disc {n})"),
                crc: None,
            }),
            in_library: false,
        }
    };
    let mut o = opts(Mode::Move);
    o.playlists = Some(lib.join("playlists"));
    let plan = build(&[disc(1), disc(2)], &lib, &o);
    assert!(execute(&plan.ops).error.is_none());
    let dir = "Sony - PlayStation/FF (USA)";
    assert_eq!(
        tree(&lib),
        [
            format!("{dir}/FF (USA) (Disc 1).bin"),
            format!("{dir}/FF (USA) (Disc 1).cue"),
            format!("{dir}/FF (USA) (Disc 2).bin"),
            format!("{dir}/FF (USA) (Disc 2).cue"),
            format!("{dir}/FF (USA).m3u"),
            "playlists/Sony - PlayStation.lpl".into(),
        ]
    );
    let d = lib.join(dir);
    assert_eq!(
        fs::read_to_string(d.join("FF (USA).m3u")).unwrap(),
        "FF (USA) (Disc 1).cue\nFF (USA) (Disc 2).cue\n"
    );
    assert!(
        fs::read_to_string(d.join("FF (USA) (Disc 2).cue"))
            .unwrap()
            .starts_with("FILE \"FF (USA) (Disc 2).bin\"")
    );
    let lpl = fs::read_to_string(lib.join("playlists/Sony - PlayStation.lpl")).unwrap();
    assert!(lpl.contains("\"label\": \"FF (USA)\""));
}

#[test]
fn verdicts_keep_or_trash_rejected_releases() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let items = [
        item(file(&inbox, "a.sfc", "a"), game("Mario (USA)"), false),
        item(file(&inbox, "b.sfc", "b"), game("Mario (Europe)"), false),
        item(file(&inbox, "c.sfc", "c"), game("Mario (Japan)"), false),
    ];
    let mut o = opts(Mode::Copy);
    let key = |n: &str| (SYS.to_owned(), n.to_owned());
    o.verdicts.insert(key("Mario (USA)"), Verdict::Keep);
    o.verdicts.insert(key("Mario (Japan)"), Verdict::Discard);
    let plan = build(&items, &lib, &o);
    assert!(plan.decisions.is_empty());
    assert_eq!((plan.placed, plan.discarded), (2, 1));
    let ex = execute(&plan.ops);
    assert!(ex.error.is_none());
    assert_eq!(
        tree(&lib),
        [
            "Nintendo - SNES/Mario (Europe).sfc",
            "Nintendo - SNES/Mario (USA).sfc",
            "_trash/c.sfc"
        ]
    );
    assert_eq!(tree(&inbox), ["a.sfc", "b.sfc"], "discard always moves");
    assert!(undo(&ex.done).is_empty());
    assert_eq!(tree(&inbox), ["a.sfc", "b.sfc", "c.sfc"]);
}

#[test]
fn keep_is_ignored_for_duplicates() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let items = [
        item(file(&inbox, "a.sfc", "a"), game("Mario (Europe)"), false),
        item(file(&inbox, "a2.sfc", "a"), game("Mario (Europe)"), false),
    ];
    let mut o = opts(Mode::Move);
    o.verdicts
        .insert((SYS.into(), "Mario (Europe)".into()), Verdict::Keep);
    let plan = build(&items, &lib, &o);
    assert!(
        matches!(&plan.decisions[..], [Decision::Rejected { reason, .. }] if reason == "Duplicate"),
        "{:?}",
        plan.decisions
    );
    o.verdicts
        .insert((SYS.into(), "Mario (Europe)".into()), Verdict::Discard);
    let plan = build(&items, &lib, &o);
    assert!(plan.decisions.is_empty());
    assert_eq!((plan.placed, plan.discarded), (1, 1));
}

#[test]
fn prefer_resolves_tie_and_rejects_the_rest() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let items = [
        item(
            file(&inbox, "1.sfc", "1"),
            game("Game (USA) (Capcom Town)"),
            false,
        ),
        item(file(&inbox, "2.sfc", "2"), game("Game (USA)"), false),
    ];
    let mut o = opts(Mode::Move);
    let key = |n: &str| (SYS.to_owned(), n.to_owned());
    o.verdicts
        .insert(key("Game (USA) (Capcom Town)"), Verdict::Prefer);
    let plan = build(&items, &lib, &o);
    assert_eq!(plan.placed, 1);
    assert!(
        matches!(&plan.decisions[..], [Decision::Rejected { name, kept: Some(k), .. }] if name == "Game (USA)" && k == "Game (USA) (Capcom Town)"),
        "{:?}",
        plan.decisions
    );
    o.verdicts.insert(key("Game (USA)"), Verdict::Discard);
    let plan = build(&items, &lib, &o);
    assert!(plan.decisions.is_empty());
    assert_eq!((plan.placed, plan.discarded), (1, 1));
}

#[test]
fn quarantine_keeps_inbox_subfolders_so_equal_names_do_not_clash() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let items = [
        item(file(&inbox, "x.zip", "1"), Ident::Unknown, false),
        item(file(&inbox, "old/x.zip", "2"), Ident::Unknown, false),
        item(file(&inbox, "old/a.sfc", "a"), game("Mario (USA)"), false),
    ];
    let o = Options {
        inbox: Some(inbox.clone()),
        ignore: Vec::new(),
        ..opts(Mode::Move)
    };
    let plan = build(&items, &lib, &o);
    assert_eq!(plan.quarantined, 2);
    assert!(plan.decisions.is_empty());
    execute(&plan.ops);
    assert_eq!(
        tree(&lib),
        [
            "Nintendo - SNES/Mario (USA).sfc",
            "_quarantine/old/x.zip",
            "_quarantine/x.zip"
        ]
    );
}

#[test]
fn unknown_files_in_folders_without_matches_stay_put() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let items = [
        item(
            file(&inbox, "Daphne/ace.daphne/ace.txt", "1"),
            Ident::Unknown,
            false,
        ),
        item(file(&inbox, "loose.bin", "2"), Ident::Unknown, false),
        item(
            file(&inbox, "00bios/dc/dc_nvmem.bin", "3"),
            Ident::Unknown,
            false,
        ),
        item(
            file(&inbox, "00bios/scph.bin", "4"),
            Ident::Bios(Game {
                system: "Sony - PlayStation".into(),
                name: "[BIOS] PS (USA)".into(),
                crc: Some(1),
            }),
            false,
        ),
    ];
    let o = Options {
        inbox: Some(inbox.clone()),
        ignore: Vec::new(),
        ..opts(Mode::Move)
    };
    let plan = build(&items, &lib, &o);
    assert_eq!(plan.quarantined, 1);
    execute(&plan.ops);
    // recognized BIOS is placed, the rest of the BIOS folder stays put
    assert_eq!(tree(&lib), ["_bios/scph.bin", "_quarantine/loose.bin"]);
}

#[test]
fn game_folder_moves_whole_under_its_own_name() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let dos = |name: &str| {
        Ident::Known(Game {
            system: "DOS".into(),
            name: name.into(),
            crc: Some(1),
        })
    };
    let items = [
        item(
            file(&inbox, "PC - DOS/Abuse.dos/ABUSE.EXE", "e"),
            dos("Abuse (1995)"),
            false,
        ),
        item(
            file(&inbox, "PC - DOS/Abuse.dos/ADDON/x.lsp", "x"),
            Ident::Unknown,
            false,
        ),
        item(
            file(&inbox, "PC - DOS/Doom/DOOM.EXE", "d"),
            dos("Doom (1993)"),
            false,
        ),
        item(
            file(&inbox, "PC - DOS/Doom/data/a.wad", "w"),
            Ident::Unknown,
            false,
        ),
    ];
    let o = Options {
        inbox: Some(inbox.clone()),
        ignore: Vec::new(),
        ..opts(Mode::Move)
    };
    let plan = build(&items, &lib, &o);
    assert_eq!((plan.placed, plan.quarantined), (2, 0));
    assert!(plan.decisions.is_empty());
    execute(&plan.ops);
    assert_eq!(
        tree(&lib),
        [
            "DOS/Abuse/ABUSE.EXE",
            "DOS/Abuse/ADDON/x.lsp",
            "DOS/Doom/DOOM.EXE",
            "DOS/Doom/data/a.wad"
        ]
    );
    // re-planning the library changes nothing
    let lib_items = [
        item(lib.join("DOS/Abuse/ABUSE.EXE"), dos("Abuse (1995)"), true),
        item(lib.join("DOS/Abuse/ADDON/x.lsp"), Ident::Unknown, true),
    ];
    let again = build(&lib_items, &lib, &opts(Mode::Move));
    assert!(again.ops.is_empty() && again.decisions.is_empty());
    assert_eq!(again.unchanged, 1);
}

#[test]
fn game_folder_keeps_its_name_when_the_key_file_is_shared() {
    // `dosbox.bat` matches another game; the folder name and suffix decide
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let known = |system: &str, name: &str| {
        Ident::Known(Game {
            system: system.into(),
            name: name.into(),
            crc: Some(1),
        })
    };
    let items = [
        item(
            file(&inbox, "Bloodstone.dos/dosbox.bat", "b"),
            known("DOS", "Battle Chess 4000 (1993)"),
            false,
        ),
        item(
            file(&inbox, "Sky.scummvm/SKY.EXE", "s"),
            known("DOS", "Beneath a Steel Sky (1994)"),
            false,
        ),
    ];
    let o = Options {
        inbox: Some(inbox.clone()),
        ignore: Vec::new(),
        ..opts(Mode::Move)
    };
    execute(&build(&items, &lib, &o).ops);
    assert_eq!(
        tree(&lib),
        ["DOS/Bloodstone/dosbox.bat", "ScummVM/Sky/SKY.EXE"]
    ); // auditing the library keeps the ScummVM folder although only DOS knows the file
    let lib_items = [item(
        lib.join("ScummVM/Sky/SKY.EXE"),
        known("DOS", "Beneath a Steel Sky (1994)"),
        true,
    )];
    let again = build(&lib_items, &lib, &opts(Mode::Move));
    assert!(again.ops.is_empty(), "{:?}", again.ops);
}

#[test]
fn incomplete_arcade_set_is_quarantined_with_reason() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let set = file(&inbox, "1943.zip", "z");
    let it = Item {
        files: Files::Set {
            archive: set,
            chds: Vec::new(),
            alt: Vec::new(),
            dat_note: String::new(),
        },
        ident: Ident::Incomplete("MAME: 1 missing (c.bin)".into()),
        in_library: false,
    };
    let plan = build(&[it], &lib, &opts(Mode::Move));
    assert_eq!(plan.quarantined, 1);
    assert_eq!(plan.why[0].rule, crate::rules::Rule::ArcadeDat);
    assert_eq!(plan.why[0].detail, "MAME: 1 missing (c.bin)");
    assert!(execute(&plan.ops).error.is_none());
    assert_eq!(tree(&lib), ["_quarantine/1943.zip"]);
}

#[test]
fn game_folder_repairs_broken_scummvm_launcher() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let launcher = file(&inbox, "Monkey 2.scummvm/monkey2.scummvm", "{\\rtf1\\ansi}");
    file(&inbox, "Monkey 2.scummvm/MONKEY2.000", "data");
    let items = [item(
        launcher,
        Ident::Known(Game {
            system: "ScummVM".into(),
            name: "Monkey Island 2: LeChuck's Revenge".into(),
            crc: Some(1),
        }),
        false,
    )];
    let o = Options {
        inbox: Some(inbox.clone()),
        ..opts(Mode::Move)
    };
    let plan = build(&items, &lib, &o);
    assert!(
        plan.why
            .iter()
            .any(|w| w.rule == crate::rules::Rule::ScummvmLauncher)
    );
    let ex = execute(&plan.ops);
    let fixed = lib.join("ScummVM/Monkey 2/monkey2.scummvm");
    assert_eq!(fs::read_to_string(&fixed).unwrap(), "monkey2");
    assert!(crate::plan::undo(&ex.done).is_empty());
    assert_eq!(
        fs::read_to_string(inbox.join("Monkey 2.scummvm/monkey2.scummvm")).unwrap(),
        "{\\rtf1\\ansi}"
    );
}

#[test]
fn msu1_folder_moves_whole_with_its_chip_dump() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let dir = "snes-msu1/Mega Man X2 (USA) (MSU1)";
    let rom = file(&inbox, &format!("{dir}/MMX2.sfc"), "rom");
    // the marker file is empty, so the scan yields no item for it
    file(&inbox, &format!("{dir}/MMX2.msu"), "");
    let pcm = file(&inbox, &format!("{dir}/MMX2-1.pcm"), "pcm");
    let chip = file(&inbox, &format!("{dir}/cx4.data.rom"), "cx4");
    let items = [
        item(rom, Ident::Unknown, false),
        item(pcm, Ident::Unknown, false),
        item(
            chip,
            Ident::Known(Game {
                system: "Nintendo - Super Nintendo Entertainment System".into(),
                name: "CX4 (World) (Enhancement Chip)".into(),
                crc: Some(7),
            }),
            false,
        ),
    ];
    let o = Options {
        inbox: Some(inbox.clone()),
        ..opts(Mode::Move)
    };
    let plan = build(&items, &lib, &o);
    assert!(plan.decisions.is_empty());
    execute(&plan.ops);
    let game = "Nintendo - Super Nintendo Entertainment System (MSU-1)/Mega Man X2 (USA) (MSU1)";
    let want: Vec<String> = ["MMX2-1.pcm", "MMX2.msu", "MMX2.sfc", "cx4.data.rom"]
        .iter()
        .map(|f| format!("{game}/{f}"))
        .collect();
    let got: Vec<String> = tree(&lib)
        .into_iter()
        .filter(|p| !p.starts_with("_playlists"))
        .collect();
    assert_eq!(got, want);
    // a library audit leaves the placed folder alone
    let lib_items: Vec<Item> = want
        .iter()
        .map(|p| item(lib.join(p), Ident::Unknown, true))
        .collect();
    let again = build(&lib_items, &lib, &opts(Mode::Move));
    assert!(again.ops.iter().all(|op| matches!(op, Op::Write { .. })));
}

#[test]
fn msu1_folder_without_marker_gets_one() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let rom = file(&inbox, "msu/Bubsy (MSU1)/Bubsy.sfc", "rom");
    file(&inbox, "msu/Bubsy (MSU1)/Bubsy-3.pcm", "pcm");
    let o = Options {
        inbox: Some(inbox.clone()),
        ..opts(Mode::Move)
    };
    execute(&build(&[item(rom, Ident::Unknown, false)], &lib, &o).ops);
    let got: Vec<String> = tree(&lib)
        .into_iter()
        .filter(|p| !p.starts_with("_playlists"))
        .collect();
    let game = "Nintendo - Super Nintendo Entertainment System (MSU-1)/Bubsy (MSU1)";
    assert_eq!(
        got,
        [
            format!("{game}/Bubsy-3.pcm"),
            format!("{game}/Bubsy.msu"),
            format!("{game}/Bubsy.sfc")
        ]
    );
}
