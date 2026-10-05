use super::*;
use std::fs::{self, File};
use std::io::Write;
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
        rules: Default::default(),
        playlists: None,
        verdicts: Default::default(),
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
fn extracts_known_members_and_quarantines_rest() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let set = inbox.join("set.zip");
    zip(&set, &["mario.sfc", "x.bin"]);
    let items = [
        member(&set, "mario.sfc", game("Mario (Europe)")),
        member(&set, "x.bin", Ident::Unknown),
    ];
    let ex = execute(&build(&items, &lib, &opts(Mode::Move)).ops);
    assert!(ex.error.is_none());
    assert_eq!(
        tree(&lib),
        ["Nintendo - SNES/Mario (Europe).sfc", "_quarantine/set.zip"]
    );
    assert_eq!(
        fs::read(lib.join("Nintendo - SNES/Mario (Europe).sfc")).unwrap(),
        b"mario.sfc"
    );
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
