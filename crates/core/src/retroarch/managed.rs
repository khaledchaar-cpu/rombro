//! The RetroArch romburak downloads and runs itself: official stable builds from the
//! libretro buildbot, unpacked into `<data dir>/romburak/retroarch/versions/<version>/`.
//! Download → verify → unpack into a temp folder → atomic rename → `current` marker.
//! Cores, saves, states and the config live next to the versions, so updates keep them.

use std::io;
use std::path::{Path, PathBuf};

pub mod cheevos;
pub mod config;
pub mod display;
pub mod launch;
mod unpack;

/// Version shipped with this romburak release (raised with releases).
pub const PINNED: &str = "1.22.2";

/// Buildbot root of the stable builds.
pub const STABLE_URL: &str = "https://buildbot.libretro.com/stable";

/// SHA-256 of the pinned archives per target; newer versions are checked by archive CRCs only.
const PINNED_SHA256: &[(Target, &str)] = &[
    (
        Target::LinuxX64,
        "7d62da9a21397d6e1b9490785cedbeafd262781b50115076736fbe8a77ef30e9",
    ),
    (
        Target::WindowsX64,
        "b2139b1d0f9d4526dc6b5ce23cbb3efdc766096fa6f2c3df016818b486ac6372",
    ),
    (
        Target::MacUniversal,
        "81b79121ba26d539064ae13b4d0419a120c3d165afbe656cf5f5412b15fdb434",
    ),
];

/// Download with progress (bytes done, total if known) straight into a file.
pub type FetchFile<'a> = &'a dyn Fn(&str, &Path, &dyn Fn(u64, Option<u64>)) -> io::Result<()>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    LinuxX64,
    WindowsX64,
    MacUniversal,
}

impl Target {
    /// Target of this build, `None` where the buildbot has no stable desktop build.
    pub fn current() -> Option<Self> {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => Some(Self::LinuxX64),
            ("windows", "x86_64") => Some(Self::WindowsX64),
            ("macos", _) => Some(Self::MacUniversal),
            _ => None,
        }
    }

    fn remote(self) -> &'static str {
        match self {
            Self::LinuxX64 => "linux/x86_64/RetroArch.7z",
            Self::WindowsX64 => "windows/x86_64/RetroArch.7z",
            Self::MacUniversal => "apple/osx/universal/RetroArch_Metal.dmg",
        }
    }

    /// Executable relative to the version folder.
    pub fn executable(self) -> &'static str {
        match self {
            Self::LinuxX64 => "RetroArch-Linux-x86_64/RetroArch-Linux-x86_64.AppImage",
            Self::WindowsX64 => "RetroArch-Win64/retroarch.exe",
            Self::MacUniversal => "RetroArch.app/Contents/MacOS/RetroArch",
        }
    }

    /// Bundled assets folder (menu icons, fonts) relative to the version folder; the macOS
    /// app finds its own.
    pub fn assets(self) -> Option<&'static str> {
        match self {
            Self::LinuxX64 => Some(
                "RetroArch-Linux-x86_64/RetroArch-Linux-x86_64.AppImage.home/.config/retroarch/assets",
            ),
            Self::WindowsX64 => Some("RetroArch-Win64/assets"),
            Self::MacUniversal => None,
        }
    }

    pub fn url(self, version: &str) -> String {
        format!("{STABLE_URL}/{version}/{}", self.remote())
    }

    fn sha256(self, version: &str) -> Option<&'static str> {
        (version == PINNED)
            .then(|| PINNED_SHA256.iter().find(|(t, _)| *t == self))
            .flatten()
            .map(|(_, h)| *h)
    }
}

/// Folder layout of the managed RetroArch.
#[derive(Debug, Clone)]
pub struct Managed {
    pub root: PathBuf,
    pub target: Target,
    /// Fullscreen or window, written into the config; `None` keeps RetroArch's setting.
    pub display: Option<display::Display>,
    /// RetroAchievements login + hardcore; `None` keeps RetroArch's settings.
    pub cheevos: Option<cheevos::Cheevos>,
}

impl Managed {
    /// `<data dir>/romburak/retroarch` for this OS.
    pub fn detect() -> Option<Self> {
        Some(Self {
            root: dirs::data_dir()?.join(crate::paths::APP).join("retroarch"),
            target: Target::current()?,
            display: None,
            cheevos: None,
        })
    }

    pub fn version_dir(&self, version: &str) -> PathBuf {
        self.root.join("versions").join(version)
    }

