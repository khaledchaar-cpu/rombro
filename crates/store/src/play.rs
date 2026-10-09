//! Launcher data: the core chosen for a single game (setting `core:<game key>`), play time and
//! favorites. All three follow the game's content (`sha1:<hex>` of its first
//! indexed ROM), so they survive renames and moves; unindexed files fall back to `path:<abs>`.

use crate::files::{key, prefix};
use crate::{Result, Store};
use romburak_core::CachedRom;
use rusqlite::{OptionalExtension, params};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::{Path, PathBuf};

/// Runs shorter than this (seconds) are not counted (failed starts, quick checks).
pub const MIN_PLAY_SECS: u64 = 30;

/// Play statistics of one game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct PlayStats {
    pub plays: u64,
    pub seconds: u64,
    /// Unix seconds of the last counted run.
    pub last: i64,
}

fn content_key(roms: &str) -> Option<String> {
    let roms: Vec<CachedRom> = serde_json::from_str(roms).ok()?;
    // archives keep their whole-file hash as `member: None`; prefer the first member
    let r = roms.iter().find(|r| r.member.is_some()).or(roms.first())?;
    let mut s = String::from("sha1:");
    for b in r.hashes.sha1 {
        let _ = write!(s, "{b:02x}");
    }
    Some(s)
}

/// Pre-v0.9.1 override key (by path); still read so existing choices keep working.
fn legacy_override_key(game: &Path) -> String {
    format!("core:{}", key(game))
}

