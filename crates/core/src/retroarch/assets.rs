//! System files some cores cannot start without (blueMSX machine definitions, PPSSPP's UI
//! atlas, Dolphin's `Sys` folder …): RetroArch's buildbot serves them as zips under
//! `assets/system/`, unpacked into the system folder as they are.

use crate::plan::Op;
use std::path::{Path, PathBuf};

/// Core id → (asset zip on the buildbot, path inside the system folder that shows it is there).
/// Only files the core cannot start without; optional extras (themes, cheats, hiscores) and
/// cores that run without their zip (ScummVM, NXEngine, PrBoom) are left out.
const ASSETS: [(&str, &str, &str); 5] = [
    ("bluemsx", "blueMSX.zip", "Machines/COL - ColecoVision"),
    ("ppsspp", "PPSSPP.zip", "PPSSPP/ppge_atlas.zim"),
    ("dolphin", "Dolphin.zip", "dolphin-emu/Sys"),
    ("pcsx2", "LRPS2.zip", "pcsx2/resources"),
    ("ecwolf", "ECWolf.zip", "ecwolf.pk3"),
];

/// The asset zip `core` needs, if its marker is missing from `system`.
pub fn missing(core: &str, system: &Path) -> Option<&'static str> {
    ASSETS
        .iter()
        .find(|(id, _, marker)| *id == core && !system.join(marker).exists())
        .map(|(_, zip, _)| *zip)
}

/// Download URL of an asset zip: next to the core folder on the same buildbot host
/// (`https://buildbot.libretro.com/nightly/linux/x86_64/latest` → `…/assets/system/<zip>`).
pub fn url(buildbot: &str, zip: &str) -> String {
    let host_end = buildbot
        .find("://")
        .and_then(|i| buildbot[i + 3..].find('/').map(|j| i + 3 + j))
        .unwrap_or(buildbot.len());
    let name = zip
        .replace(' ', "%20")
        .replace('(', "%28")
        .replace(')', "%29");
    format!("{}/assets/system/{name}", &buildbot[..host_end])
}

/// One extraction per file of a downloaded asset zip that `system` still lacks.
pub fn unpack_ops(zip: &Path, system: &Path) -> std::io::Result<Vec<Op>> {
    Ok(crate::archive::file_paths(zip)?
        .into_iter()
        .filter_map(|m| {
            let to: PathBuf = system.join(&m);
            (!to.exists()).then(|| Op::Extract {
                archive: zip.to_path_buf(),
                member: m,
                to,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_sits_on_the_buildbot_host() {
        let bb = "https://buildbot.libretro.com/nightly/linux/x86_64/latest";
        assert_eq!(
            url(bb, "NXEngine (Cave Story).zip"),
            "https://buildbot.libretro.com/assets/system/NXEngine%20%28Cave%20Story%29.zip"
        );
    }

    #[test]
    fn missing_only_without_marker() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert_eq!(missing("bluemsx", tmp.path()), Some("blueMSX.zip"));
        assert_eq!(missing("snes9x", tmp.path()), None);
        std::fs::create_dir_all(tmp.path().join("Machines/COL - ColecoVision")).unwrap();
        assert_eq!(missing("bluemsx", tmp.path()), None);
    }

    #[test]
    fn unpacks_only_missing_files() {
        use std::io::Write;
        let tmp = tempfile::TempDir::new().unwrap();
        let (zip, system) = (tmp.path().join("a.zip"), tmp.path().join("system"));
        let mut w = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        w.add_directory("Machines/", o).unwrap();
        for f in ["Machines/a.ini", "Machines/b.ini"] {
            w.start_file(f, o).unwrap();
            w.write_all(b"x").unwrap();
        }
        w.finish().unwrap();
        std::fs::create_dir_all(system.join("Machines")).unwrap();
        std::fs::write(system.join("Machines/a.ini"), b"mine").unwrap();
        let ops = unpack_ops(&zip, &system).unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].target(), system.join("Machines/b.ini"));
    }
}
