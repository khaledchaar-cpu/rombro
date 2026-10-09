//! The `retroarch.cfg` of the managed RetroArch. romburak owns the folder settings (and the display
//! mode and achievement login, if chosen); every other
//! line (the user's menu changes, saved on exit) is kept.

use super::Managed;
use std::io;
use std::path::Path;

impl Managed {
    /// Folder settings romburak writes, for the version in use and `library` (BIOS from `_bios`).
    pub fn managed_keys(
        &self,
        version: &str,
        library: Option<&Path>,
    ) -> Vec<(&'static str, String)> {
        let dir = |p: &Path| p.to_string_lossy().into_owned();
        let sub = |name: &str| dir(&self.root.join(name));
        let system = library.map_or_else(|| sub("system"), |l| dir(&l.join("_bios")));
        let mut keys = vec![
            ("libretro_directory", sub("cores")),
            ("libretro_info_path", sub("info")),
            ("system_directory", system),
            ("savefile_directory", sub("saves")),
            ("savestate_directory", dir(&self.states_dir())),
            ("screenshot_directory", sub("screenshots")),
            ("playlist_directory", sub("playlists")),
            ("core_assets_directory", sub("downloads")),
            ("sort_savefiles_enable", "true".into()),
            ("sort_savestates_enable", "true".into()),
            // romburak sends commands (live shader switch) over the child's stdin
            ("stdin_cmd_enable", "true".into()),
        ];
        if self.target == super::Target::LinuxX64 {
            // the default `gl` driver cannot run slang shaders (also when switched live)
            keys.push(("video_driver", "glcore".into()));
        }
        if let Some(d) = self.display {
            keys.extend(d.keys());
        }
        if let Some(c) = &self.cheevos {
            keys.extend(c.keys());
        }
        if let Some(a) = self.target.assets() {
            keys.push(("assets_directory", dir(&self.version_dir(version).join(a))));
        }
        keys
    }

    /// Writes the config (atomically) with the managed keys set and creates their folders.
    pub fn write_config(&self, library: Option<&Path>) -> io::Result<()> {
        let version = self
            .current()
            .ok_or_else(|| io::Error::other("RetroArch is not installed"))?;
        let old = std::fs::read_to_string(self.cfg()).unwrap_or_default();
        self.adopt_config_dir(&old)?;
        let mut keys = self.managed_keys(&version, library);
        keys.extend(self.video_keys(&version)?);
        for (k, v) in &keys {
            // The library's `_bios` is the planner's to create.
            let in_library = *k == "system_directory" && library.is_some();
            if (k.ends_with("_directory") || k.ends_with("_path")) && !in_library {
                std::fs::create_dir_all(v)?;
            }
        }
        let tmp = self.cfg().with_extension("cfg.tmp");
        std::fs::write(&tmp, merge(&old, &keys))?;
        std::fs::rename(tmp, self.cfg())
    }
}

/// `old` with `keys` replaced or appended (RetroArch syntax `key = "value"`).
pub fn merge(old: &str, keys: &[(&str, String)]) -> String {
    let key_of = |line: &str| {
        line.split('=')
            .next()
            .map(str::trim)
            .unwrap_or("")
            .to_owned()
    };
    let mut out: Vec<String> = old
        .lines()
        .filter(|l| !keys.iter().any(|(k, _)| key_of(l) == *k))
        .map(str::to_owned)
        .collect();
    out.extend(keys.iter().map(|(k, v)| format!("{k} = \"{v}\"")));
    out.join("\n") + "\n"
}
