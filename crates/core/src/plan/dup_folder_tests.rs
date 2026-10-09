use super::tests::{file, item, opts, tree};
use super::*;
use tempfile::TempDir;

fn quake() -> Ident {
    Ident::Known(Game {
        system: "Quake".into(),
        name: "Quake (v1.06)".into(),
        crc: Some(1),
    })
}

/// Two folders of the same title in the library: the one with more files stays, the other
/// waits for a decision; "discard" trashes it whole, "keep" leaves both.
#[test]
fn duplicate_game_folder_is_a_decision() {
    let tmp = TempDir::new().unwrap();
    let lib = tmp.path().join("lib");
    let items = [
        item(file(&lib, "Quake/quake/id1/pak0.pak", "p"), quake(), true),
        item(
            file(&lib, "Quake/tyrquake/id1/pak0.pak", "p"),
            quake(),
            true,
        ),
        item(
            file(&lib, "Quake/tyrquake/id1/rogue/pak0.pak", "r"),
            Ident::Unknown,
            true,
        ),
    ];
    let plan = build(&items, &lib, &opts(Mode::Move));
    assert!(plan.ops.is_empty());
    assert!(matches!(&plan.decisions[..],
        [Decision::Rejected { path, kept: Some(k), reason, .. }]
            if path.ends_with("Quake/quake") && k == "Quake/tyrquake" && reason == "Duplicate"));

    let key = ("Quake".to_owned(), "Quake (v1.06)".to_owned());
    let mut o = opts(Mode::Move);
    o.verdicts.insert(key.clone(), Verdict::Keep);
    let keep = build(&items, &lib, &o);
    assert!(keep.ops.is_empty() && keep.decisions.is_empty());

    o.verdicts.insert(key, Verdict::Discard);
    let discard = build(&items, &lib, &o);
    assert_eq!(discard.discarded, 1);
    assert!(execute(&discard.ops).error.is_none());
    assert_eq!(
        tree(&lib),
        [
            "Quake/tyrquake/id1/pak0.pak",
            "Quake/tyrquake/id1/rogue/pak0.pak",
            "_trash/quake/id1/pak0.pak"
        ]
    );
}
