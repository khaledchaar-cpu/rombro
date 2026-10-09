//! RetroAchievements popularity: distinct players per game (`API_GetGameExtended`), fetched
//! slowly in the background for the library's games and kept for [`MAX_AGE`] seconds.

use crate::cheevos::polite_get;
use crate::dat_sync::Fetch;
use crate::{Result, Store};
use rusqlite::params;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

/// Player counts older than this (30 days) are fetched again.
pub const MAX_AGE: i64 = 30 * 24 * 3600;

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiPlayers {
    #[serde(default)]
    num_distinct_players: u64,
}

/// Distinct players from a game info body (any endpoint carrying `NumDistinctPlayers`).
pub(crate) fn parse_players(body: &[u8]) -> std::io::Result<u64> {
    let p: ApiPlayers = serde_json::from_slice(body).map_err(std::io::Error::other)?;
    Ok(p.num_distinct_players)
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct RaPlayersReport {
    pub fetched: usize,
    pub failed: usize,
    pub cancelled: bool,
}

impl Store {
    /// Distinct players per RA game id.
    pub fn ra_players(&self) -> Result<HashMap<u64, u64>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT game, players FROM ra_players")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn set_ra_players(&self, game: u64, players: u64, now: i64) -> Result<()> {
        self.conn
            .prepare_cached("INSERT OR REPLACE INTO ra_players VALUES (?1, ?2, ?3)")?
            .execute(params![game, players, now])?;
        Ok(())
    }

    /// Of `games`, those without a player count or with one older than [`MAX_AGE`].
    pub fn ra_players_stale(&self, games: &[u64], now: i64) -> Result<Vec<u64>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT fetched FROM ra_players WHERE game = ?1")?;
        let mut out = Vec::new();
        for &g in games {
            let fetched: Option<i64> = stmt.query_row([g], |r| r.get(0)).ok();
            if fetched.is_none_or(|f| now - f > MAX_AGE) {
                out.push(g);
            }
        }
        Ok(out)
    }

    /// Fetches the player counts of `games` one by one (politely paced); stops early when
    /// `cancel` is set. Failed games are skipped and retried next time.
    pub fn ra_players_fetch(
        &self,
        api_key: &str,
        games: &[u64],
        now: i64,
        cancel: &AtomicBool,
        progress: &dyn Fn(usize, usize),
    ) -> Result<RaPlayersReport> {
        self.ra_players_fetch_with(&polite_get, api_key, games, now, cancel, progress)
    }

    pub fn ra_players_fetch_with(
        &self,
        fetch: Fetch,
        api_key: &str,
        games: &[u64],
        now: i64,
        cancel: &AtomicBool,
        progress: &dyn Fn(usize, usize),
    ) -> Result<RaPlayersReport> {
        let mut report = RaPlayersReport::default();
        for (i, &g) in games.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                report.cancelled = true;
                break;
            }
            progress(i, games.len());
            let url = format!(
                "https://retroachievements.org/API/API_GetGameExtended.php?i={g}&y={api_key}"
            );
            match fetch(&url).and_then(|b| parse_players(&b)) {
                Ok(n) => {
                    self.set_ra_players(g, n, now)?;
                    report.fetched += 1;
                }
                Err(_) => report.failed += 1,
            }
        }
        progress(games.len(), games.len());
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetches_stale_and_keeps_fresh() {
        let s = Store::open_in_memory().unwrap();
        s.set_ra_players(1, 10, 100).unwrap();
        s.set_ra_players(2, 20, 100 - MAX_AGE - 1).unwrap();
        assert_eq!(s.ra_players_stale(&[1, 2, 3], 100).unwrap(), vec![2, 3]);
        let fetch = |url: &str| {
            if url.contains("i=3&") {
                Err(std::io::Error::other("boom"))
            } else {
                Ok(br#"{"NumDistinctPlayers":4711,"Achievements":{}}"#.to_vec())
            }
        };
        let r = s
            .ra_players_fetch_with(
                &fetch,
                "k",
                &[2, 3],
                100,
                &AtomicBool::new(false),
                &|_, _| {},
            )
            .unwrap();
        assert_eq!((r.fetched, r.failed, r.cancelled), (1, 1, false));
        let p = s.ra_players().unwrap();
        assert_eq!((p[&1], p[&2], p.get(&3)), (10, 4711, None));
    }

    #[test]
    fn cancel_stops_fetching() {
        let s = Store::open_in_memory().unwrap();
        let fetch = |_: &str| Ok(b"{}".to_vec());
        let r = s
            .ra_players_fetch_with(&fetch, "k", &[1, 2], 0, &AtomicBool::new(true), &|_, _| {})
            .unwrap();
        assert!(r.cancelled);
        assert!(s.ra_players().unwrap().is_empty());
    }
}