impl Store {
    /// Key under which play time and favorites of `game` are stored.
    pub fn game_key(&self, game: &Path) -> Result<String> {
        let roms: Option<String> = self
            .conn
            .query_row("SELECT roms FROM file WHERE path = ?1", [key(game)], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(roms
            .and_then(|r| content_key(&r))
            .unwrap_or_else(|| format!("path:{}", key(game))))
    }

    /// [`game_key`](Self::game_key) of every indexed file below `root` (one query).
    pub fn game_keys(&self, root: &Path) -> Result<HashMap<PathBuf, String>> {
        let pre = prefix(root);
        let mut stmt = self
            .conn
            .prepare_cached("SELECT path, roms FROM file WHERE substr(path, 1, ?2) = ?1")?;
        let rows = stmt.query_map(params![pre, pre.chars().count() as i64], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        let mut map = HashMap::new();
        for row in rows {
            let (path, roms) = row?;
            if let Some(k) = content_key(&roms) {
                map.insert(PathBuf::from(path), k);
            }
        }
        Ok(map)
    }

    /// Counts a run of `seconds` ending at `at` (unix seconds); runs below [`MIN_PLAY_SECS`]
    /// are ignored. Returns whether it was counted.
    pub fn record_play(&self, game: &str, seconds: u64, at: i64) -> Result<bool> {
        if seconds < MIN_PLAY_SECS {
            return Ok(false);
        }
        self.conn.execute(
            "INSERT INTO play_stats (game, plays, seconds, last) VALUES (?1, 1, ?2, ?3)
             ON CONFLICT(game) DO UPDATE SET plays = plays + 1, seconds = seconds + ?2, last = ?3",
            params![game, seconds as i64, at],
        )?;
        self.conn.execute(
            "INSERT INTO play_session (game, start, seconds) VALUES (?1, ?2, ?3)",
            params![game, at - seconds as i64, seconds as i64],
        )?;
        Ok(true)
    }

    /// Counted runs starting at or after `since` as (start, seconds), oldest first.
    pub fn play_sessions(&self, since: i64) -> Result<Vec<(i64, u64)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT start, seconds FROM play_session WHERE start >= ?1 ORDER BY start",
        )?;
        let rows = stmt.query_map([since], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as u64)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn play_stats(&self, game: &str) -> Result<PlayStats> {
        Ok(self
            .conn
            .query_row(
                "SELECT plays, seconds, last FROM play_stats WHERE game = ?1",
                [game],
                |r| {
                    Ok(PlayStats {
                        plays: r.get::<_, i64>(0)? as u64,
                        seconds: r.get::<_, i64>(1)? as u64,
                        last: r.get(2)?,
                    })
                },
            )
            .optional()?
            .unwrap_or_default())
    }

    /// Play statistics of all games ever counted.
    pub fn all_play_stats(&self) -> Result<HashMap<String, PlayStats>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT game, plays, seconds, last FROM play_stats")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                PlayStats {
                    plays: r.get::<_, i64>(1)? as u64,
                    seconds: r.get::<_, i64>(2)? as u64,
                    last: r.get(3)?,
                },
            ))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn set_favorite(&self, game: &str, on: bool) -> Result<()> {
        if on {
            self.conn.execute(
                "INSERT OR IGNORE INTO favorite (game, added) VALUES (?1, ?2)",
                params![game, crate::files::now()],
            )?;
        } else {
            self.conn
                .execute("DELETE FROM favorite WHERE game = ?1", [game])?;
        }
        Ok(())
    }

    pub fn favorites(&self) -> Result<HashSet<String>> {
        let mut stmt = self.conn.prepare_cached("SELECT game FROM favorite")?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Core id the user chose for `game`, overriding the system's core.
    pub fn core_override(&self, game: &Path) -> Result<Option<String>> {
        if let Some(c) = self.setting(&format!("core:{}", self.game_key(game)?))? {
            return Ok(Some(c));
        }
        self.setting(&legacy_override_key(game))
    }

    /// Sets (`Some`) or clears (`None`) the core override of `game`.
    pub fn set_core_override(&self, game: &Path, core: Option<&str>) -> Result<()> {
        let k = format!("core:{}", self.game_key(game)?);
        self.conn.execute(
            "DELETE FROM settings WHERE key IN (?1, ?2)",
            params![k, legacy_override_key(game)],
        )?;
        match core {
            Some(c) => self.set_setting(&k, c),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_clears_core_override() {
        let s = Store::open_in_memory().unwrap();
        let g = Path::new("/lib/NES/a.nes");
        assert_eq!(s.core_override(g).unwrap(), None);
        s.set_core_override(g, Some("nestopia")).unwrap();
        assert_eq!(s.core_override(g).unwrap().as_deref(), Some("nestopia"));
        s.set_core_override(g, None).unwrap();
        assert_eq!(s.core_override(g).unwrap(), None);
    }

    #[test]
    fn logs_sessions() {
        let s = Store::open_in_memory().unwrap();
        assert!(s.record_play("g", 100, 1000).unwrap());
        assert!(!s.record_play("g", 5, 2000).unwrap());
        assert!(s.record_play("h", 60, 3000).unwrap());
        assert_eq!(s.play_sessions(0).unwrap(), [(900, 100), (2940, 60)]);
        assert_eq!(s.play_sessions(1000).unwrap(), [(2940, 60)]);
    }

    #[test]
    fn play_time_and_favorites_follow_the_content() {
        let s = Store::open_in_memory().unwrap();
        let roms = |sha: u8| {
            let h = romburak_core::Hashes {
                size: 1,
                crc: 0,
                sha1: [sha; 20],
                md5: None,
            };
            serde_json::to_string(&[CachedRom {
                member: None,
                hashes: h,
                header: None,
                headerless: None,
                disc: None,
            }])
            .unwrap()
        };
        s.conn
            .execute(
                "INSERT INTO file (path, size, mtime, roms) VALUES ('/lib/NES/a.nes', 1, 1, ?1)",
                [roms(0xab)],
            )
            .unwrap();
        let k = s.game_key(Path::new("/lib/NES/a.nes")).unwrap();
        assert_eq!(k, format!("sha1:{}", "ab".repeat(20)));
        assert_eq!(s.game_keys(Path::new("/lib")).unwrap().len(), 1);
        assert_eq!(
            s.game_key(Path::new("/lib/x.m3u")).unwrap(),
            "path:/lib/x.m3u"
        );

        assert!(!s.record_play(&k, 29, 100).unwrap());
        assert!(s.record_play(&k, 60, 200).unwrap());
        assert!(s.record_play(&k, 30, 300).unwrap());
        let st = s.play_stats(&k).unwrap();
        assert_eq!((st.plays, st.seconds, st.last), (2, 90, 300));
        assert_eq!(s.all_play_stats().unwrap()[&k], st);

        s.set_core_override(Path::new("/lib/NES/a.nes"), Some("mesen"))
            .unwrap();
        s.conn
            .execute("UPDATE file SET path = '/lib/NES/b.nes'", [])
            .unwrap();
        let renamed = s.core_override(Path::new("/lib/NES/b.nes")).unwrap();
        assert_eq!(renamed.as_deref(), Some("mesen"));

        s.set_favorite(&k, true).unwrap();
        s.set_favorite(&k, true).unwrap();
        assert!(s.favorites().unwrap().contains(&k));
        s.set_favorite(&k, false).unwrap();
        assert!(s.favorites().unwrap().is_empty());
    }
}
