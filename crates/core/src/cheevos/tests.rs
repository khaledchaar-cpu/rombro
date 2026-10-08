use super::*;
use std::io::Write;
use tempfile::TempDir;

fn md5(data: &[u8]) -> String {
    hex(&Md5::digest(data))
}

#[test]
fn table_maps_systems() {
    let nes = console("Nintendo - Nintendo Entertainment System").unwrap();
    assert_eq!((nes.id, nes.method), (7, Method::Nes));
    assert_eq!(console("MAME 2003-Plus").unwrap().method, Method::Arcade);
    assert!(console("Sony - PlayStation").is_none());
    let ids = console_ids();
    assert!(ids.contains(&27) && ids.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn strips_headers() {
    let rom: Vec<u8> = (0..0x4000u32).map(|i| i as u8).collect();
    let mut nes = b"NES\x1a".to_vec();
    nes.resize(16, 0);
    nes.extend_from_slice(&rom);
    assert_eq!(hash_bytes(&nes, Method::Nes).unwrap(), md5(&rom));
    assert_eq!(hash_bytes(&rom, Method::Nes).unwrap(), md5(&rom));

    let mut snes = vec![0u8; 512];
    snes.extend_from_slice(&rom);
    assert_eq!(hash_bytes(&snes, Method::Snes).unwrap(), md5(&rom));
    assert_eq!(hash_bytes(&snes, Method::Pce).unwrap(), md5(&rom));

    let mut a78 = vec![1u8];
    a78.extend_from_slice(b"ATARI7800");
    a78.resize(128, 0);
    a78.extend_from_slice(&rom);
    assert_eq!(hash_bytes(&a78, Method::A7800).unwrap(), md5(&rom));

    let mut lynx = b"LYNX\0".to_vec();
    lynx.resize(64, 0);
    lynx.extend_from_slice(&rom);
    assert_eq!(hash_bytes(&lynx, Method::Lynx).unwrap(), md5(&rom));
}

#[test]
fn n64_byte_orders_match() {
    let z64 = [0x80, 0x37, 0x12, 0x40, 1, 2, 3, 4];
    let v64 = [0x37, 0x80, 0x40, 0x12, 2, 1, 4, 3];
    let n64 = [0x40, 0x12, 0x37, 0x80, 4, 3, 2, 1];
    let want = md5(&z64);
    assert_eq!(hash_bytes(&z64, Method::N64).unwrap(), want);
    assert_eq!(hash_bytes(&v64, Method::N64).unwrap(), want);
    assert_eq!(hash_bytes(&n64, Method::N64).unwrap(), want);
    assert!(hash_bytes(&[0u8; 8], Method::N64).is_none());
}

#[test]
fn files_archives_and_arcade() {
    let tmp = TempDir::new().unwrap();
    let zip_path = tmp.path().join("Game (USA).zip");
    let mut w = zip::ZipWriter::new(File::create(&zip_path).unwrap());
    w.start_file("Game (USA).gb", zip::write::SimpleFileOptions::default())
        .unwrap();
    w.write_all(b"gameboy").unwrap();
    w.finish().unwrap();
    assert_eq!(
        hash_file(&zip_path, Method::Whole).unwrap().unwrap(),
        md5(b"gameboy")
    );
    let set = tmp.path().join("1943.zip");
    std::fs::copy(&zip_path, &set).unwrap();
    assert_eq!(
        hash_file(&set, Method::Arcade).unwrap().unwrap(),
        md5(b"1943")
    );
}

#[test]
fn title_keys_match_across_versions() {
    assert_eq!(
        title_key("Legend of Zelda, The - A Link to the Past (Europe)"),
        title_key("Legend of Zelda, The: A Link to the Past")
    );
    assert_eq!(
        title_key("Mario Kart 64 [Subset - Shortcuts]"),
        "mariokart64"
    );
}
