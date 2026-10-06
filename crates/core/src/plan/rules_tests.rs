//! Configurable rules (M13): quarantine toggle, per-system overrides, arcade order.

use super::*;
use crate::g1r::SystemRules;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const SYS: &str = "Nintendo - SNES";

fn known(dir: &Path, file: &str, name: &str) -> Item {
    let p = dir.join(file);
    fs::create_dir_all(dir).unwrap();
    fs::write(&p, file).unwrap();
    Item {
        files: Files::Single(p),
        ident: Ident::Known(Game {
            system: SYS.into(),
            name: name.into(),
            crc: Some(1),
        }),
        in_library: false,
    }
}

fn opts(rules: Rules) -> Options {
    Options {
        mode: Mode::Copy,
        rules,
        playlists: None,
        verdicts: Default::default(),
        inbox: None,
        ignore: Vec::new(),
    }
}

#[test]
fn quarantine_off_leaves_unknown_files() {
    let tmp = TempDir::new().unwrap();
    let inbox = tmp.path().join("inbox");
    let mut unknown = known(&inbox, "x.bin", "x");
    unknown.ident = Ident::Unknown;
    let items = [known(&inbox, "a.sfc", "Mario (Europe)"), unknown];
    let lib = tmp.path().join("lib");
    let on = build(&items, &lib, &opts(Rules::default()));
    assert_eq!(on.quarantined, 1);
    let rules = Rules {
        quarantine: false,
        ..Rules::default()
    };
    let off = build(&items, &lib, &opts(rules));
    assert_eq!((off.placed, off.quarantined), (1, 0));
}

#[test]
fn per_system_regions_override_global() {
    let tmp = TempDir::new().unwrap();
    let inbox = tmp.path().join("inbox");
    let items = [
        known(&inbox, "e.sfc", "Mario (Europe)"),
        known(&inbox, "j.sfc", "Mario (Japan)"),
    ];
    let lib = tmp.path().join("lib");
    let pick = |rules: Rules| match &build(&items, &lib, &opts(rules)).decisions[..] {
        [Decision::Rejected { kept: Some(k), .. }] => k.clone(),
        d => panic!("{d:?}"),
    };
    assert_eq!(pick(Rules::default()), "Mario (Europe)");
    let mut rules = Rules::default();
    rules.systems.insert(
        SYS.into(),
        SystemRules {
            regions: Some(vec!["Japan".into()]),
            ..Default::default()
        },
    );
    assert_eq!(pick(rules), "Mario (Japan)");
}

#[test]
fn arcade_rank_follows_user_order() {
    use crate::arcade::rank_in;
    let order = vec!["MAME".to_owned(), "FBNeo - Arcade Games".to_owned()];
    assert!(rank_in(&order, "MAME") < rank_in(&order, "FBNeo - Arcade Games"));
    // unlisted arcade DBs after listed ones, non-arcade last
    assert!(rank_in(&order, "HBMAME") > rank_in(&order, "FBNeo - Arcade Games"));
    assert!(rank_in(&order, "HBMAME") < rank_in(&order, "Sony - PlayStation"));
}

#[test]
fn ignored_paths_are_never_planned() {
    let tmp = TempDir::new().unwrap();
    let inbox = tmp.path().join("inbox");
    let mut unknown = known(&inbox.join("keep"), "x.bin", "x");
    unknown.ident = Ident::Unknown;
    let items = [
        known(&inbox.join("keep"), "a.sfc", "Mario (Europe)"),
        unknown,
    ];
    let mut o = opts(Rules::default());
    o.ignore = vec![inbox.join("keep")];
    let p = build(&items, &tmp.path().join("lib"), &o);
    assert!(p.ops.is_empty() && p.decisions.is_empty(), "{p:?}");
}

#[test]
fn identical_copies_are_duplicates_different_ones_conflict() {
    let tmp = TempDir::new().unwrap();
    let lib = tmp.path().join("lib");
    let item = known(&tmp.path().join("inbox"), "m.sfc", "Mario (Europe)");
    // an unindexed file already sits at the target
    let target = lib.join(SYS).join("Mario (Europe).sfc");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    let plan = |content: &str| {
        fs::write(&target, content).unwrap();
        build(std::slice::from_ref(&item), &lib, &opts(Rules::default())).decisions
    };
    let same = plan("m.sfc"); // same bytes as the inbox file
    assert!(
        matches!(&same[..], [Decision::Rejected { reason, kept: Some(k), .. }] if reason == "Duplicate" && k == "Mario (Europe)"),
        "{same:?}"
    );
    let other = plan("different");
    assert!(
        matches!(&other[..], [Decision::Conflict { .. }]),
        "{other:?}"
    );
}

