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

    let h = |d: &[u8]| hash_reader(d, None).unwrap().0;
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
