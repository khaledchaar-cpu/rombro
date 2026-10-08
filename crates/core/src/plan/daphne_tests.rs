use super::tests::{file, item, opts, tree};
use super::*;
use tempfile::TempDir;

#[test]
fn daphne_collection_moves_whole_and_lists_games_with_video() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let paths = [
        "daphne/lair.daphne/lair.txt",
        "daphne/lair.daphne/lair.m2v",
        "daphne/roms/lair.zip",
        "daphne/roms/lair_x.zip",
    ];
    let items: Vec<Item> = paths
        .iter()
        .map(|p| item(file(&inbox, p, p), Ident::Unknown, false))
        .collect();
    let o = Options {
        inbox: Some(inbox.clone()),
        ..opts(Mode::Move)
    };
    let plan = build(&items, &lib, &o);
    assert!(plan.decisions.is_empty(), "{:?}", plan.decisions);
    execute(&plan.ops);
    let got: Vec<String> = tree(&lib)
        .into_iter()
        .filter(|p| !p.starts_with("_playlists"))
        .collect();
    let mut want: Vec<String> = paths
        .iter()
        .map(|p| p.replacen("daphne/", "Daphne/", 1))
        .collect();
    want.sort();
    assert_eq!(got, want);
    // a library audit leaves the collection alone
    let lib_items: Vec<Item> = want
        .iter()
        .map(|p| item(lib.join(p), Ident::Unknown, true))
        .collect();
    let again = build(&lib_items, &lib, &opts(Mode::Move));
    assert!(again.ops.iter().all(|op| matches!(op, Op::Write { .. })));
}
