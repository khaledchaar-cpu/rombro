//! RetroArch databases from the libretro buildbot (`database-rdb.zip`, rebuilt daily) into the
//! managed RetroArch's `database/rdb`, which romburak and that RetroArch share.

use romburak_core::retroarch::managed::{FRONTEND_ASSETS_URL, Managed};
use std::io;
use std::path::PathBuf;

const STAMP: &str = ".last-modified";

fn last_modified(url: &str) -> Option<String> {
    let resp = ureq::head(url)
        .header("User-Agent", "romburak")
        .call()
        .ok()?;
    Some(
        resp.headers()
            .get("last-modified")?
            .to_str()
            .ok()?
            .to_owned(),
    )
}

/// Downloads the databases unless the local copy matches the server's; returns the folder
/// and whether it was updated. Offline with a previous download: that folder, not updated.
pub fn update_rdbs(
    m: &Managed,
    progress: &dyn Fn(u64, Option<u64>),
) -> io::Result<(PathBuf, bool)> {
    let dir = m.database_dir();
    let url = format!("{FRONTEND_ASSETS_URL}/database-rdb.zip");
    let have = std::fs::read_to_string(dir.join(STAMP)).ok();
    let remote = last_modified(&url);
    match (&remote, &have) {
        (Some(r), Some(h)) if r == h => return Ok((dir, false)),
        (None, Some(_)) => return Ok((dir, false)),
        (None, None) => return Err(io::Error::other("database download: server not reachable")),
        _ => {}
    }
    let parent = dir
        .parent()
        .ok_or_else(|| io::Error::other("bad database folder"))?;
    std::fs::create_dir_all(parent)?;
    let zip = parent.join("database-rdb.zip.part");
    crate::http_download(&url, &zip, progress)?;
    let tmp = parent.join("rdb.part");
    let _ = std::fs::remove_dir_all(&tmp);
    zip::ZipArchive::new(std::fs::File::open(&zip)?)
        .and_then(|mut z| z.extract(&tmp))
        .map_err(io::Error::other)?;
    let _ = std::fs::remove_file(&zip);
    if let Some(r) = &remote {
        std::fs::write(tmp.join(STAMP), r)?;
    }
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&tmp, &dir)?;
    Ok((dir, true))
}
