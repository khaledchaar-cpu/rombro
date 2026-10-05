//! Arcade romsets (FBNeo/MAME): identified as whole archives, kept under their short name.

/// Arcade systems in placement priority (an archive matching several lands in the first).
const PRIORITY: [&str; 9] = [
    "FBNeo - Arcade Games",
    "MAME",
    "MAME 2016",
    "MAME 2015",
    "MAME 2010",
    "MAME 2003-Plus",
    "MAME 2003",
    "MAME 2000",
    "HBMAME",
];

/// Placement rank of a system (lower wins); non-arcade systems rank last.
pub fn rank(system: &str) -> usize {
    PRIORITY
        .iter()
        .position(|s| *s == system)
        .unwrap_or(PRIORITY.len())
}

/// BIOS and device sets that games load from; matched by short name or description.
const BIOS_SETS: [&str; 18] = [
    "neogeo", "pgm", "skns", "bubsys", "cchip", "decocass", "isgsm", "midssio", "namcoc69",
    "namcoc70", "namcoc75", "nmk004", "ym2608", "stvbios", "naomi", "naomi2", "awbios", "hng64",
];

/// Whether an entry (short `rom_name` like `neogeo.zip`, long `name`) is a BIOS/device set.
pub fn is_bios(rom_name: Option<&str>, name: &str) -> bool {
    let short = rom_name
        .and_then(|r| r.rsplit_once('.').map(|(s, _)| s))
        .unwrap_or_default();
    if BIOS_SETS.contains(&short) {
        return true;
    }
    let n = name.to_lowercase();
    n.ends_with("system bios")
        || n.ends_with("(bios)")
        || n.contains("internal rom")
        || n.contains("internal prom")
}

/// Library-relative folder where RetroArch's core for `system` finds BIOS sets.
/// FBNeo reads `system/fbneo/`; the MAME cores only search the romset folder itself.
pub fn bios_dir(system: &str) -> std::path::PathBuf {
    match system {
        "FBNeo - Arcade Games" => std::path::Path::new(crate::plan::BIOS_DIR).join("fbneo"),
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn fbneo_beats_newest_mame_beats_older() {
        assert!(rank("FBNeo - Arcade Games") < rank("MAME"));
        assert!(rank("MAME") < rank("MAME 2016"));
        assert!(rank("MAME 2003-Plus") < rank("Nintendo - SNES"));
    }

    #[test]
    fn detects_bios_but_not_bios_named_games() {
        assert!(is_bios(Some("neogeo.zip"), "Neo Geo"));
        assert!(is_bios(Some("x.zip"), "Super Kaneko Nova System BIOS"));
        assert!(is_bios(Some("x.zip"), "Namco C69 (M37702) (Bios)"));
        assert!(!is_bios(
            Some("sfiiin.zip"),
            "Street Fighter III: New Generation (Asia 970204, NO CD, BIOS set 1)"
        ));
        assert!(!is_bios(Some("burningf.zip"), "Burning Fight"));
    }

    #[test]
    fn bios_lands_where_each_core_looks() {
        assert_eq!(bios_dir("FBNeo - Arcade Games"), Path::new("_bios/fbneo"));
        assert_eq!(bios_dir("MAME 2003-Plus"), Path::new("MAME 2003-Plus"));
    }
}
