//! Scanner integration tests with synthetic files and archives.

use rombro_core::header::Header;
use rombro_core::{hash_reader, scan};
use std::fs;
use std::io::Write;

fn rom(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed))
        .collect()
}

#[test]
fn scans_plain_zip_7z_and_headers() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let plain = rom(4096, 1);
    fs::write(root.join("a.gb"), &plain).unwrap();

    let body = rom(40960, 2);
    let mut nes = b"NES\x1a\x02\x01".to_vec();
    nes.resize(16, 0);
    nes.extend_from_slice(&body);
    fs::create_dir(root.join("sub")).unwrap();
    fs::write(root.join("sub/b.nes"), &nes).unwrap();

    let zipped = rom(2048, 3);
    let mut zw = zip::ZipWriter::new(fs::File::create(root.join("c.zip")).unwrap());
    zw.start_file("c.md", zip::write::SimpleFileOptions::default())
        .unwrap();
    zw.write_all(&zipped).unwrap();
    zw.add_directory("d/", zip::write::SimpleFileOptions::default())
        .unwrap();
    zw.finish().unwrap();

    let sz = rom(3000, 4);
    let src = root.join("src7");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("e.sms"), &sz).unwrap();
    sevenz_rust2::compress_to_path(&src, root.join("e.7z")).unwrap();
    fs::remove_dir_all(&src).unwrap();

    let report = scan(root);
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    assert_eq!(report.roms.len(), 4, "{:#?}", report.roms);

    let h = |d: &[u8]| hash_reader(d, None, false).unwrap().0;
    let find = |name: &str| {
        report
            .roms
            .iter()
            .find(|r| {
                r.member
                    .as_deref()
                    .unwrap_or(r.path.to_str().unwrap())
                    .ends_with(name)
            })
            .unwrap()
    };
    assert_eq!(find("a.gb").hashes, h(&plain));
    assert_eq!(find("c.md").hashes, h(&zipped));
    assert_eq!(find("e.sms").hashes, h(&sz));
    let n = find("b.nes");
    assert_eq!(n.header, Some(Header::Ines));
    assert_eq!(n.hashes, h(&nes));
    assert_eq!(n.headerless, Some(h(&body)));
}

#[test]
fn broken_archive_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("bad.zip"), b"not a zip").unwrap();
    let report = scan(dir.path());
    assert!(report.roms.is_empty());
    assert_eq!(report.failures.len(), 1);
}

#[test]
fn scans_discs_without_loose_tracks() {
    use rombro_core::disc::iso9660::testimg;
    use rombro_core::disc::{DiscKind, Platform};
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // PS1: raw mode-2 data track + audio track, referenced with different case.
    let ps1 = testimg::iso(&[("SYSTEM.CNF;1", b"BOOT = cdrom:\\SLUS_005.94;1\r\n")]);
    fs::write(root.join("Game (Track 1).bin"), testimg::raw(&ps1, 2)).unwrap();
    fs::write(root.join("Game (Track 2).bin"), rom(2352 * 4, 9)).unwrap();
    fs::write(
        root.join("Game.cue"),
        "FILE \"game (track 1).bin\" BINARY\n  TRACK 01 MODE2/2352\n\
         FILE \"Game (Track 2).bin\" BINARY\n  TRACK 02 AUDIO\nFILE \"gone.bin\" BINARY\n",
    )
    .unwrap();
    // PS2 cooked ISO.
    let ps2 = testimg::iso(&[("SYSTEM.CNF;1", b"BOOT2 = cdrom0:\\SLES_509.33;1\n")]);
    fs::write(root.join("ps2.iso"), &ps2).unwrap();
    fs::write(root.join("Set.m3u"), "Game.cue\n").unwrap();
    fs::write(root.join("loose.gb"), rom(1024, 4)).unwrap();

    let r = scan(root);
    assert!(r.failures.is_empty(), "{:?}", r.failures);
    assert_eq!(r.roms.len(), 1, "only loose.gb is a plain rom");
    assert_eq!(r.discs.len(), 2);
    let cue = &r.discs[0];
    assert_eq!(cue.kind, DiscKind::Cue);
    assert_eq!(cue.tracks.len(), 2);
    assert_eq!(cue.missing, vec![root.join("gone.bin")]);
    let id = cue.id.as_ref().unwrap();
    assert_eq!(
        (id.platform, id.serial.as_str()),
        (Platform::Ps1, "SLUS-00594")
    );
    let iso = &r.discs[1];
    assert_eq!(iso.id.as_ref().unwrap().serial, "SLES-50933");
    assert_eq!(
        iso.tracks[0].hashes,
        hash_reader(&ps2[..], None, false).unwrap().0
    );
    assert_eq!(r.playlists[0].entries, vec![root.join("Game.cue")]);
}

