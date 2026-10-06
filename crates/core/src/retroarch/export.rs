//! Export to RetroArch: the library playlists (with the matching installed core as default)
//! into RetroArch's playlist folder, identified firmware into its system folder, and, on
//! request, the missing cores from RetroArch's buildbot into its core folder.
//! Only plans operations; executing them goes through the journal like any import. Core
//! archives are downloaded by the caller (see [`Export::downloads`]) before executing.

use super::Dirs;
use super::firmware::Firmware;
use super::info::Core;
use super::pick;
use crate::plan::{BIOS_DIR, Op, PLAYLIST_DIR, TRASH_DIR};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// What to export.
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub playlists: bool,
    pub bios: bool,
    /// Install the core each playlist wants if it is missing.
    pub install_cores: bool,
    /// Core per system chosen by the user (system → core id); others use the recommendation.
    pub picks: BTreeMap<String, String>,
    /// Folder for downloaded core archives.
    pub cache: PathBuf,
}

/// The default folder for downloaded core archives (`<cache>/rombro/cores`).
pub fn default_cache() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("rombro/cores"))
}

/// Planned operations plus what the user should know about them.
#[derive(Debug, Default)]
pub struct Export {
    pub ops: Vec<Op>,
    /// Playlists written (system, core name if one is installed or will be).
    pub playlists: Vec<(String, Option<String>)>,
    /// Cores to install (id), each extracted from an archive in [`Self::downloads`].
    pub cores_install: Vec<String>,
    /// Core archives to download before executing: (url, file).
    pub downloads: Vec<(String, PathBuf)>,
    /// Wanted cores that are missing and not to be installed (system, core id).
    pub cores_missing: Vec<(String, String)>,
    /// Systems no known core runs (no core info lists them): their playlist is not exported.
    pub playlists_no_core: Vec<String>,
    /// Earlier exported playlists whose system has no games left (moved to the trash).
    pub playlists_removed: Vec<String>,
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
    opts: &Options,
) -> Export {
    let mut ex = Export::default();
    let systems = library_systems(library);
    if opts.playlists {
        plan_playlists(library, dirs, cores, opts, &mut ex);
    }
    if opts.bios {
        plan_bios(library, dirs, firmware, library_files, &systems, &mut ex);
    }
    ex
}

fn plan_playlists(library: &Path, dirs: &Dirs, cores: &[Core], opts: &Options, ex: &mut Export) {
    let install = opts.install_cores && dirs.buildbot.is_some();
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
        // RetroArch could not start any entry (e.g. Solarus: standalone engine only)
        if !cores.is_empty() && pick::resolve(cores, &system, &opts.picks, true).is_none() {
            ex.playlists_no_core.push(system);
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&src) else {
            continue;
        };
        let Ok(mut doc) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if let Some(w) = pick::resolve(cores, &system, &opts.picks, true)
            && !w.installed
            && !install
        {
            ex.cores_missing.push((system.clone(), w.id.clone()));
        }
        let core = pick::resolve(cores, &system, &opts.picks, install);
        if let (Some(c), Some(url)) = (core, &dirs.buildbot)
            && !c.installed
            && !ex.cores_install.contains(&c.id)
        {
            plan_install(c, url, &opts.cache, ex);
        }
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
    plan_stale(library, dirs, ex);
}

/// RetroArch playlists without a library playlist whose entries all point into the library
/// (exported by us earlier, system now empty) go to `<library>/_trash/playlists`.
/// Playlists of other content are left alone.
fn plan_stale(library: &Path, dirs: &Dirs, ex: &mut Export) {
    let Ok(rd) = std::fs::read_dir(&dirs.playlists) else {
        return;
    };
    let mut stale: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "lpl"))
        .filter(|p| {
            // systems without core count as gone: an old export of theirs is stale too
            let no_core = p.file_stem().is_some_and(|s| {
                ex.playlists_no_core
                    .iter()
                    .any(|n| *s.to_string_lossy() == *n)
            });
            p.file_name()
                .is_some_and(|n| no_core || !library.join(PLAYLIST_DIR).join(n).exists())
        })
        .filter(|p| from_library(p, library))
        .collect();
    stale.sort();
    for from in stale {
        let Some(name) = from.file_name() else {
            continue;
        };
        let to = library.join(TRASH_DIR).join("playlists").join(name);
        ex.playlists_removed
            .push(name.to_string_lossy().into_owned());
        ex.ops.push(Op::Move { from, to });
    }
}