#[test]
fn frontend_metadata_goes_to_trash_and_game_folder_moves_without_it() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let put = |rel: &str| {
        let p = inbox.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, rel).unwrap();
        p
    };
    let unknown = |rel: &str| Item {
        files: Files::Single(put(rel)),
        ident: Ident::Unknown,
        in_library: false,
    };
    let items = [
        Item {
            files: Files::Single(put("tyrquake/pak0.pak")),
            ident: Ident::Known(Game {
                system: "Quake".into(),
                name: "Quake".into(),
                crc: Some(1),
            }),
            in_library: false,
        },
        unknown("tyrquake/id1/pak1.pak"),
        unknown("tyrquake/gamelist.xml"),
        unknown("tyrquake/gamelist.xml.old"),
        unknown("tyrquake/images/Quake-image.png"),
        // a frontend folder without any game
        unknown("mrboom/gamelist.xml"),
        unknown("mrboom/videos/MrBoom-video.mp4"),
        unknown("mrboom/MrBoom.libretro"),
    ];
    let mut o = Options {
        mode: Mode::Move,
        inbox: Some(inbox.clone()),
        ..opts(Rules::default())
    };
    let plan = build(&items, &lib, &o);
    assert_eq!(plan.discarded, 5);
    let r = crate::plan::execute(&plan.ops);
    assert!(r.error.is_none(), "{:?}", r.error);
    let left: Vec<_> = walkdir::WalkDir::new(&lib)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().strip_prefix(&lib).unwrap().display().to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(
        left,
        [
            "Quake/tyrquake/id1/pak1.pak",
            "Quake/tyrquake/pak0.pak",
            "_trash/frontend/mrboom/gamelist.xml",
            "_trash/frontend/mrboom/videos/MrBoom-video.mp4",
            "_trash/frontend/tyrquake/gamelist.xml",
            "_trash/frontend/tyrquake/gamelist.xml.old",
            "_trash/frontend/tyrquake/images/Quake-image.png",
        ]
    );
    assert!(inbox.join("mrboom/MrBoom.libretro").is_file());

    // switched off: nothing goes to the trash
    o.rules.frontend_trash = false;
    let tmp2 = TempDir::new().unwrap();
    let off = build(&items[4..5], &tmp2.path().join("lib"), &o);
    assert_eq!(off.discarded, 0);
}

#[test]
fn unknown_files_go_to_trash_and_old_quarantine_is_emptied() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let put = |p: PathBuf| {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, p.to_string_lossy().as_bytes()).unwrap();
        p
    };
    let it = |p: PathBuf, ident: Ident, in_library: bool| Item {
        files: Files::Single(put(p)),
        ident,
        in_library,
    };
    let items = [
        it(lib.join("_quarantine/ngp/old.zip"), Ident::Unknown, true),
        // identified since it was quarantined: stays
        it(
            lib.join("_quarantine/ngp/now known.zip"),
            Ident::Known(Game {
                system: SYS.into(),
                name: "Now Known".into(),
                crc: Some(2),
            }),
            true,
        ),
        known(&inbox, "Mario (Europe).sfc", "Mario (Europe)"),
        it(inbox.join("bad dump.sfc"), Ident::Unknown, false),
    ];
    let o = Options {
        mode: Mode::Move,
        inbox: Some(inbox.clone()),
        ..opts(Rules::default())
    };
    let plan = build(&items, &lib, &o);
    assert_eq!(plan.quarantined, 2);
    assert!(crate::plan::execute(&plan.ops).error.is_none());
    assert!(lib.join("_trash/unknown/ngp/old.zip").is_file());
    assert!(lib.join("_trash/unknown/bad dump.sfc").is_file());
    assert!(lib.join("_quarantine/ngp/now known.zip").is_file());
}

#[test]
fn playlist_of_a_system_without_games_goes_to_the_trash() {
    let tmp = TempDir::new().unwrap();
    let lib = tmp.path().join("lib");
    let pl = lib.join("_playlists");
    fs::create_dir_all(&pl).unwrap();
    fs::write(pl.join("MAME.lpl"), "{}").unwrap();
    let mut item = known(&lib.join(SYS), "Mario (Europe).sfc", "Mario (Europe)");
    item.in_library = true;
    let o = Options {
        playlists: Some(pl.clone()),
        ..opts(Rules::default())
    };
    let plan = build(&[item], &lib, &o);
    assert!(plan.ops.iter().any(|op| matches!(op,
        Op::Move { from, to } if from == &pl.join("MAME.lpl")
            && to == &lib.join("_trash/playlists/MAME.lpl"))));
    assert!(plan.ops.iter().any(|op| matches!(op,
        Op::Write { path, .. } if path == &pl.join(format!("{SYS}.lpl")))));
}

#[test]
fn name_folder_places_unknown_files_by_name_with_1g1r() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox/n64dd"), tmp.path().join("lib"));
    let unknown = |file: &str| {
        let mut it = known(&inbox, file, "");
        it.ident = Ident::Unknown;
        it
    };
    let items = [
        unknown("F-Zero X + Expansion Kit (Japan).n64"),
        unknown("F-Zero X + Expansion Kit (USA).n64"),
        unknown("gamelist.Missing.Serial.txt"),
    ];
    let mut o = opts(Rules::default());
    o.inbox = Some(tmp.path().join("inbox"));
    let plan = build(&items, &lib, &o);
    let n64 = "Nintendo - Nintendo 64";
    let placed: Vec<_> = plan
        .ops
        .iter()
        .map(|op| op.target().to_path_buf())
        .collect();
    assert!(placed.contains(&lib.join(n64).join("F-Zero X + Expansion Kit (USA).n64")));
    assert!(
        plan.why
            .iter()
            .any(|w| w.rule == crate::rules::Rule::NameOnly)
    );
    // the Japanese release loses 1G1R: a decision, never trashed by name alone
    assert!(
        plan.decisions
            .iter()
            .any(|d| matches!(d, Decision::Rejected { name, .. }
        if name == "F-Zero X + Expansion Kit (Japan)"))
    );
    assert!(
        !placed
            .iter()
            .any(|p| p.to_string_lossy().contains("(Japan)"))
    );

    // audit: the placed file in the system folder stays identified (not trashed as unknown)
    let lib_item = Item {
        files: Files::Single(lib.join(n64).join("F-Zero X + Expansion Kit (USA).n64")),
        ident: Ident::Unknown,
        in_library: true,
    };
    let named = name_only(&[lib_item], &Rules::default());
    assert!(matches!(&named[0].ident, Ident::Named(g) if g.system == n64));
}
