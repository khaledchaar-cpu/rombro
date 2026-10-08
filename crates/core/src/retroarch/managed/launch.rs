//! Starting a game in the managed RetroArch: the core info files (buildbot `info.zip`),
//! installing missing cores from the buildbot and building the command line.

use super::{FetchFile, Managed, Target};
use crate::retroarch::{info, info::Core, pick};
use std::collections::BTreeMap;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Info files of all cores, kept up to date by the libretro buildbot.
pub const INFO_URL: &str = "https://buildbot.libretro.com/assets/frontend/info.zip";

impl Target {
    /// Buildbot folder with the latest core builds (`<core>_libretro.<ext>.zip`).
    pub fn cores_url(self) -> &'static str {
        match self {
            Self::LinuxX64 => "https://buildbot.libretro.com/nightly/linux/x86_64/latest",
            Self::WindowsX64 => "https://buildbot.libretro.com/nightly/windows/x86_64/latest",
            Self::MacUniversal if std::env::consts::ARCH == "aarch64" => {
                "https://buildbot.libretro.com/nightly/apple/osx/arm64/latest"
            }
            Self::MacUniversal => "https://buildbot.libretro.com/nightly/apple/osx/x86_64/latest",
        }
    }
}

impl Managed {
    pub fn cores_dir(&self) -> PathBuf {
        self.root.join("cores")
    }

    pub fn info_dir(&self) -> PathBuf {
        self.root.join("info")
    }

    /// All cores with an info file, installed or not.
    /// Buildbot listing of the cores built for this target (`.index-extended`).
    pub fn index_path(&self) -> PathBuf {
        self.root.join("cores.index")
    }

    /// Cores with an info file that are installed or downloadable for this target
    /// (info files also exist for cores the buildbot does not build, e.g. FBNeo subsets).
    pub fn cores(&self) -> Vec<Core> {
        let mut all = info::available(&self.info_dir(), &self.cores_dir());
        if let Ok(index) = std::fs::read_to_string(self.index_path()) {
            all.retain(|c| c.installed || on_buildbot(&index, c));
        }
        all
    }

    /// Info files and core index are missing or older than a week.
    pub fn info_stale(&self) -> bool {
        let age = std::fs::metadata(self.index_path())
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok());
        !has_info(&self.info_dir()) || age.is_none_or(|a| a.as_secs() > 7 * 86_400)
    }

    /// Downloads the info files unless present (or `refresh`), replacing the folder atomically.
    pub fn ensure_info(&self, fetch: FetchFile, refresh: bool) -> io::Result<()> {
        let dir = self.info_dir();
        if !refresh && !self.info_stale() {
            return Ok(());
        }
        let index_url = format!("{}/.index-extended", self.target.cores_url());
        let tmp_index = self.root.join("cores.index.tmp");
        std::fs::create_dir_all(&self.root)?;
        fetch(&index_url, &tmp_index, &|_, _| {})
            .map_err(|e| io::Error::other(format!("{index_url}: {e}")))?;
        std::fs::rename(&tmp_index, self.index_path())?;
        let dl = self.root.join("download");
        std::fs::create_dir_all(&dl)?;
        let zip = dl.join("info.zip");
        fetch(INFO_URL, &zip, &|_, _| {})
            .map_err(|e| io::Error::other(format!("{INFO_URL}: {e}")))?;
        let tmp = self.root.join(".info.tmp");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp)?;
        unzip_info(&zip, &tmp)?;
        let _ = std::fs::remove_file(&zip);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::rename(tmp, dir)
    }

    /// Downloads `core` from the buildbot into the core folder (atomically).
    pub fn install_core(&self, core: &Core, fetch: FetchFile) -> io::Result<()> {
        let file = core
            .path
            .file_name()
            .ok_or_else(|| io::Error::other("core without file name"))?
            .to_string_lossy()
            .into_owned();
        let url = format!("{}/{file}.zip", self.target.cores_url());
        let dl = self.root.join("download");
        std::fs::create_dir_all(&dl)?;
        let zip = dl.join(format!("{file}.zip"));
        fetch(&url, &zip, &|_, _| {}).map_err(|e| io::Error::other(format!("{url}: {e}")))?;
        std::fs::create_dir_all(self.cores_dir())?;
        let tmp = self.cores_dir().join(format!(".{file}.tmp"));
        let _ = std::fs::remove_file(&tmp);
        let res = crate::archive::extract(&zip, &file, &tmp);
        let _ = std::fs::remove_file(&zip);
        res.map_err(|e| io::Error::other(format!("{url}: {e}")))?;
        std::fs::rename(tmp, &core.path)
    }

    /// `retroarch --config <cfg> -L <core> <rom>`.
    pub fn command(&self, core: &Core, rom: &Path) -> io::Result<Command> {
        let exe = self
            .executable()
            .ok_or_else(|| io::Error::other("RetroArch is not installed"))?;
        let mut cmd = Command::new(exe);
        cmd.arg("--config")
            .arg(self.cfg())
            .arg("-L")
            .arg(&core.path)
            .arg(rom);
        Ok(cmd)
    }
}

