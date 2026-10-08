//! Downloading arcade DATs from the libretro core repositories (and MAME releases).
//! Each source is versioned (commit or release tag); unchanged versions are not downloaded.

use crate::{Result, Store};
use rombro_core::arcade::dat;
use std::io::{self, Cursor, Read};

/// Where a core's DAT lives.
enum Source {
    /// File in a GitHub repository (latest commit touching it).
    Repo(&'static str, &'static str),
    /// `mame*lx.zip` asset of the latest MAME release.
    MameRelease,
}

/// DAT sources per arcade core (HBMAME publishes none → its sets stay unchecked).
const SOURCES: [(&str, Source); 8] = [
    (
        "FBNeo - Arcade Games",
        Source::Repo(
            "libretro/FBNeo",
            "dats/FinalBurn Neo (ClrMame Pro XML, Arcade only).dat",
        ),
    ),
    ("MAME", Source::MameRelease),
    (
        "MAME 2016",
        Source::Repo(
            "libretro/mame2016-libretro",
            "metadata/MAME 0.174 Arcade XML DAT.zip",
        ),
    ),
    (
        "MAME 2015",
        Source::Repo("libretro/mame2015-libretro", "metadata/mame2015-xml.zip"),
    ),
    (
        "MAME 2010",
        Source::Repo("libretro/mame2010-libretro", "metadata/mame2010.xml"),
    ),
    (
        "MAME 2003-Plus",
        Source::Repo(
            "libretro/mame2003-plus-libretro",
            "metadata/mame2003-plus.xml",
        ),
    ),
    (
        "MAME 2003",
        Source::Repo("libretro/mame2003-libretro", "metadata/mame2003.xml"),
    ),
    (
        "MAME 2000",
        Source::Repo("libretro/mame2000-libretro", "metadata/MAME 0.37b5 XML.dat"),
    ),
];

/// Outcome of [`Store::sync_dats`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DatSyncReport {
    /// (core, version) newly loaded.
    pub updated: Vec<(String, String)>,
    pub unchanged: usize,
    /// Cores whose DAT could not be refreshed (offline, …); the previous one stays in use.
    pub warnings: Vec<String>,
}

/// HTTP GET returning the body (injected for tests).
pub type Fetch<'a> = &'a dyn Fn(&str) -> io::Result<Vec<u8>>;

impl Store {
    /// Downloads every core's newest DAT over HTTPS.
    pub fn sync_dats(&mut self, now: i64) -> Result<DatSyncReport> {
        self.sync_dats_progress(now, &|_, _| {})
    }

    /// [`Self::sync_dats`], reporting (done, total) DAT sources to `progress`.
    pub fn sync_dats_progress(
        &mut self,
        now: i64,
        progress: &dyn Fn(usize, usize),
    ) -> Result<DatSyncReport> {
        self.sync_dats_inner(&http_get, now, progress)
    }

    pub fn sync_dats_with(&mut self, fetch: Fetch, now: i64) -> Result<DatSyncReport> {
        self.sync_dats_inner(fetch, now, &|_, _| {})
    }

    fn sync_dats_inner(
        &mut self,
        fetch: Fetch,
        now: i64,
        progress: &dyn Fn(usize, usize),
    ) -> Result<DatSyncReport> {
        let known = self.dats()?;
        let mut report = DatSyncReport::default();
        progress(0, SOURCES.len());
        for (i, (system, source)) in SOURCES.iter().enumerate() {
            let have = known
                .iter()
                .find(|d| d.system == *system)
                .map(|d| d.version.as_str());
            let res = latest(fetch, source).and_then(|(version, url)| {
                if have == Some(version.as_str()) {
                    return Ok(None);
                }
                let sets = parse(&fetch(&url)?, &url)?;
                Ok(Some((version, sets)))
            });
            match res {
                Ok(None) => report.unchanged += 1,
                Ok(Some((version, sets))) => {
                    self.import_dat(system, &version, now, &sets)?;
                    report.updated.push((system.to_string(), version));
                }
                Err(e) => report.warnings.push(match have {
                    Some(v) => format!("{system}: {e} (keeping {v})"),
                    None => format!("{system}: {e} (no DAT, sets unchecked)"),
                }),
            }
            progress(i + 1, SOURCES.len());
        }
        Ok(report)
    }
}

