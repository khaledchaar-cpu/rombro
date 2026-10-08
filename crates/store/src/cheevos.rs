//! RetroAchievements: games with achievements and their hashes per console (Web API
//! `API_GetGameList`), and the RA hash of library files (cached by size + mtime).

use crate::dat_sync::Fetch;
use crate::files::{key, prefix};
use crate::{Result, Store, http_get};
use rombro_core::cheevos::{self, Method};
use rusqlite::{OptionalExtension, params};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

const API: &str = "https://retroachievements.org/API/API_GetGameList.php";

/// A RetroAchievements game (only games with achievements are synced).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RaGame {
    pub id: u64,
    pub console: u32,
    pub title: String,
    pub achievements: u32,
    pub points: u32,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct RaSyncReport {
    pub consoles: usize,
    pub games: usize,
    pub hashes: usize,
    /// Consoles that could not be fetched: (id, error).
    pub failed: Vec<(u32, String)>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiGame {
    #[serde(rename = "ID")]
    id: u64,
    title: String,
    num_achievements: u32,
    #[serde(default)]
    points: u32,
    #[serde(default)]
    hashes: Vec<String>,
}

/// GET with a pause before each request and retries on HTTP 429 (the Web API rate-limits).
fn polite_get(url: &str) -> std::io::Result<Vec<u8>> {
    let mut wait = Duration::from_millis(1500);
    for _ in 0..4 {
        std::thread::sleep(wait);
        match http_get(url) {
            Err(e) if e.to_string().contains("429") => wait *= 3,
            r => return r,
        }
    }
    http_get(url)
}

fn game(r: &rusqlite::Row, at: usize) -> rusqlite::Result<RaGame> {
    Ok(RaGame {
        id: r.get(at)?,
        console: r.get(at + 1)?,
        title: r.get(at + 2)?,
        achievements: r.get(at + 3)?,
        points: r.get(at + 4)?,
    })
}

impl Store {
    /// Downloads the game lists of all consoles RomBro can hash (needs the user's Web API key).
    pub fn ra_sync(
        &mut self,
        api_key: &str,
        now: i64,
        progress: &dyn Fn(usize, usize),
    ) -> Result<RaSyncReport> {
        self.ra_sync_with(&polite_get, api_key, now, progress)
    }

    pub fn ra_sync_with(
        &mut self,
        fetch: Fetch,
        api_key: &str,
        now: i64,
        progress: &dyn Fn(usize, usize),
    ) -> Result<RaSyncReport> {
        let ids = cheevos::console_ids();
        let mut report = RaSyncReport::default();
        for (i, &id) in ids.iter().enumerate() {
            progress(i, ids.len());
            let url = format!("{API}?i={id}&f=1&h=1&y={api_key}");
            // errors never echo the URL: it carries the key
            let games: Vec<ApiGame> = match fetch(&url)
                .map_err(|e| e.to_string())
                .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))
            {
                Ok(g) => g,
                Err(e) => {
                    report.failed.push((id, e.replace(api_key, "***")));
                    continue;
                }
            };
            let tx = self.conn.transaction()?;
            tx.execute(
                "DELETE FROM ra_hash WHERE game IN (SELECT id FROM ra_game WHERE console = ?1)",
                [id],
            )?;
            tx.execute("DELETE FROM ra_game WHERE console = ?1", [id])?;
            {
                let mut game = tx.prepare(
                    "INSERT OR REPLACE INTO ra_game (id, console, title, achievements, points)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                )?;
                let mut hash = tx.prepare("INSERT OR REPLACE INTO ra_hash VALUES (?1, ?2)")?;
                for g in games.iter().filter(|g| g.num_achievements > 0) {
                    game.execute(params![g.id, id, g.title, g.num_achievements, g.points])?;
                    report.games += 1;
                    for h in &g.hashes {
                        hash.execute(params![h.to_ascii_lowercase(), g.id])?;
                        report.hashes += 1;
                    }
                }
            }
            tx.execute(
                "INSERT OR REPLACE INTO ra_console VALUES (?1, ?2)",
                [id as i64, now],
            )?;
            tx.commit()?;
            report.consoles += 1;
        }
        progress(ids.len(), ids.len());
        Ok(report)
    }

    /// Unix seconds of the last successful sync of any console, if any.
    pub fn ra_synced(&self) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row("SELECT max(synced) FROM ra_console", [], |r| r.get(0))?)
    }

    /// The RA game whose hash list contains `hash`.
    pub fn ra_game(&self, hash: &str) -> Result<Option<RaGame>> {
        Ok(self
            .conn
            .prepare_cached(
                "SELECT g.id, g.console, g.title, g.achievements, g.points
                 FROM ra_hash h JOIN ra_game g ON g.id = h.game WHERE h.hash = ?1",
            )?
            .query_row([hash], |r| game(r, 0))
            .optional()?)
    }

    /// RA games of the hashed files below `root` (from the hash cache; nothing is read).
    pub fn ra_file_games(&self, root: &Path) -> Result<HashMap<PathBuf, RaGame>> {
        let pre = prefix(root);
        let mut stmt = self.conn.prepare_cached(
            "SELECT f.path, g.id, g.console, g.title, g.achievements, g.points
             FROM ra_file f JOIN ra_hash h ON h.hash = f.hash JOIN ra_game g ON g.id = h.game
             WHERE substr(f.path, 1, ?2) = ?1",
        )?;
        let rows = stmt.query_map(params![pre, pre.chars().count() as i64], |r| {
            Ok((PathBuf::from(r.get::<_, String>(0)?), game(r, 1)?))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Main RA games (no subsets, hacks or homebrew) by (console, [`cheevos::title_key`]).
    pub fn ra_titles(&self) -> Result<HashMap<(u32, String), RaGame>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, console, title, achievements, points FROM ra_game
             WHERE title NOT LIKE '%[Subset%' AND title NOT LIKE '~%'",
        )?;
        let rows = stmt.query_map([], |r| game(r, 0))?;
        let mut map = HashMap::new();
        for g in rows {
            let g = g?;
            map.insert((g.console, cheevos::title_key(&g.title)), g);
        }
        Ok(map)
    }

    /// RA hash of `path` (cached by size + mtime); `None` if it cannot be hashed.
    pub fn ra_hash(&self, path: &Path, method: Method) -> Result<Option<String>> {
        let meta = std::fs::metadata(path)?;
        let size = meta.len() as i64;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs() as i64);
        let k = key(path);
        let cached: Option<Option<String>> = self
            .conn
            .prepare_cached(
                "SELECT hash FROM ra_file WHERE path = ?1 AND size = ?2 AND mtime = ?3",
            )?
            .query_row(params![k, size, mtime], |r| r.get(0))
            .optional()?;
        if let Some(h) = cached {
            return Ok(h);
        }
        let hash = cheevos::hash_file(path, method)?;
        self.conn
            .prepare_cached("INSERT OR REPLACE INTO ra_file VALUES (?1, ?2, ?3, ?4)")?
            .execute(params![k, size, mtime, hash])?;
        Ok(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_and_lookup() {
        let mut s = Store::open_in_memory().unwrap();
        let body = br#"[{"Title":"Game","ID":5,"ConsoleID":7,"NumAchievements":3,"Points":40,
            "Hashes":["ABCDEF"]},{"Title":"Empty","ID":6,"NumAchievements":0,"Hashes":["11"]}]"#;
        let fetch = |url: &str| {
            if url.contains("i=7&") {
                Ok(body.to_vec())
            } else if url.contains("i=27&") {
                Err(std::io::Error::other(format!("401 for {url}")))
            } else {
                Ok(b"[]".to_vec())
            }
        };
        let r = s.ra_sync_with(&fetch, "secret", 9, &|_, _| {}).unwrap();
        assert_eq!((r.games, r.hashes), (1, 1));
        assert_eq!(r.failed.len(), 1);
        assert!(!r.failed[0].1.contains("secret"));
        let g = s.ra_game("abcdef").unwrap().unwrap();
        assert_eq!((g.id, g.achievements, g.title.as_str()), (5, 3, "Game"));
        assert!(s.ra_game("11").unwrap().is_none());
        assert_eq!(s.ra_synced().unwrap(), Some(9));
    }

    #[test]
    fn caches_file_hash() {
        let s = Store::open_in_memory().unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path().join("a.gb");
        std::fs::write(&p, b"gb").unwrap();
        let h = s.ra_hash(&p, Method::Whole).unwrap().unwrap();
        assert_eq!(h.len(), 32);
        assert_eq!(s.ra_hash(&p, Method::Whole).unwrap(), Some(h));
    }
}
