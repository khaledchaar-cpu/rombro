//! CHD CD images: the data track is hashed and its serial read like a loose `.bin`.

use rombro_core::disc::iso9660::testimg;
use rombro_core::disc::{DiscKind, Platform};
use rombro_core::hash::hash_reader;
use rombro_core::scan::scan;
use std::fs;

const FRAME: usize = 2448;
const HUNK_FRAMES: usize = 8;

/// Builds an uncompressed CHD v5 CD image from (type, raw sector data) tracks.
fn chd(tracks: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let hunk_bytes = FRAME * HUNK_FRAMES;
    let mut frames = Vec::new();
    let mut meta = Vec::new();
    for (i, (kind, data)) in tracks.iter().enumerate() {
        let n = data.len() / 2352;
        for s in data.chunks(2352) {
            let mut f = s.to_vec();
            f.resize(FRAME, 0);
            frames.extend_from_slice(&f);
        }
        frames.resize(frames.len() + (n.div_ceil(4) * 4 - n) * FRAME, 0);
        let text = format!(
            "TRACK:{} TYPE:{kind} SUBTYPE:NONE FRAMES:{n} PREGAP:0 PGTYPE:MODE1 PGSUB:NONE POSTGAP:0\0",
            i + 1
        );
        meta.push(text.into_bytes());
    }
    let logical = frames.len();
    frames.resize(logical.div_ceil(hunk_bytes) * hunk_bytes, 0);
    let hunks = frames.len() / hunk_bytes;

    let mut out = vec![0u8; hunk_bytes]; // header, map and metadata live in the first hunk
    let be32 = |v: usize| (v as u32).to_be_bytes();
    out[0..8].copy_from_slice(b"MComprHD");
    out[8..12].copy_from_slice(&be32(124));
    out[12..16].copy_from_slice(&be32(5));
    out[32..40].copy_from_slice(&(logical as u64).to_be_bytes());
    let map_at = 124;
    out[40..48].copy_from_slice(&(map_at as u64).to_be_bytes());
    out[56..60].copy_from_slice(&be32(hunk_bytes));
    out[60..64].copy_from_slice(&be32(FRAME));
    for h in 0..hunks {
        out[map_at + h * 4..map_at + h * 4 + 4].copy_from_slice(&be32(h + 1));
    }
    let mut at = map_at + hunks * 4;
    out[48..56].copy_from_slice(&(at as u64).to_be_bytes());
    for (i, m) in meta.iter().enumerate() {
        let next = if i + 1 < meta.len() {
            at + 16 + m.len()
        } else {
            0
        };
        out[at..at + 4].copy_from_slice(b"CHT2");
        out[at + 4..at + 8].copy_from_slice(&be32(0x0100_0000 | m.len()));
        out[at + 8..at + 16].copy_from_slice(&(next as u64).to_be_bytes());
        out[at + 16..at + 16 + m.len()].copy_from_slice(m);
        at += 16 + m.len();
    }
    out.extend_from_slice(&frames);
    out
}

#[test]
fn scans_chd_data_track_like_a_bin() {
    let dir = tempfile::tempdir().unwrap();
    let ps1 = testimg::iso(&[("SYSTEM.CNF;1", b"BOOT = cdrom:\\SLUS_005.94;1\r\n")]);
    let bin = testimg::raw(&ps1, 2);
    let img = chd(&[("MODE2_RAW", bin.clone()), ("AUDIO", vec![7u8; 2352 * 5])]);
    fs::write(dir.path().join("Game.chd"), img).unwrap();

    let r = scan(dir.path());
    assert!(r.failures.is_empty(), "{:?}", r.failures);
    assert!(r.roms.is_empty());
    let d = &r.discs[0];
    assert_eq!(d.kind, DiscKind::Chd);
    assert_eq!(
        d.tracks[0].hashes,
        hash_reader(&bin[..], None, false).unwrap().0
    );
    let id = d.id.as_ref().unwrap();
    assert_eq!(
        (id.platform, id.serial.as_str()),
        (Platform::Ps1, "SLUS-00594")
    );
}