/// Whether the playlist at `p` has entries and all of them lie inside `library`.
fn from_library(p: &Path, library: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(p) else {
        return false;
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    let Some(items) = doc["items"].as_array().filter(|i| !i.is_empty()) else {
        return false;
    };
    items.iter().all(|i| {
        i["path"]
            .as_str()
            .is_some_and(|s| Path::new(s.split('#').next().unwrap_or(s)).starts_with(library))
    })
}

/// Systems with a playlist in the library, sorted.
pub fn playlist_systems(library: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(library.join(PLAYLIST_DIR))
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|e| e == "lpl"))
                .filter_map(|p| Some(p.file_stem()?.to_string_lossy().into_owned()))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Downloads the core archives of `ex` with `fetch` into the cache (always fresh: the
/// buildbot serves the latest build under the same name). Reports (done, total, current
/// file name) to `progress` before each download and once at the end.
pub fn download(
    ex: &Export,
    fetch: &dyn Fn(&str) -> std::io::Result<Vec<u8>>,
    progress: &dyn Fn(usize, usize, &str),
) -> std::io::Result<()> {
    let total = ex.downloads.len();
    for (i, (url, file)) in ex.downloads.iter().enumerate() {
        let name = url.rsplit('/').next().unwrap_or(url);
        progress(i, total, name);
        let body = fetch(url).map_err(|e| std::io::Error::other(format!("{url}: {e}")))?;
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = file.with_extension("part");
        std::fs::write(&tmp, body)?;
        std::fs::rename(tmp, file)?;
    }
    progress(total, total, "");
    Ok(())
}

/// Download of `<core file>.zip` from the buildbot, then extraction into the core folder.
fn plan_install(core: &Core, buildbot: &str, cache: &Path, ex: &mut Export) {
    let Some(file) = core.path.file_name() else {
        return;
    };
    let file = file.to_string_lossy().into_owned();
    let zip = cache.join(format!("{file}.zip"));
    ex.downloads
        .push((format!("{buildbot}/{file}.zip"), zip.clone()));
    ex.ops.push(Op::Extract {
        archive: zip,
        member: file,
        to: core.path.clone(),
    });
    ex.cores_install.push(core.id.clone());
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
            (None, false) if in_library(&f.system, systems) => {
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
/// Whether the library has a folder for the firmware's system (also under its RDB name).
fn in_library(system: &str, systems: &BTreeSet<String>) -> bool {
    std::iter::once(system)
        .chain(super::firmware::library_systems(system).iter().copied())
        .any(|s| systems.contains(&s.to_lowercase()))
}

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
            buildbot: None,
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
            id: "snes9x".into(),
            installed: true,
            name: "Snes9x".into(),
            databases: vec!["Nintendo - SNES".into()],
        }];
        let opts = Options {
            playlists: true,
            bios: true,
            ..Options::default()
        };
        let ex = plan(&lib, &dirs, &cores, &firmware, &files, &opts);
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
        let again = plan(&lib, &dirs, &cores, &firmware, &files, &opts);
        assert!(again.ops.is_empty());
        assert_eq!((again.playlists_unchanged, again.bios_present), (1, 2));

        // a system without games left: its exported playlist goes to the trash, foreign ones stay
        let item = |p: &Path| format!(r#"{{"items":[{{"path":"{}"}}]}}"#, p.display());
        fs::write(ra.join("playlists/MAME.lpl"), item(&lib.join("MAME/x.zip"))).unwrap();
        fs::write(
            ra.join("playlists/Mine.lpl"),
            item(Path::new("/elsewhere/y.zip")),
        )
        .unwrap();
        let gone = plan(&lib, &dirs, &cores, &firmware, &files, &opts);
        assert_eq!(gone.playlists_removed, ["MAME.lpl"]);
        assert!(gone.playlists_no_core.is_empty());
        assert!(crate::plan::execute(&gone.ops).error.is_none());
        assert!(lib.join("_trash/playlists/MAME.lpl").exists());
        assert!(ra.join("playlists/Mine.lpl").exists());

        // a system no core runs gets no playlist (an old export of it goes to the trash)
        let z = item(&lib.join("Solarus/z.solarus"));
        fs::write(lib.join("_playlists/Solarus.lpl"), &z).unwrap();
        fs::write(ra.join("playlists/Solarus.lpl"), &z).unwrap();
        let none = plan(&lib, &dirs, &cores, &firmware, &files, &opts);
        assert_eq!(none.playlists_no_core, ["Solarus"]);
        assert_eq!(none.playlists_removed, ["Solarus.lpl"]);
    }

    #[test]
    fn installs_missing_recommended_core() {
        const SNES: &str = "Nintendo - Super Nintendo Entertainment System";
        let tmp = tempfile::tempdir().unwrap();
        let (lib, ra) = (tmp.path().join("lib"), tmp.path().join("ra"));
        let dirs = Dirs {
            playlists: ra.join("playlists"),
            system: ra.join("system"),
            cores: ra.join("cores"),
            info: ra.join("cores"),
            buildbot: Some("https://bb/latest".into()),
        };
        fs::create_dir_all(lib.join("_playlists")).unwrap();
        fs::write(
            lib.join(format!("_playlists/{SNES}.lpl")),
            r#"{"items":[]}"#,
        )
        .unwrap();
        let core = |id: &str, installed| Core {
            path: ra.join(format!("cores/{id}_libretro.so")),
            id: id.into(),
            installed,
            name: id.into(),
            databases: vec![SNES.into()],
        };
        let cores = [core("bsnes", true), core("snes9x", false)];
        let mut opts = Options {
            playlists: true,
            cache: tmp.path().join("cache"),
            ..Options::default()
        };
        // Without install: the installed core, and the recommendation reported missing.
        let ex = plan(&lib, &dirs, &cores, &[], &HashMap::new(), &opts);
        assert_eq!(ex.playlists, [(SNES.into(), Some("bsnes".into()))]);
        assert_eq!(ex.cores_missing, [(SNES.into(), "snes9x".into())]);
        assert!(ex.downloads.is_empty());

        opts.install_cores = true;
        let ex = plan(&lib, &dirs, &cores, &[], &HashMap::new(), &opts);
        assert_eq!(ex.cores_install, ["snes9x"]);
        let (url, zip) = &ex.downloads[0];
        assert_eq!(url, "https://bb/latest/snes9x_libretro.so.zip");
        // Stand-in for the download.
        fs::create_dir_all(zip.parent().unwrap()).unwrap();
        let mut w = zip::ZipWriter::new(fs::File::create(zip).unwrap());
        w.start_file(
            "snes9x_libretro.so",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        std::io::Write::write_all(&mut w, b"ELF").unwrap();
        w.finish().unwrap();
        let r = crate::plan::execute(&ex.ops);
        assert!(r.error.is_none(), "{:?}", r.error);
        assert_eq!(
            fs::read(ra.join("cores/snes9x_libretro.so")).unwrap(),
            b"ELF"
        );
        let pl = fs::read_to_string(ra.join(format!("playlists/{SNES}.lpl"))).unwrap();
        assert!(pl.contains("snes9x_libretro.so"));
    }

    #[test]
    fn firmware_systems_match_library_folders_under_their_rdb_name() {
        let systems: BTreeSet<String> = ["nec - pc engine cd - turbografx-cd".to_owned()].into();
        assert!(in_library(
            "NEC - PC Engine - TurboGrafx 16 - SuperGrafx",
            &systems
        ));
        assert!(!in_library("Sega - Mega CD - Sega CD", &systems));
    }
}