/// What a start needs to do first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    RetroArch(super::Phase),
    Info,
    Core(String),
    /// Downloading system files a core needs (asset zip name).
    Assets(String),
}

/// A game ready to start.
pub struct Launch {
    pub system: String,
    pub core: Core,
    pub command: Command,
    /// System files the core still needs (extractions into the system folder). In a library
    /// this is `_bios`, so the caller executes them like any plan (journaled, undoable).
    pub assets: Vec<crate::plan::Op>,
}

/// Inputs of [`Managed::prepare`].
pub struct Game<'a> {
    pub rom: &'a Path,
    pub library: Option<&'a Path>,
    pub picks: &'a BTreeMap<String, String>,
    pub core: Option<&'a str>,
}

impl Managed {
    /// Installs what is missing (RetroArch, info files, the core), writes the config and
    /// returns the command that starts `game`.
    pub fn prepare(
        &self,
        game: &Game,
        fetch: FetchFile,
        progress: &dyn Fn(Step),
    ) -> io::Result<Launch> {
        if self.current().is_none() {
            self.install(super::PINNED, fetch, &|p| progress(Step::RetroArch(p)))?;
        }
        if self.info_stale() {
            progress(Step::Info);
            // Offline with older info files: start with what is there.
            if let Err(e) = self.ensure_info(fetch, false)
                && !has_info(&self.info_dir())
            {
                return Err(e);
            }
        }
        let cores = self.cores();
        let system = system_of(game.rom, game.library, &cores).ok_or_else(|| {
            io::Error::other(format!("{}: unknown system folder", game.rom.display()))
        })?;
        let core = core_for_game(&cores, &system, game.picks, game.core)
            .ok_or_else(|| io::Error::other(format!("no core runs {system}")))?
            .clone();
        if !core.path.is_file() {
            progress(Step::Core(core.id.clone()));
            self.install_core(&core, fetch)?;
        }
        let assets = self.core_assets(&core, game.library, fetch, progress)?;
        self.write_config(game.library)?;
        let command = self.command(&core, &content(&core, game.rom))?;
        Ok(Launch {
            system,
            core,
            command,
            assets,
        })
    }

    /// Downloads the asset zip `core` cannot start without (kept under `download/assets/`)
    /// and returns the extractions the system folder still lacks.
    fn core_assets(
        &self,
        core: &Core,
        library: Option<&Path>,
        fetch: FetchFile,
        progress: &dyn Fn(Step),
    ) -> io::Result<Vec<crate::plan::Op>> {
        use crate::retroarch::assets;
        let system = library.map_or_else(
            || self.root.join("system"),
            |l| l.join(crate::plan::BIOS_DIR),
        );
        let Some(name) = assets::missing(&core.id, &system) else {
            return Ok(vec![]);
        };
        let dir = self.root.join("download").join("assets");
        let zip = dir.join(name);
        if !zip.is_file() {
            progress(Step::Assets(name.to_owned()));
            std::fs::create_dir_all(&dir)?;
            let url = assets::url(self.target.cores_url(), name);
            let part = dir.join(format!(".{name}.part"));
            fetch(&url, &part, &|_, _| {}).map_err(|e| io::Error::other(format!("{url}: {e}")))?;
            std::fs::rename(&part, &zip)?;
        }
        assets::unpack_ops(&zip, &system)
    }
}

/// `index` (lines `date crc file`) lists the archive of `core`.
fn on_buildbot(index: &str, core: &Core) -> bool {
    let Some(file) = core.path.file_name().and_then(|f| f.to_str()) else {
        return false;
    };
    index
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .any(|f| f.strip_suffix(".zip") == Some(file))
}

fn has_info(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|mut rd| {
        rd.any(|e| e.is_ok_and(|e| e.path().extension().is_some_and(|x| x == "info")))
    })
}

/// Writes the `*.info` members of `zip` (flat) into `dir`.
fn unzip_info(zip: &Path, dir: &Path) -> io::Result<()> {
    let mut z = zip::ZipArchive::new(BufReader::new(std::fs::File::open(zip)?))
        .map_err(io::Error::other)?;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(io::Error::other)?;
        let Some(name) = f
            .enclosed_name()
            .and_then(|p| p.file_name().map(|n| n.to_owned()))
        else {
            continue;
        };
        if !f.is_file() || Path::new(&name).extension().is_none_or(|e| e != "info") {
            continue;
        }
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf)?;
        std::fs::write(dir.join(name), buf)?;
    }
    Ok(())
}

