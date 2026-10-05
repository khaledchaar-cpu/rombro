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
fn extracts_members_and_trashes_emptied_archive() {
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
        [
            "Nintendo - SNES/Mario (Europe).sfc",
            "_quarantine/x.bin",
            "_trash/set.zip"
        ]
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