#[test]
fn reports_progress_per_file() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..3u8 {
        fs::write(dir.path().join(format!("{i}.bin")), rom(64, i)).unwrap();
    }
    let calls = std::sync::Mutex::new(Vec::new());
    let report = rombro_core::scan_with_progress(dir.path(), &|p| {
        assert_eq!(p.bytes_total, 3 * 64);
        calls.lock().unwrap().push((p.done, p.total));
    });
    assert_eq!(report.roms.len(), 3);
    let mut calls = calls.into_inner().unwrap();
    calls.sort_unstable();
    assert_eq!(calls, [(0, 3), (1, 3), (2, 3), (3, 3)]);
}

#[test]
fn cached_scan_reuses_unchanged_files_only() {
    use rombro_core::{CachedRom, HashCache, Stamp, scan_cached};
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a.gb"), dir.path().join("b.gb"));
    fs::write(&a, rom(1024, 1)).unwrap();
    fs::write(&b, rom(1024, 2)).unwrap();
    let mut cache = HashCache::default();
    for r in scan(dir.path()).roms {
        let mut c = CachedRom::from_rom(&r);
        c.hashes.crc = 0xdead_beef; // marker: proves the cached value is used
        cache
            .0
            .insert(r.path.clone(), (Stamp::of(&r.path).unwrap(), vec![c]));
    }
    fs::write(&b, rom(2048, 3)).unwrap(); // size changes → rehash
    let report = scan_cached(dir.path(), &cache, &|_| {});
    assert_eq!(report.roms[0].hashes.crc, 0xdead_beef);
    assert_ne!(report.roms[1].hashes.crc, 0xdead_beef);
    assert_eq!(report.roms[1].hashes.size, 2048);
}

#[test]
fn retroarch_playlists_are_not_roms() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("Nintendo - Game Boy.lpl"), b"{}").unwrap();
    let report = scan(dir.path());
    assert!(report.roms.is_empty() && report.failures.is_empty());
}

#[test]
fn hashes_archives_as_a_whole() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("set.zip");
    let mut w = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    for name in ["a.rom", "b.rom"] {
        w.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(&rom(32, 1)).unwrap();
    }
    w.finish().unwrap();
    let report = scan(dir.path());
    assert_eq!(report.roms.len(), 2);
    assert_eq!(report.archives.len(), 1);
    let (whole, _) = hash_reader(fs::File::open(&path).unwrap(), None, false).unwrap();
    assert_eq!(report.archives[0].hashes, whole);
    assert_eq!(report.archives[0].member, None);
}

#[test]
fn ignores_os_clutter_and_empty_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("game.bin"), rom(64, 1)).unwrap();
    fs::write(root.join(".DS_Store"), b"junk").unwrap();
    fs::write(root.join("._game.bin"), b"junk").unwrap();
    fs::write(root.join("Thumbs.db"), b"junk").unwrap();
    fs::create_dir(root.join("hi")).unwrap();
    fs::write(root.join("hi/.keep"), b"").unwrap();
    let report = scan(root);
    let paths: Vec<_> = report.roms.iter().map(|r| r.path.clone()).collect();
    assert_eq!(paths, vec![root.join("game.bin")]);
    assert!(report.failures.is_empty());
}

#[test]
fn reports_bytes_while_hashing_a_disc() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("big.iso"), vec![0u8; 4 << 20]).unwrap();
    let calls = std::sync::Mutex::new(Vec::new());
    rombro_core::scan_with_progress(dir.path(), &|p| {
        let item = p.item.map(str::to_owned);
        calls.lock().unwrap().push((p.done, p.bytes, item));
    });
    let calls = calls.into_inner().unwrap();
    assert!(
        calls
            .iter()
            .any(|(d, b, i)| *d == 0 && *b > 0 && i.as_deref() == Some("big.iso"))
    );
    assert_eq!(calls.last().unwrap(), &(1, 4 << 20, None));
}
