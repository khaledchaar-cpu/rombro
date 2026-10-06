//! Extracting single members from ZIP and 7z archives.

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;

/// Writes `member` of `archive` to the new file `to` (fails if `to` exists).
/// A partially written file is removed on error.
pub fn extract(archive: &Path, member: &str, to: &Path) -> io::Result<()> {
    let mut out = BufWriter::new(File::create_new(to)?);
    let res = write_member(archive, member, &mut out).and_then(|()| out.flush());
    drop(out);
    if res.is_err() {
        let _ = fs::remove_file(to);
    }
    res
}

/// Reads a (small) member into memory, e.g. a disc sheet.
pub fn read_member(archive: &Path, member: &str) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    write_member(archive, member, &mut buf)?;
    Ok(buf)
}

/// Names and CRCs of a zip's files, read from its central directory (nothing is unpacked).
pub fn members(zip: &Path) -> io::Result<Vec<(String, u32)>> {
    let mut zip =
        zip::ZipArchive::new(BufReader::new(File::open(zip)?)).map_err(io::Error::other)?;
    let mut out = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let f = zip.by_index_raw(i).map_err(io::Error::other)?;
        if f.is_file() {
            let name = f.name();
            out.push((
                name.rsplit('/').next().unwrap_or(name).to_owned(),
                f.crc32(),
            ));
        }
    }
    Ok(out)
}

fn write_member(archive: &Path, member: &str, out: &mut impl Write) -> io::Result<()> {
    let ext = archive
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "zip" => {
            let mut zip = zip::ZipArchive::new(BufReader::new(File::open(archive)?))
                .map_err(io::Error::other)?;
            let mut f = zip.by_name(member).map_err(io::Error::other)?;
            io::copy(&mut f, out).map(|_| ())
        }
        "7z" => {
            let mut ar =
                sevenz_rust2::ArchiveReader::open(archive, sevenz_rust2::Password::empty())
                    .map_err(io::Error::other)?;
            let mut found = false;
            ar.for_each_entries(|e, r| {
                if e.name() != member {
                    return Ok(true);
                }
                io::copy(r, out)?;
                found = true;
                Ok(false)
            })
            .map_err(io::Error::other)?;
            if found {
                Ok(())
            } else {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("{member} not in {}", archive.display()),
                ))
            }
        }
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("not an archive: {}", archive.display()),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn extracts_zip_member_and_refuses_overwrite() {
        let tmp = TempDir::new().unwrap();
        let zip_path = tmp.path().join("set.zip");
        let mut w = zip::ZipWriter::new(File::create(&zip_path).unwrap());
        for (name, data) in [("a.nes", b"aaaa"), ("b.nes", b"bbbb")] {
            w.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
        let to = tmp.path().join("b.nes");
        extract(&zip_path, "b.nes", &to).unwrap();
        assert_eq!(fs::read(&to).unwrap(), b"bbbb");
        assert!(extract(&zip_path, "a.nes", &to).is_err());
        assert_eq!(fs::read(&to).unwrap(), b"bbbb");
        let missing = tmp.path().join("c.nes");
        assert!(extract(&zip_path, "c.nes", &missing).is_err());
        assert!(!missing.exists());
    }
}
