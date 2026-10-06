//! Core info files (`*_libretro.info`): which databases (systems) a core runs, and whether
//! the core itself is installed.

use std::path::{Path, PathBuf};

/// An installed core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Core {
    /// Library file, e.g. `…/cores/snes9x_libretro.so`.
    pub path: PathBuf,
    /// Display name RetroArch stores in playlists (`corename`).
    pub name: String,
    /// Systems (RDB names) the core runs.
    pub databases: Vec<String>,
}

/// `key = "value"` pairs of a RetroArch config or info file.
pub fn kv(text: &str) -> impl Iterator<Item = (&str, &str)> {
    text.lines().filter_map(|l| {
        let (k, v) = l.split_once('=')?;
        Some((k.trim(), v.trim().trim_matches('"')))
    })
}

/// Cores with an info file in `info` and their library in `cores`, sorted by file name.
pub fn installed(info: &Path, cores: &Path) -> Vec<Core> {
    let ext = std::env::consts::DLL_EXTENSION;
    let Ok(dir) = std::fs::read_dir(info) else {
        return Vec::new();
    };
    let mut out: Vec<Core> = dir
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "info"))
        .filter_map(|p| {
            let lib = cores.join(p.file_stem()?).with_extension(ext);
            if !lib.is_file() {
                return None;
            }
            let text = std::fs::read_to_string(&p).ok()?;
            Some(parse(&text, lib))
        })
        .collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn parse(text: &str, path: PathBuf) -> Core {
    let mut core = Core {
        name: path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path,
        databases: Vec::new(),
    };
    for (k, v) in kv(text) {
        match k {
            "corename" => core.name = v.to_owned(),
            "database" => core.databases = v.split('|').map(str::to_owned).collect(),
            _ => {}
        }
    }
    core
}

/// The core for `system`: one that runs only few systems wins (specialised cores, and the
/// exact arcade core for `MAME 2003-Plus` etc.), then the file name.
pub fn core_for<'a>(cores: &'a [Core], system: &str) -> Option<&'a Core> {
    cores
        .iter()
        .filter(|c| c.databases.iter().any(|d| d == system))
        .min_by_key(|c| c.databases.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_info_and_picks_specialised_core() {
        let snes = parse(
            "corename = \"Snes9x\"\ndatabase = \"Nintendo - SNES|Nintendo - Sufami Turbo\"\n",
            "/c/snes9x_libretro.so".into(),
        );
        assert_eq!(snes.name, "Snes9x");
        assert_eq!(snes.databases.len(), 2);
        let multi = parse(
            "corename = \"Multi\"\ndatabase = \"A|B|Nintendo - SNES\"\n",
            "/c/multi.so".into(),
        );
        let cores = [multi, snes];
        assert_eq!(core_for(&cores, "Nintendo - SNES").unwrap().name, "Snes9x");
        assert!(core_for(&cores, "Sega - Saturn").is_none());
    }
}