/// The system (RDB name) of a library file: its top folder below `library`, else the
/// nearest parent folder some core runs.
pub fn system_of(rom: &Path, library: Option<&Path>, cores: &[Core]) -> Option<String> {
    if let Some(top) = library
        .and_then(|l| rom.strip_prefix(l).ok())
        .and_then(|r| r.components().next())
        .filter(|_| rom.parent() != library)
    {
        return Some(top.as_os_str().to_string_lossy().into_owned());
    }
    rom.ancestors()
        .skip(1)
        .filter_map(|p| p.file_name()?.to_str())
        .find(|name| {
            pick::recommended(name).is_some()
                || cores.iter().any(|c| c.databases.iter().any(|d| d == name))
        })
        .map(str::to_owned)
}

/// What the core is given: an `.m3u` it cannot load (FCEUmm, Famicom Disk System) is
/// replaced by its first entry.
pub fn content(core: &Core, rom: &Path) -> PathBuf {
    let m3u = rom
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("m3u"));
    if !m3u || core.extensions.is_empty() || core.extensions.iter().any(|e| e == "m3u") {
        return rom.to_path_buf();
    }
    std::fs::read_to_string(rom)
        .ok()
        .and_then(|t| {
            t.lines()
                .map(str::trim)
                .find(|l| !l.is_empty() && !l.starts_with('#'))
                .map(|l| rom.with_file_name(l))
        })
        .unwrap_or_else(|| rom.to_path_buf())
}

/// The core a game starts with: the per-game override, else the system's pick or
/// recommendation, else the most specialised core (installed or not).
pub fn core_for_game<'a>(
    cores: &'a [Core],
    system: &str,
    picks: &BTreeMap<String, String>,
    over: Option<&str>,
) -> Option<&'a Core> {
    over.and_then(|id| cores.iter().find(|c| c.id == id))
        .or_else(|| pick::resolve(cores, system, picks, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core(id: &str, dbs: &[&str]) -> Core {
        Core {
            path: PathBuf::from(format!("/c/{id}_libretro.so")),
            id: id.into(),
            installed: false,
            name: id.into(),
            databases: dbs.iter().map(|s| s.to_string()).collect(),
            extensions: Vec::new(),
        }
    }

    #[test]
    fn m3u_becomes_first_disk_for_cores_without_m3u() {
        let tmp = tempfile::TempDir::new().unwrap();
        let m3u = tmp.path().join("T (Japan).m3u");
        std::fs::write(&m3u, "T (Japan) (Disk 1).fds\nT (Japan) (Disk 2).fds\n").unwrap();
        let mut c = core("fceumm", &[]);
        c.extensions = vec!["fds".into(), "nes".into()];
        assert_eq!(content(&c, &m3u), tmp.path().join("T (Japan) (Disk 1).fds"));
        c.extensions.push("m3u".into());
        assert_eq!(content(&c, &m3u), m3u);
    }

    #[test]
    fn system_from_library_top_folder_or_known_parent() {
        let cores = [core(
            "fceumm",
            &["Nintendo - Nintendo Entertainment System"],
        )];
        let lib = Path::new("/lib");
        let rom = Path::new("/lib/Sony - PlayStation/Game (USA)/Game (USA).m3u");
        assert_eq!(
            system_of(rom, Some(lib), &cores).as_deref(),
            Some("Sony - PlayStation")
        );
        let rom = Path::new("/x/Nintendo - Nintendo Entertainment System/a.nes");
        assert_eq!(
            system_of(rom, None, &cores).as_deref(),
            Some("Nintendo - Nintendo Entertainment System")
        );
        assert_eq!(system_of(Path::new("/x/y/a.nes"), None, &cores), None);
    }

    #[test]
    fn buildbot_index_lists_core_archives() {
        let index = "2026-10-08 dcff8275 fbneo_libretro.so.zip\n";
        assert!(on_buildbot(index, &core("fbneo", &[])));
        assert!(!on_buildbot(index, &core("fbneo_cps12", &[])));
    }

    #[test]
    fn override_wins_over_system_pick() {
        let sys = "Nintendo - Nintendo Entertainment System";
        let cores = [core("fceumm", &[sys]), core("nestopia", &[sys])];
        let mut picks = BTreeMap::new();
        picks.insert(sys.to_owned(), "fceumm".to_owned());
        assert_eq!(
            core_for_game(&cores, sys, &picks, None).unwrap().id,
            "fceumm"
        );
        let got = core_for_game(&cores, sys, &picks, Some("nestopia")).unwrap();
        assert_eq!(got.id, "nestopia");
        let got = core_for_game(&cores, sys, &picks, Some("gone")).unwrap();
        assert_eq!(got.id, "fceumm");
    }
}
