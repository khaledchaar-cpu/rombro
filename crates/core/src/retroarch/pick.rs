//! Which core runs a system: the user's pick, else the recommendation (derived from
//! Batocera's x86_64 defaults, `recommended.tsv`), else the most specialised core.

use super::info::{Core, core_for};
use std::collections::BTreeMap;

const TABLE: &str = include_str!("recommended.tsv");

/// The recommended core id (e.g. `snes9x`) for a RetroArch system.
pub fn recommended(system: &str) -> Option<&'static str> {
    TABLE
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t'))
        .find(|(s, _)| *s == system)
        .map(|(_, c)| c.trim())
}

/// The core wanted for `system`: the pick in `picks` (system → core id), else the
/// recommendation, if its info file is known.
pub fn wanted<'a>(
    cores: &'a [Core],
    system: &str,
    picks: &BTreeMap<String, String>,
) -> Option<&'a Core> {
    let id = picks
        .get(system)
        .map(String::as_str)
        .or_else(|| recommended(system))?;
    cores.iter().find(|c| c.id == id)
}

/// The core the playlist of `system` gets. Without `install` only installed cores count:
/// the wanted one if installed, else the most specialised installed one. With `install`
/// the wanted core (or the most specialised known one) is returned even if missing.
pub fn resolve<'a>(
    cores: &'a [Core],
    system: &str,
    picks: &BTreeMap<String, String>,
    install: bool,
) -> Option<&'a Core> {
    match wanted(cores, system, picks) {
        Some(c) if c.installed || install => Some(c),
        _ => core_for(cores, system).or_else(|| {
            install
                .then(|| {
                    cores
                        .iter()
                        .filter(|c| c.databases.iter().any(|d| d == system))
                        .min_by_key(|c| c.databases.len())
                })
                .flatten()
        }),
    }
}

/// The cores to offer for `system`: the recommended first, then installed ones, then by id.
pub fn options<'a>(cores: &'a [Core], system: &str) -> Vec<&'a Core> {
    let rec = recommended(system);
    let mut out: Vec<&Core> = cores
        .iter()
        .filter(|c| c.databases.iter().any(|d| d == system) || Some(c.id.as_str()) == rec)
        .collect();
    out.sort_by_key(|c| (Some(c.id.as_str()) != rec, !c.installed, c.id.clone()));
    out
}

/// Systems of the library a core can be chosen for: its top-level system folders
/// (rombro's own `_…` folders left out), sorted.
pub fn library_systems(library: &std::path::Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(library)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| !n.starts_with('_') && !n.starts_with('.'))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core(id: &str, installed: bool, dbs: &[&str]) -> Core {
        Core {
            path: format!("/c/{id}_libretro.so").into(),
            id: id.into(),
            installed,
            name: id.into(),
            databases: dbs.iter().map(|s| (*s).into()).collect(),
        }
    }

    const SNES: &str = "Nintendo - Super Nintendo Entertainment System";

    #[test]
    fn table_has_known_systems() {
        assert_eq!(recommended(SNES), Some("snes9x"));
        assert_eq!(recommended("Sony - PlayStation"), Some("pcsx_rearmed"));
        assert_eq!(recommended("MAME 2003-Plus"), None);
    }

    #[test]
    fn resolves_pick_recommendation_and_fallback() {
        let cores = [
            core("bsnes", true, &[SNES]),
            core("snes9x", false, &[SNES, "Nintendo - Sufami Turbo"]),
        ];
        let none = BTreeMap::new();
        // Recommendation missing: the installed core without install, snes9x with install.
        assert_eq!(resolve(&cores, SNES, &none, false).unwrap().id, "bsnes");
        assert_eq!(resolve(&cores, SNES, &none, true).unwrap().id, "snes9x");
        let picks = BTreeMap::from([(SNES.to_owned(), "bsnes".to_owned())]);
        assert_eq!(resolve(&cores, SNES, &picks, true).unwrap().id, "bsnes");
        assert!(resolve(&cores, "Sega - Saturn", &none, true).is_none());
        let ids: Vec<_> = options(&cores, SNES).iter().map(|c| &c.id).collect();
        assert_eq!(ids, ["snes9x", "bsnes"]);
    }
}
