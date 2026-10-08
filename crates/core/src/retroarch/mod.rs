//! RetroArch integration: its folders (from `retroarch.cfg`), installed cores (`*.info`),
//! required firmware (libretro `System.dat`) and the export of playlists and BIOS files.

pub mod assets;
pub mod export;
pub mod firmware;
pub mod info;
pub mod pick;
pub mod scummvm;

use std::path::{Path, PathBuf};

/// The RetroArch folders rombro writes to or reads from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    pub playlists: PathBuf,
    pub system: PathBuf,
    pub cores: PathBuf,
    pub info: PathBuf,
    /// Where RetroArch's core updater downloads cores (`<url>/<core>_libretro.so.zip`).
    pub buildbot: Option<String>,
}

impl Dirs {
    /// Folders from the first `retroarch.cfg` found; settings it lacks default to its folder.
    pub fn detect() -> Option<Self> {
        cfg_candidates()
            .into_iter()
            .find(|p| p.is_file())
            .and_then(|p| Self::from_cfg(&p).ok())
    }

    /// Reads the folder settings of `cfg`.
    pub fn from_cfg(cfg: &Path) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(cfg)?;
        let base = cfg.parent().unwrap_or(Path::new("."));
        Ok(Self::parse(&text, base))
    }

    fn parse(text: &str, base: &Path) -> Self {
        let get = |key: &str, default: &str| {
            let v = info::kv(text)
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v)
                .filter(|v| !v.is_empty() && *v != "default");
            match v {
                Some(v) => expand(v, base),
                None => base.join(default),
            }
        };
        let cores = get("libretro_directory", "cores");
        Self {
            playlists: get("playlist_directory", "playlists"),
            system: get("system_directory", "system"),
            info: get("libretro_info_path", &cores.to_string_lossy()),
            cores,
            buildbot: info::kv(text)
                .find(|(k, _)| *k == "core_updater_buildbot_cores_url")
                .map(|(_, v)| v.trim_end_matches('/').to_owned())
                .filter(|v| !v.is_empty()),
        }
    }
}

/// `~/…` → home, `:/…` → relative to the config folder (RetroArch's notation).
fn expand(v: &str, base: &Path) -> PathBuf {
    if let Some(rest) = v.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    if let Some(rest) = v.strip_prefix(":/").or_else(|| v.strip_prefix(":\\")) {
        return base.join(rest);
    }
    PathBuf::from(v)
}

/// Candidate `retroarch.cfg` locations for this OS, most likely first.
pub fn cfg_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let home = dirs::home_dir();
    if cfg!(target_os = "macos") {
        if let Some(h) = &home {
            out.push(h.join("Library/Application Support/RetroArch/config/retroarch.cfg"));
        }
    } else if cfg!(windows) {
        if let Some(d) = dirs::config_dir() {
            out.push(d.join("RetroArch/retroarch.cfg"));
        }
        for root in ["C:\\RetroArch-Win64", "C:\\RetroArch"] {
            out.push(PathBuf::from(root).join("retroarch.cfg"));
        }
    } else {
        if let Some(d) = dirs::config_dir() {
            out.push(d.join("retroarch/retroarch.cfg"));
        }
        if let Some(h) = &home {
            out.push(h.join(".var/app/org.libretro.RetroArch/config/retroarch/retroarch.cfg"));
            out.push(h.join("snap/retroarch/current/.config/retroarch/retroarch.cfg"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_dirs_with_defaults_and_relative_paths() {
        let cfg = "playlist_directory = \":/pl\"\nsystem_directory = \"default\"\n\
                   libretro_directory = \"/usr/lib/libretro\"\n";
        let d = Dirs::parse(cfg, Path::new("/ra"));
        assert_eq!(d.playlists, Path::new("/ra/pl"));
        assert_eq!(d.system, Path::new("/ra/system"));
        assert_eq!(d.cores, Path::new("/usr/lib/libretro"));
        assert_eq!(d.info, Path::new("/usr/lib/libretro"));
    }
}
