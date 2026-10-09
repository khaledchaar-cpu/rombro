use super::tests::{file, item, opts, tree};
use super::*;
use std::path::Path;
use tempfile::TempDir;

fn known(system: &str, name: &str) -> Ident {
    Ident::Known(Game {
        system: system.into(),
        name: name.into(),
        crc: Some(1),
    })
}

/// An arcade set item whose databases list it under its file name: `system`, then `alt`.
fn set(archive: &Path, system: &str, alt: &[&str]) -> Item {
    Item {
        files: Files::Set {
            archive: archive.to_path_buf(),
            chds: Vec::new(),
            alt: alt
                .iter()
                .map(|s| Game {
                    system: (*s).into(),
                    name: "x".into(),
                    crc: None,
                })
                .collect(),
            dat_note: String::new(),
        },
        ident: known(system, "x"),
        in_library: true,
    }
}

const GB: &str = "Nintendo - Game Boy";
const FBNEO: &str = "FBNeo - Arcade Games";

/// Bit-identical copies across systems and arcade cores: one stays (own system's folder,
/// best core), the rest is trashed; other content, game folders and `_bios` stay.
#[test]
fn bit_identical_library_copies_are_trashed() {
    let tmp = TempDir::new().unwrap();
    let lib = tmp.path().join("lib");
    let gb = file(&lib, &format!("{GB}/Tetris (World).gb"), "t");
    let gbc = file(&lib, "Nintendo - Game Boy Color/Tetris (World).gb", "t");
    let fb = file(&lib, &format!("{FBNEO}/kov.zip"), "k");
    let mame = file(&lib, "MAME/kov.zip", "k");
    let other = file(&lib, "MAME 2016/kov.zip", "k2");
    let dos1 = file(&lib, "DOS/A/dosbox.bat", "d");
    let dos2 = file(&lib, "DOS/B/dosbox.bat", "d");
    let items = [
        item(gb.clone(), known(GB, "Tetris (World)"), true),
        item(gbc.clone(), known(GB, "Tetris (World)"), true),
        set(&fb, FBNEO, &["MAME", "MAME 2016"]),
        set(&mame, FBNEO, &["MAME", "MAME 2016"]),
        set(&other, "MAME 2016", &[]),
        item(dos1.clone(), known("DOS", "A"), true),
        item(dos2.clone(), known("DOS", "B"), true),
    ];
    let mut o = opts(Mode::Move);
    o.rules.folder_systems.push("DOS".into());
    for (p, h) in [
        (&gb, 1),
        (&gbc, 1),
        (&fb, 2),
        (&mame, 2),
        (&other, 3),
        (&dos1, 4),
        (&dos2, 4),
    ] {
        o.hashes.insert(p.clone(), [h; 20]);
    }
    let plan = build(&items, &lib, &o);
    let trashed: Vec<_> = plan
        .ops
        .iter()
        .filter(|op| op.target().starts_with(lib.join(TRASH_DIR)))
        .filter_map(Op::source)
        .collect();
    assert_eq!(trashed, [gbc.as_path(), mame.as_path()], "{:?}", plan.ops);
    assert_eq!(plan.discarded, 2);
    execute(&plan.ops);
    let t = tree(&lib);
    assert!(t.contains(&format!("{FBNEO}/kov.zip")), "{t:?}");
    assert!(t.contains(&"MAME 2016/kov.zip".to_owned()), "{t:?}");
    assert!(t.contains(&format!("{GB}/Tetris (World).gb")), "{t:?}");
}

/// The same set under two names in one core folder: the name the core knows stays.
#[test]
fn identical_set_keeps_the_name_its_core_knows() {
    let tmp = TempDir::new().unwrap();
    let lib = tmp.path().join("lib");
    let fb_own = file(&lib, &format!("{FBNEO}/twsoc96.zip"), "s");
    let fb_mame_name = file(&lib, &format!("{FBNEO}/tws96.zip"), "s");
    let items = [
        set(&fb_own, FBNEO, &["MAME"]),
        set(&fb_mame_name, "MAME", &["MAME 2016"]),
    ];
    let mut o = opts(Mode::Move);
    o.hashes.insert(fb_own.clone(), [9; 20]);
    o.hashes.insert(fb_mame_name.clone(), [9; 20]);
    let plan = build(&items, &lib, &o);
    let trashed: Vec<_> = plan
        .ops
        .iter()
        .filter(|op| op.target().starts_with(lib.join(TRASH_DIR)))
        .filter_map(Op::source)
        .collect();
    assert_eq!(trashed, [fb_mame_name.as_path()], "{:?}", plan.ops);
}
