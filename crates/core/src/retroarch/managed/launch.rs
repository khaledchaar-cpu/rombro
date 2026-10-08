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
    pub fn cores(&self) -> Vec<Core> {
        info::available(&self.info_dir(), &self.cores_dir())
    }

    /// Downloads the info files unless present (or `refresh`), replacing the folder atomically.
    pub fn ensure_info(&self, fetch: FetchFile, refresh: bool) -> io::Result<()> {
        let dir = self.info_dir();
        if !refresh && has_info(&dir) {
            return Ok(());
        }
        let dl = self.root.join("download");
        std::fs::create_dir_all(&dl)?;
        let zip = dl.join("info.zip");
        fetch(INFO_URL, &zip, &|_, _| {}).map_err(|e| io::Error::other(format!("{INFO_URL}: {e}")))?;
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
}

/// A game ready to start.
pub struct Launch {
    pub system: String,
    pub core: Core,
    pub command: Command,
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
        if !has_info(&self.info_dir()) {
            progress(Step::Info);
            self.ensure_info(fetch, false)?;
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
        self.write_config(game.library)?;
        let command = self.command(&core, game.rom)?;
        Ok(Launch {
            system,
            core,
            command,
        })
    }
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
        }
    }

    #[test]
    fn system_from_library_top_folder_or_known_parent() {
        let cores = [core("fceumm", &["Nintendo - Nintendo Entertainment System"])];
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
    fn override_wins_over_system_pick() {
        let sys = "Nintendo - Nintendo Entertainment System";
        let cores = [core("fceumm", &[sys]), core("nestopia", &[sys])];
        let mut picks = BTreeMap::new();
        picks.insert(sys.to_owned(), "fceumm".to_owned());
        assert_eq!(core_for_game(&cores, sys, &picks, None).unwrap().id, "fceumm");
        let got = core_for_game(&cores, sys, &picks, Some("nestopia")).unwrap();
        assert_eq!(got.id, "nestopia");
        let got = core_for_game(&cores, sys, &picks, Some("gone")).unwrap();
        assert_eq!(got.id, "fceumm");
    }
}
