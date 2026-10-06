//! Configurable rules (M13): quarantine toggle, per-system overrides, arcade order.

use super::*;
use crate::g1r::SystemRules;
use std::fs;
use std::path::Path;
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
