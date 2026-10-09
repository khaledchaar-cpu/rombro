//! Per-OS default locations: app data/cache dirs and RetroArch's RDB folder.

use std::path::{Path, PathBuf};

pub mod legacy;

/// Folder name of the app's data and cache dirs.
pub const APP: &str = "romburak";

/// Environment override for the RDB directory.
pub const RDB_ENV: &str = "ROMBURAK_RDB_DIR";

/// Database file: `<data dir>/romburak/romburak.db`
/// (Linux `$XDG_DATA_HOME`, macOS `~/Library/Application Support`, Windows `%APPDATA%`).
pub fn database() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join(APP).join(format!("{APP}.db")))
}

/// Cache root: `<cache dir>/romburak`.
pub fn cache() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join(APP))
}

/// Candidate RetroArch RDB directories for this OS, most likely first.
pub fn rdb_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(d) = std::env::var_os(RDB_ENV) {
        out.push(PathBuf::from(d));
    }
    let home = dirs::home_dir();
    let rel = Path::new("database/rdb");
    if cfg!(target_os = "macos") {
        if let Some(h) = &home {
            out.push(h.join("Library/Application Support/RetroArch").join(rel));
        }
        out.push(PathBuf::from("/Applications/RetroArch.app/Contents/Resources").join(rel));
    } else if cfg!(windows) {
        if let Some(d) = dirs::config_dir() {
            out.push(d.join("RetroArch").join(rel));
        }
        for root in ["C:\\RetroArch-Win64", "C:\\RetroArch"] {
            out.push(PathBuf::from(root).join(rel));
        }
    } else {
        if let Some(d) = dirs::config_dir() {
            out.push(d.join("retroarch").join(rel));
        }
        if let Some(h) = &home {
            out.push(
                h.join(".var/app/org.libretro.RetroArch/config/retroarch")
                    .join(rel),
            );
            out.push(h.join("snap/retroarch/current/.config/retroarch").join(rel));
        }
        out.push(PathBuf::from("/usr/share/libretro").join(rel));
    }
    out
}

/// First candidate that contains at least one `.rdb` file.
pub fn rdb_dir() -> Option<PathBuf> {
    rdb_candidates().into_iter().find(|d| has_rdb(d))
}

fn has_rdb(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|rd| {
        rd.flatten()
            .any(|e| e.path().extension().is_some_and(|x| x == "rdb"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_dir_with_rdb() {
        let t = tempfile::tempdir().unwrap();
        assert!(!has_rdb(t.path()));
        std::fs::write(t.path().join("x.rdb"), b"").unwrap();
        assert!(has_rdb(t.path()));
        assert!(!has_rdb(&t.path().join("missing")));
    }

    #[test]
    fn candidates_are_absolute() {
        assert!(rdb_candidates().iter().all(|p| p.is_absolute()));
    }
}
