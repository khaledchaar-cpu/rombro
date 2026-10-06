//! Export to RetroArch: the library playlists (with the matching installed core as default)
//! into RetroArch's playlist folder, and identified firmware into its system folder.
//! Only plans operations; executing them goes through the journal like any import.

use super::Dirs;
use super::firmware::Firmware;
use super::info::{Core, core_for};
use crate::plan::{BIOS_DIR, Op, PLAYLIST_DIR};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// Planned operations plus what the user should know about them.
#[derive(Debug, Default)]
pub struct Export {
    pub ops: Vec<Op>,
    /// Playlists written (system, core name if one is installed).
    pub playlists: Vec<(String, Option<String>)>,
    /// Playlists already up to date.
    pub playlists_unchanged: usize,
    /// Firmware copied into the system folder (path inside it).
    pub bios_copied: Vec<String>,
    /// Firmware already in place with the right content.
    pub bios_present: usize,
    /// Firmware in the system folder with other content: left alone.
    pub bios_conflicts: Vec<String>,
    /// Firmware of systems in the library that the library lacks (system, path).
    pub bios_missing: Vec<(String, String)>,
}

/// Plans the export. `library_files` maps whole-file SHA1 to a library path (from the index).
pub fn plan(
    library: &Path,
    dirs: &Dirs,
    cores: &[Core],
    firmware: &[Firmware],
    library_files: &HashMap<[u8; 20], PathBuf>,
    playlists: bool,
    bios: bool,
) -> Export {
    let mut ex = Export::default();
    let systems = library_systems(library);
    if playlists {
        plan_playlists(library, dirs, cores, &mut ex);
    }
    if bios {
        plan_bios(library, dirs, firmware, library_files, &systems, &mut ex);
    }
    ex
}

fn plan_playlists(library: &Path, dirs: &Dirs, cores: &[Core], ex: &mut Export) {
    let Ok(rd) = std::fs::read_dir(library.join(PLAYLIST_DIR)) else {
        return;
    };
    let mut files: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "lpl"))
        .collect();
    files.sort();
    for src in files {
        let Some(name) = src.file_name() else {
            continue;
        };
        let system = src
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Ok(text) = std::fs::read_to_string(&src) else {
            continue;
        };
        let Ok(mut doc) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let core = core_for(cores, &system);
        if let Some(c) = core {
            doc["default_core_path"] = c.path.to_string_lossy().into_owned().into();
            doc["default_core_name"] = c.name.clone().into();
        }
        let mut contents = serde_json::to_string_pretty(&doc).unwrap_or_default();
        contents.push('\n');
        let path = dirs.playlists.join(name);
        if std::fs::read_to_string(&path).is_ok_and(|old| old == contents) {
            ex.playlists_unchanged += 1;
            continue;
        }
        ex.ops.push(Op::Write { path, contents });
        ex.playlists.push((system, core.map(|c| c.name.clone())));
    }
}

fn plan_bios(
    library: &Path,
    dirs: &Dirs,
    firmware: &[Firmware],
    library_files: &HashMap<[u8; 20], PathBuf>,
    systems: &BTreeSet<String>,
    ex: &mut Export,
) {
    let mut planned = BTreeSet::new();
    for f in firmware {
        if !planned.insert(f.path.as_str()) {
            continue;
        }
        let target = dirs.system.join(&f.path);
        let source = library_files
            .get(&f.sha1)
            .cloned()
            .or_else(|| by_name(library, &f.path));
        match (source, target.exists()) {
            (Some(src), true) => {
                if same_bytes(&src, &target) {
                    ex.bios_present += 1;
                } else {
                    ex.bios_conflicts.push(f.path.clone());
                }
            }
            (Some(src), false) => {
                ex.ops.push(Op::Copy {
                    from: src,
                    to: target,
                });
                ex.bios_copied.push(f.path.clone());
            }
            (None, true) => ex.bios_present += 1,
            (None, false) if systems.contains(&f.system.to_lowercase()) => {
                ex.bios_missing.push((f.system.clone(), f.path.clone()));
            }
            (None, false) => {}
        }
    }
}

