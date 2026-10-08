//! Unpacking the downloaded build: `.7z` (Linux, Windows) or `.dmg` (macOS).

use super::Target;
use std::io;
use std::path::Path;

/// Unpacks the downloaded build into `dest`, reporting (bytes done, total).
pub(super) fn unpack(
    target: Target,
    archive: &Path,
    dest: &Path,
    progress: &dyn Fn(u64, u64),
) -> io::Result<()> {
    match target {
        Target::MacUniversal => unpack_dmg(archive, dest),
        _ => unpack_7z(archive, dest, progress),
    }
}

fn unpack_7z(archive: &Path, dest: &Path, progress: &dyn Fn(u64, u64)) -> io::Result<()> {
    use sevenz_rust2::{ArchiveReader, Password};
    let total: u64 = ArchiveReader::open(archive, Password::empty())
        .map_err(io::Error::other)?
        .archive()
        .files
        .iter()
        .map(|f| f.size)
        .sum();
    let (mut done, mut shown) = (0u64, 0u64);
    progress(0, total);
    sevenz_rust2::decompress_file_with_extract_fn(archive, dest, |entry, reader, path| {
        let ok = sevenz_rust2::default_entry_extract_fn(entry, reader, path)?;
        done += entry.size;
        if done - shown >= 4 << 20 || done == total {
            shown = done;
            progress(done, total);
        }
        Ok(ok)
    })
    .map_err(io::Error::other)
}

/// Copies `RetroArch.app` out of the disk image (`hdiutil`, macOS only).
fn unpack_dmg(dmg: &Path, dest: &Path) -> io::Result<()> {
    use std::process::Command;
    let mount = dest.join(".mnt");
    let ok = |s: io::Result<std::process::ExitStatus>, what: &str| match s {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(io::Error::other(format!("{what} failed: {s}"))),
        Err(e) => Err(io::Error::other(format!("{what}: {e}"))),
    };
    ok(
        Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
            .arg(&mount)
            .arg(dmg)
            .status(),
        "hdiutil attach",
    )?;
    let copied = ok(
        Command::new("cp")
            .arg("-R")
            .arg(mount.join("RetroArch.app"))
            .arg(dest)
            .status(),
        "copy RetroArch.app",
    );
    let detached = ok(
        Command::new("hdiutil").arg("detach").arg(&mount).status(),
        "hdiutil detach",
    );
    copied.and(detached)?;
    let _ = std::fs::remove_dir(&mount);
    Ok(())
}