    /// Installed version in use, if any.
    pub fn current(&self) -> Option<String> {
        let v = std::fs::read_to_string(self.root.join("current")).ok()?;
        let v = v.trim().to_owned();
        self.executable_of(&v).is_file().then_some(v)
    }

    pub fn executable_of(&self, version: &str) -> PathBuf {
        self.version_dir(version).join(self.target.executable())
    }

    /// Executable of the version in use.
    pub fn executable(&self) -> Option<PathBuf> {
        self.current().map(|v| self.executable_of(&v))
    }

    /// Savestates folder (`savestate_directory` in the managed config).
    pub fn states_dir(&self) -> PathBuf {
        self.root.join("states")
    }

    pub fn cfg(&self) -> PathBuf {
        self.root.join("retroarch.cfg")
    }

    /// Downloads, verifies and installs `version`, then makes it current. An already
    /// installed version is only re-activated. Older versions are removed afterwards.
    pub fn install(
        &self,
        version: &str,
        fetch: FetchFile,
        progress: &dyn Fn(Phase),
    ) -> io::Result<()> {
        if !self.executable_of(version).is_file() {
            let dl = self.root.join("download");
            std::fs::create_dir_all(&dl)?;
            let archive = dl.join(self.target.remote().rsplit('/').next().unwrap_or("ra"));
            let url = self.target.url(version);
            fetch(&url, &archive, &|done, total| {
                progress(Phase::Download { done, total })
            })
            .map_err(|e| io::Error::other(format!("{url}: {e}")))?;
            progress(Phase::Verify);
            if let Some(want) = self.target.sha256(version) {
                let got = sha256_file(&archive)?;
                if got != want {
                    let _ = std::fs::remove_file(&archive);
                    return Err(io::Error::other(format!(
                        "{url}: checksum mismatch (got {got}, want {want})"
                    )));
                }
            }
            progress(Phase::Unpack { done: 0, total: 0 });
            let tmp = self.root.join("versions").join(format!(".{version}.tmp"));
            let _ = std::fs::remove_dir_all(&tmp);
            std::fs::create_dir_all(&tmp)?;
            unpack::unpack(self.target, &archive, &tmp, &|done, total| {
                progress(Phase::Unpack { done, total })
            })?;
            let exe = tmp.join(self.target.executable());
            if !exe.is_file() {
                let _ = std::fs::remove_dir_all(&tmp);
                return Err(io::Error::other(format!(
                    "{url}: archive lacks {}",
                    self.target.executable()
                )));
            }
            set_executable(&exe)?;
            let dest = self.version_dir(version);
            let _ = std::fs::remove_dir_all(&dest);
            std::fs::rename(&tmp, &dest)?;
            let _ = std::fs::remove_dir_all(&dl);
        }
        let marker = self.root.join("current.tmp");
        std::fs::write(&marker, version)?;
        std::fs::rename(marker, self.root.join("current"))?;
        self.prune(version);
        progress(Phase::Done);
        Ok(())
    }

    /// Removes every installed version except `keep`.
    fn prune(&self, keep: &str) {
        let Ok(rd) = std::fs::read_dir(self.root.join("versions")) else {
            return;
        };
        for e in rd.flatten() {
            if e.file_name() != keep {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
}

/// Install progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Download {
        done: u64,
        total: Option<u64>,
    },
    Verify,
    /// Bytes unpacked of `total` (0 while unknown, e.g. for a disk image).
    Unpack {
        done: u64,
        total: u64,
    },
    Done,
}

/// Newest version in the buildbot's `/stable/` listing.
pub fn latest_stable(index_html: &str) -> Option<String> {
    index_html
        .split("href=\"")
        .skip(1)
        .filter_map(|s| s.split('"').next())
        .filter_map(|h| h.trim_end_matches('/').rsplit('/').next())
        .filter_map(|v| Some((parse_version(v)?, v.to_owned())))
        .max()
        .map(|(_, v)| v)
}

/// `1.22.2` → `[1, 22, 2]`; anything else → `None`.
pub fn parse_version(v: &str) -> Option<Vec<u32>> {
    let parts: Option<Vec<u32>> = v.split('.').map(|p| p.parse().ok()).collect();
    parts.filter(|p| p.len() >= 2)
}

fn sha256_file(path: &Path) -> io::Result<String> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = io::Read::read(&mut f, &mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(unix)]
fn set_executable(p: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(p)?.permissions();
    perm.set_mode(perm.mode() | 0o755);
    std::fs::set_permissions(p, perm)
}

#[cfg(not(unix))]
fn set_executable(_: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests;