/// Whole-file SHA1 → path for every file (and archive) of a library scan.
pub fn files_by_sha1(report: &crate::ScanReport) -> HashMap<[u8; 20], PathBuf> {
    report
        .roms
        .iter()
        .chain(&report.archives)
        .filter(|r| r.member.is_none())
        .map(|r| (r.hashes.sha1, r.path.clone()))
        .collect()
}

/// Arcade BIOS sets are matched by name: `_bios/` is laid out like the system folder, and
/// FBNeo sets also serve requests for the bare name.
fn by_name(library: &Path, path: &str) -> Option<PathBuf> {
    if !path.ends_with(".zip") {
        return None;
    }
    let bios = library.join(BIOS_DIR);
    [bios.join(path), bios.join("fbneo").join(path)]
        .into_iter()
        .find(|p| p.is_file())
}

/// System folders at the top of the library (lower case: `System.dat` spells some differently).
fn library_systems(library: &Path) -> BTreeSet<String> {
    std::fs::read_dir(library)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

fn same_bytes(a: &Path, b: &Path) -> bool {
    match (std::fs::read(a), std::fs::read(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn plans_playlists_with_core_and_bios_copies() {
        let tmp = tempfile::tempdir().unwrap();
        let (lib, ra) = (tmp.path().join("lib"), tmp.path().join("ra"));
        let dirs = Dirs {
            playlists: ra.join("playlists"),
            system: ra.join("system"),
            cores: ra.join("cores"),
            info: ra.join("cores"),
        };
        fs::create_dir_all(lib.join("_playlists")).unwrap();
        fs::create_dir_all(lib.join("Sony - PlayStation")).unwrap();
        fs::create_dir_all(lib.join("_bios/fbneo")).unwrap();
        fs::write(
            lib.join("_playlists/Nintendo - SNES.lpl"),
            r#"{"default_core_path":"","default_core_name":"","items":[]}"#,
        )
        .unwrap();
        fs::write(lib.join("st.sfc"), b"st bios").unwrap();
        fs::write(lib.join("_bios/fbneo/neogeo.zip"), b"zip").unwrap();
        let fw = |system: &str, path: &str, sha: u8| Firmware {
            system: system.into(),
            path: path.into(),
            size: 1,
            sha1: [sha; 20],
        };
        let firmware = [
            fw("Nintendo - SuFami Turbo", "STBIOS.bin", 1),
            fw("Arcade", "fbneo/neogeo.zip", 2),
            fw("Sony - PlayStation", "scph5501.bin", 3),
            fw("Sega - Saturn", "saturn_bios.bin", 4),
        ];
        let files = HashMap::from([([1u8; 20], lib.join("st.sfc"))]);
        let cores = [Core {
            path: "/c/snes9x_libretro.so".into(),
            name: "Snes9x".into(),
            databases: vec!["Nintendo - SNES".into()],
        }];
        let ex = plan(&lib, &dirs, &cores, &firmware, &files, true, true);
        assert_eq!(
            ex.playlists,
            [("Nintendo - SNES".into(), Some("Snes9x".into()))]
        );
        assert_eq!(ex.bios_copied, ["STBIOS.bin", "fbneo/neogeo.zip"]);
        assert_eq!(
            ex.bios_missing,
            [("Sony - PlayStation".into(), "scph5501.bin".into())]
        );
        let r = crate::plan::execute(&ex.ops);
        assert!(r.error.is_none());
        assert_eq!(fs::read(ra.join("system/STBIOS.bin")).unwrap(), b"st bios");
        let pl = fs::read_to_string(ra.join("playlists/Nintendo - SNES.lpl")).unwrap();
        assert!(pl.contains("snes9x_libretro.so"));

        // second run: nothing left to do
        let again = plan(&lib, &dirs, &cores, &firmware, &files, true, true);
        assert!(again.ops.is_empty());
        assert_eq!((again.playlists_unchanged, again.bios_present), (1, 2));
    }
}
