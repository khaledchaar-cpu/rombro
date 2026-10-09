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
    assert_eq!(console("Sony - PlayStation").unwrap().method, Method::Psx);
    assert!(console("Sony - PlayStation 3").is_none());
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

use crate::disc::iso9660::testimg;

fn cat(parts: &[&[u8]]) -> String {
    md5(&parts.concat())
}

#[test]
fn playstation_hashes_boot_name_and_executable() {
    let mut exe = b"PS-X EXE".to_vec();
    exe.resize(100, 7);
    let cnf = b"BOOT = cdrom:\\SLUS_005.94;1\r\nTCB = 4\r\n";
    let img = testimg::iso(&[("SYSTEM.CNF;1", cnf), ("SLUS_005.94;1", &exe)]);
    let tmp = TempDir::new().unwrap();
    // PS-X EXE: size from the header (0 here) + 2048, i.e. the whole first sector
    let mut sector = exe.clone();
    sector.resize(2048, 0);
    let want = cat(&[b"SLUS_005.94", &sector]);
    for (name, data) in [("g.iso", img.clone()), ("g.bin", testimg::raw(&img, 2))] {
        let p = tmp.path().join(name);
        std::fs::write(&p, data).unwrap();
        assert_eq!(hash_file(&p, Method::Psx).unwrap().unwrap(), want, "{name}");
    }
    // a cue sheet pointing at the raw track
    let cue = tmp.path().join("g.cue");
    std::fs::write(
        &cue,
        "FILE \"G.BIN\" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n",
    )
    .unwrap();
    assert_eq!(hash_file(&cue, Method::Psx).unwrap().unwrap(), want);

    // PS2: whole file by directory size, key BOOT2
    let cnf2 = b"BOOT2 = cdrom0:\\SLUS_200.62;1\nVER = 1.00\n";
    let img = testimg::iso(&[("SYSTEM.CNF;1", cnf2), ("SLUS_200.62;1", b"\x7fELFcode")]);
    let p = tmp.path().join("p2.iso");
    std::fs::write(&p, img).unwrap();
    assert_eq!(
        hash_file(&p, Method::Ps2).unwrap().unwrap(),
        cat(&[b"SLUS_200.62", b"\x7fELFcode"])
    );
    assert!(hash_file(&p, Method::Psx).unwrap().is_none());
}

#[test]
fn sega_cd_hashes_first_512_bytes() {
    let mut img = b"SEGA SEGASATURN ".to_vec();
    img.resize(4096, 3);
    let tmp = TempDir::new().unwrap();
    let p = tmp.path().join("s.bin");
    std::fs::write(&p, testimg::raw(&img, 1)).unwrap();
    assert_eq!(
        hash_file(&p, Method::SegaCd).unwrap().unwrap(),
        md5(&img[..512])
    );
    std::fs::write(&p, vec![0u8; 4096]).unwrap();
    assert!(hash_file(&p, Method::SegaCd).unwrap().is_none());
}

#[test]
fn psp_pbp_is_hashed_whole() {
    let tmp = TempDir::new().unwrap();
    let p = tmp.path().join("EBOOT.PBP");
    std::fs::write(&p, b"\0PBPdata").unwrap();
    assert_eq!(
        hash_file(&p, Method::Psp).unwrap().unwrap(),
        md5(b"\0PBPdata")
    );
}

#[test]
fn nds_hashes_header_code_and_icon() {
    let mut rom = vec![0u8; 0x3000];
    for (i, b) in rom.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }
    let put =
        |rom: &mut Vec<u8>, at: usize, v: u32| rom[at..at + 4].copy_from_slice(&v.to_le_bytes());
    put(&mut rom, 0x20, 0x1000);
    put(&mut rom, 0x2c, 0x100);
    put(&mut rom, 0x30, 0x1800);
    put(&mut rom, 0x3c, 0x80);
    put(&mut rom, 0x68, 0x2800);
    let mut icon = rom[0x2800..].to_vec();
    icon.resize(0xa00, 0);
    let want = cat(&[
        &rom[..0x160],
        &rom[0x1000..0x1100],
        &rom[0x1800..0x1880],
        &icon,
    ]);
    let tmp = TempDir::new().unwrap();
    let p = tmp.path().join("g.nds");
    std::fs::write(&p, &rom).unwrap();
    assert_eq!(hash_file(&p, Method::Nds).unwrap().unwrap(), want);
}

#[test]
fn library_lists_discs_not_tracks() {
    let tmp = TempDir::new().unwrap();
    let sys = tmp.path().join("Sega - Saturn");
    let multi = sys.join("Game");
    std::fs::create_dir_all(&multi).unwrap();
    std::fs::write(
        sys.join("A.cue"),
        "FILE \"A (Track 1).bin\" BINARY\n  TRACK 01 MODE1/2352\n",
    )
    .unwrap();
    for f in ["A (Track 1).bin", "B.bin", "C.chd"] {
        std::fs::write(sys.join(f), b"x").unwrap();
    }
    for f in ["Game (Disc 1).chd", "Game (Disc 2).chd", "Game.m3u"] {
        std::fs::write(multi.join(f), b"x").unwrap();
    }
    let names: Vec<String> = library_files(tmp.path())
        .unwrap()
        .iter()
        .map(|(p, _)| p.strip_prefix(&sys).unwrap().display().to_string())
        .collect();
    assert_eq!(
        names,
        [
            "A.cue",
            "B.bin",
            "C.chd",
            "Game/Game (Disc 1).chd",
            "Game/Game (Disc 2).chd"
        ]
    );
}