/// Newest version of a source and the URL of its file.
fn latest(fetch: Fetch, source: &Source) -> io::Result<(String, String)> {
    let json = |url: &str| -> io::Result<serde_json::Value> {
        serde_json::from_slice(&fetch(url)?).map_err(io::Error::other)
    };
    match source {
        Source::Repo(repo, path) => {
            let enc = path.replace(' ', "%20");
            let v = json(&format!(
                "https://api.github.com/repos/{repo}/commits?path={enc}&per_page=1"
            ))?;
            let sha = v[0]["sha"].as_str().ok_or_else(|| bad("no commit"))?;
            Ok((
                sha[..12.min(sha.len())].to_owned(),
                format!("https://raw.githubusercontent.com/{repo}/{sha}/{enc}"),
            ))
        }
        Source::MameRelease => {
            let v = json("https://api.github.com/repos/mamedev/mame/releases/latest")?;
            let tag = v["tag_name"].as_str().ok_or_else(|| bad("no release"))?;
            let url = v["assets"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|a| a["name"].as_str().is_some_and(|n| n.ends_with("lx.zip")))
                .and_then(|a| a["browser_download_url"].as_str())
                .ok_or_else(|| bad("no listxml asset"))?;
            Ok((tag.to_owned(), url.to_owned()))
        }
    }
}

/// Parses a DAT, unpacking the first XML/DAT member if it is zipped.
fn parse(body: &[u8], url: &str) -> io::Result<Vec<dat::DatSet>> {
    let parsed = if url.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(Cursor::new(body)).map_err(io::Error::other)?;
        let name = zip
            .file_names()
            .find(|n| n.ends_with(".xml") || n.ends_with(".dat"))
            .map(str::to_owned)
            .ok_or_else(|| bad("no DAT in zip"))?;
        let member = zip.by_name(&name).map_err(io::Error::other)?;
        dat::parse(io::BufReader::new(member))
    } else {
        dat::parse(body)
    };
    parsed.map_err(io::Error::other)
}

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

/// GET `url` (up to 512 MiB) – DATs, and RetroArch cores for the export.
pub fn http_get(url: &str) -> io::Result<Vec<u8>> {
    let mut resp = ureq::get(url)
        .header("User-Agent", "rombro")
        .call()
        .map_err(io::Error::other)?;
    let mut buf = Vec::new();
    resp.body_mut()
        .with_config()
        .limit(512 << 20)
        .reader()
        .read_to_end(&mut buf)?;
    Ok(buf)
}

/// Streams `url` into `to`, reporting (bytes done, total from `Content-Length`).
pub fn http_download(
    url: &str,
    to: &std::path::Path,
    progress: &dyn Fn(u64, Option<u64>),
) -> io::Result<()> {
    let mut resp = ureq::get(url)
        .header("User-Agent", "rombro")
        .call()
        .map_err(io::Error::other)?;
    let total = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok()?.parse().ok());
    let mut reader = resp.body_mut().with_config().limit(2 << 30).reader();
    let mut out = io::BufWriter::new(std::fs::File::create(to)?);
    let mut buf = vec![0u8; 1 << 16];
    let (mut done, mut shown) = (0u64, 0u64);
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        io::Write::write_all(&mut out, &buf[..n])?;
        done += n as u64;
        if done - shown >= 1 << 20 {
            shown = done;
            progress(done, total);
        }
    }
    io::Write::flush(&mut out)?;
    progress(done, total);
    if total.is_some_and(|t| t != done) {
        return Err(io::Error::other(format!(
            "incomplete download ({done} bytes)"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloads_new_versions_and_keeps_old_ones_offline() {
        let mut s = Store::open_in_memory().unwrap();
        let online = |url: &str| -> io::Result<Vec<u8>> {
            Ok(if url.contains("/commits?") {
                br#"[{"sha":"abcdef0123456789"}]"#.to_vec()
            } else if url.contains("releases/latest") {
                br#"{"tag_name":"mame0289","assets":[{"name":"x.exe"},{"name":"mame0289lx.zip","browser_download_url":"https://x/lx.zip"}]}"#.to_vec()
            } else if url.ends_with(".zip") {
                let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
                w.start_file("m.xml", zip::write::SimpleFileOptions::default())
                    .map_err(io::Error::other)?;
                io::Write::write_all(&mut w, br#"<mame><machine name="z"/></mame>"#)?;
                w.finish().map_err(io::Error::other)?.into_inner()
            } else {
                br#"<datafile><game name="g"><rom name="r" size="1" crc="1"/></game></datafile>"#
                    .to_vec()
            })
        };
        let r = s.sync_dats_with(&online, 5).unwrap();
        assert_eq!(r.updated.len(), 8, "{r:?}");
        assert!(r.warnings.is_empty());
        assert!(s.dat_set("MAME", "z").unwrap().is_some());
        assert_eq!(s.dat_set("MAME 2003", "g").unwrap().unwrap().roms.len(), 1);

        let r = s.sync_dats_with(&online, 6).unwrap();
        assert_eq!((r.updated.len(), r.unchanged), (0, 8));

        let offline = |_: &str| -> io::Result<Vec<u8>> { Err(io::Error::other("offline")) };
        let r = s.sync_dats_with(&offline, 7).unwrap();
        assert_eq!(r.warnings.len(), 8);
        assert!(r.warnings[0].contains("keeping abcdef012345"));
        assert_eq!(s.dats().unwrap().len(), 8);
    }
}
