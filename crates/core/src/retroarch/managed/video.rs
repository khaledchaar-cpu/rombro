//! Global picture settings of the managed RetroArch: shader preset and aspect ratio.
//! Stored as settings `ra_shader` (`off` or a preset path relative to `shaders_slang`) and
//! `ra_aspect`; unset leaves the choice to RetroArch's own menu.
//!
//! The preset is applied as RetroArch's global auto preset (`<root>/config/global.slangp`
//! with a `#reference`), shaders come from the RetroArch bundle or the buildbot package.

use super::{FetchFile, Managed};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shader {
    Off,
    /// Preset relative to `shaders_slang`, e.g. `crt/crt-royale.slangp`.
    Preset(String),
}

impl Shader {
    pub fn parse(s: &str) -> Self {
        match s {
            "" | "off" => Shader::Off,
            p => Shader::Preset(p.to_owned()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Shader::Off => "off",
            Shader::Preset(p) => p,
        }
    }
}

/// Aspect ratios offered (RetroArch `aspect_ratio_index` values of 1.22).
pub const ASPECTS: [(&str, u8); 5] = [
    ("core", 22),
    ("4:3", 0),
    ("16:9", 1),
    ("square", 21),
    ("full", 24),
];

pub fn aspect_index(name: &str) -> Option<u8> {
    ASPECTS.iter().find(|(n, _)| *n == name).map(|(_, i)| *i)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Video {
    pub shader: Option<Shader>,
    /// One of [`ASPECTS`].
    pub aspect: Option<String>,
}

const PACKAGES: [&str; 2] = ["shaders_slang", "autoconfig"];

impl Managed {
    /// Folder of a frontend package (`shaders_slang`, `autoconfig`): bundled with the
    /// RetroArch version, else downloaded into `<root>`.
    pub fn package_dir(&self, version: &str, name: &str) -> Option<PathBuf> {
        let sub = if name == "autoconfig" {
            "autoconfig"
        } else {
            "shaders"
        };
        let bundled = self.target.assets().map(|a| {
            let base = self.version_dir(version).join(a);
            let base = base.parent().unwrap_or(&base).join(sub);
            if sub == "shaders" {
                base.join(name)
            } else {
                base
            }
        });
        let own = self.downloaded_package(name);
        bundled
            .into_iter()
            .chain([own])
            .find(|d| d.read_dir().is_ok_and(|mut r| r.next().is_some()))
    }

    /// Folder `sub` next to the bundled assets (Linux/Windows builds ship shaders,
    /// autoconfig and databases there).
    pub fn bundled_dir(&self, version: &str, sub: &str) -> Option<PathBuf> {
        let assets = self.version_dir(version).join(self.target.assets()?);
        Some(assets.parent()?.join(sub))
    }

    fn downloaded_package(&self, name: &str) -> PathBuf {
        if name == "autoconfig" {
            self.root.join("autoconfig")
        } else {
            self.root.join("shaders").join(name)
        }
    }

    /// Downloads and unpacks a frontend package unless present; returns its folder.
    pub fn ensure_package(
        &self,
        name: &str,
        fetch: FetchFile,
        progress: &dyn Fn(u64, Option<u64>),
    ) -> io::Result<PathBuf> {
        if !PACKAGES.contains(&name) {
            return Err(io::Error::other("unknown package"));
        }
        let version = self
            .current()
            .ok_or_else(|| io::Error::other("RetroArch is not installed"))?;
        if let Some(d) = self.package_dir(&version, name) {
            return Ok(d);
        }
        let zip = self.root.join("downloads").join(format!("{name}.zip"));
        std::fs::create_dir_all(self.root.join("downloads"))?;
        let url = format!(
            "{}/{name}.zip",
            crate::retroarch::managed::FRONTEND_ASSETS_URL
        );
        fetch(&url, &zip, progress)?;
        let dest = self.downloaded_package(name);
        let tmp = dest.with_extension("part");
        let _ = std::fs::remove_dir_all(&tmp);
        zip::ZipArchive::new(std::fs::File::open(&zip)?)
            .and_then(|mut z| z.extract(&tmp))
            .map_err(io::Error::other)?;
        let _ = std::fs::remove_dir_all(&dest);
        std::fs::rename(&tmp, &dest)?;
        let _ = std::fs::remove_file(&zip);
        Ok(dest)
    }

    /// Shader presets (`.slangp`) relative to `shaders_slang`, sorted; empty if not present.
    pub fn shader_presets(&self) -> Vec<String> {
        let Some(dir) = self
            .current()
            .and_then(|v| self.package_dir(&v, "shaders_slang"))
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        walk(&dir, &dir, &mut out);
        out.sort();
        out
    }

    /// Before romburak owns `rgui_config_directory` (overrides, core options, global preset):
    /// copies the folder the config used so far (often the system RetroArch's) once.
    pub(super) fn adopt_config_dir(&self, old_cfg: &str) -> io::Result<()> {
        let dest = self.root.join("config");
        if dest.exists() {
            return Ok(());
        }
        let prev = crate::retroarch::info::kv(old_cfg)
            .find(|(k, _)| *k == "rgui_config_directory")
            .map(|(_, v)| PathBuf::from(v))
            .filter(|p| p.is_dir() && *p != dest);
        match prev {
            Some(src) => copy_dir(&src, &dest),
            None => Ok(()),
        }
    }

    /// Config keys for the picture settings; also writes or removes the global preset.
    pub(super) fn video_keys(&self, version: &str) -> io::Result<Vec<(&'static str, String)>> {
        let mut keys = Vec::new();
        let dir = |p: &Path| p.to_string_lossy().into_owned();
        let config = self.root.join("config");
        keys.push(("rgui_config_directory", dir(&config)));
        if let Some(d) = self.package_dir(version, "autoconfig") {
            keys.push(("joypad_autoconfig_dir", dir(&d)));
        }
        let slang = self.package_dir(version, "shaders_slang");
        if let Some(d) = slang.as_deref().and_then(Path::parent) {
            keys.push(("video_shader_dir", dir(d)));
        }
        let global = config.join("global.slangp");
        match (&self.video.shader, &slang) {
            (Some(Shader::Preset(p)), Some(s)) if s.join(p).is_file() => {
                std::fs::create_dir_all(&config)?;
                std::fs::write(&global, format!("#reference \"{}\"\n", s.join(p).display()))?;
                keys.push(("video_shader_enable", "true".into()));
                keys.push(("auto_shaders_enable", "true".into()));
            }
            (Some(_), _) => {
                let _ = std::fs::remove_file(&global);
                keys.push(("video_shader_enable", "false".into()));
            }
            (None, _) => {}
        }
        if let Some(i) = self.video.aspect.as_deref().and_then(aspect_index) {
            keys.push(("aspect_ratio_index", i.to_string()));
            keys.push(("video_aspect_ratio_auto", "false".into()));
        }
        Ok(keys)
    }
}

fn copy_dir(src: &Path, dest: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for e in std::fs::read_dir(src)?.flatten() {
        let (from, to) = (e.path(), dest.join(e.file_name()));
        if e.file_type()?.is_dir() {
            copy_dir(&from, &to)?;
        } else if from
            .extension()
            .is_none_or(|x| x != "slangp" && x != "glslp" && x != "cgp")
        {
            // shader presets of the old folder would apply in the managed RetroArch unasked
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(base, &p, out);
        } else if p.extension().is_some_and(|x| x == "slangp")
            && let Ok(rel) = p.strip_prefix(base)
        {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}
