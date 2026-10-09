//! One-time move of the data and cache dirs from the app's former name (`rombro`).

use std::fs;
use std::io;
use std::path::Path;

const OLD: &str = "rombro";

/// Moves `<data dir>/rombro` and `<cache dir>/rombro` to the current names once.
/// Errors are reported but never stop the app; the old dirs then simply stay.
pub fn migrate() -> io::Result<()> {
    if let Some(d) = dirs::data_dir() {
        migrate_data(&d)?;
    }
    if let Some(c) = dirs::cache_dir() {
        migrate_dir(&c.join(OLD), &c.join(super::APP))?;
    }
    Ok(())
}

/// Renames the data dir and its database, then rewrites absolute paths in the managed
/// RetroArch config and playlists.
pub fn migrate_data(base: &Path) -> io::Result<()> {
    let (old, new) = (base.join(OLD), base.join(super::APP));
    if !migrate_dir(&old, &new)? {
        return Ok(());
    }
    for ext in ["", "-wal", "-shm"] {
        let from = new.join(format!("{OLD}.db{ext}"));
        if from.exists() {
            fs::rename(&from, new.join(format!("{}.db{ext}", super::APP)))?;
        }
    }
    let (from, to) = (old.to_string_lossy(), new.to_string_lossy());
    let ra = new.join("retroarch");
    let mut files = vec![ra.join("retroarch.cfg")];
    files.extend(
        walkdir::WalkDir::new(ra.join("playlists"))
            .into_iter()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "lpl"))
            .map(|e| e.into_path()),
    );
    for f in files {
        let Ok(text) = fs::read_to_string(&f) else {
            continue;
        };
        if text.contains(from.as_ref()) {
            fs::write(&f, text.replace(from.as_ref(), to.as_ref()))?;
        }
    }
    Ok(())
}

/// `true` if `old` was moved to `new` (only when `new` does not exist yet).
fn migrate_dir(old: &Path, new: &Path) -> io::Result<bool> {
    if !old.is_dir() || new.exists() {
        return Ok(false);
    }
    fs::rename(old, new)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_data_dir_db_and_rewrites_retroarch_paths() {
        let tmp = tempfile::TempDir::new().unwrap();
        let old = tmp.path().join(OLD);
        fs::create_dir_all(old.join("retroarch/playlists/builtin")).unwrap();
        fs::write(old.join(format!("{OLD}.db")), "db").unwrap();
        let line = format!(
            "libretro_directory = \"{}/retroarch/cores\"\n",
            old.display()
        );
        fs::write(old.join("retroarch/retroarch.cfg"), &line).unwrap();
        fs::write(
            old.join("retroarch/playlists/builtin/content_history.lpl"),
            &line,
        )
        .unwrap();

        migrate_data(tmp.path()).unwrap();
        let new = tmp.path().join(super::super::APP);
        assert!(!old.exists());
        assert_eq!(fs::read_to_string(new.join("romburak.db")).unwrap(), "db");
        let want = line.replace(&*old.to_string_lossy(), &new.to_string_lossy());
        for f in ["retroarch.cfg", "playlists/builtin/content_history.lpl"] {
            assert_eq!(
                fs::read_to_string(new.join("retroarch").join(f)).unwrap(),
                want
            );
        }
        // a second run leaves everything alone
        migrate_data(tmp.path()).unwrap();
        assert!(new.join("romburak.db").exists());
    }
}
