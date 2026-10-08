//! The logged-in user's RetroAchievements progress per game (Web API
//! `API_GetUserCompletionProgress`, 500 games per request).

use crate::{Result, Store, http_get};
use rusqlite::params;
use std::collections::HashMap;
use std::io;

const API: &str = "https://retroachievements.org/API/API_GetUserCompletionProgress.php";
const PAGE: usize = 500;

/// Unlocks of one game.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct RaProgress {
    pub awarded: u32,
    /// Of `awarded`, unlocked in hardcore.
    pub hardcore: u32,
    pub total: u32,
    /// Highest award: `mastered`, `completed`, `beaten-hardcore`, `beaten-softcore`.
    pub award: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Page {
    total: usize,
    results: Vec<Row>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Row {
    #[serde(rename = "GameID")]
    game_id: u64,
    max_possible: u32,
    num_awarded: u32,
    num_awarded_hardcore: u32,
    highest_award_kind: Option<String>,
}

fn parse(body: &[u8]) -> io::Result<Page> {
    serde_json::from_slice(body).map_err(io::Error::other)
}

impl Store {
    /// Replaces the stored progress with `user`'s current one; returns the number of games.
    pub fn ra_progress_sync(&mut self, api_key: &str, user: &str) -> Result<usize> {
        let mut rows = Vec::new();
        loop {
            let url = format!("{API}?u={user}&y={api_key}&c={PAGE}&o={}", rows.len());
            let body = http_get(&url)
                .map_err(|e| io::Error::other(e.to_string().replace(api_key, "***")))?;
            let page = parse(&body)?;
            let n = page.results.len();
            rows.extend(page.results);
            if n < PAGE || rows.len() >= page.total {
                break;
            }
        }
        self.ra_progress_store(&rows)?;
        Ok(rows.len())
    }

    fn ra_progress_store(&mut self, rows: &[Row]) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM ra_progress", [])?;
        {
            let mut ins = tx.prepare(
                "INSERT OR REPLACE INTO ra_progress (game, awarded, hardcore, total, award)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for r in rows {
                ins.execute(params![
                    r.game_id,
                    r.num_awarded,
                    r.num_awarded_hardcore,
                    r.max_possible,
                    r.highest_award_kind
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Forgets the progress (logout).
    pub fn ra_progress_clear(&self) -> Result<()> {
        self.conn.execute("DELETE FROM ra_progress", [])?;
        Ok(())
    }

    /// Progress per RA game id.
    pub fn ra_progress(&self) -> Result<HashMap<u64, RaProgress>> {
        let mut st = self
            .conn
            .prepare("SELECT game, awarded, hardcore, total, award FROM ra_progress")?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get(0)?,
                RaProgress {
                    awarded: r.get(1)?,
                    hardcore: r.get(2)?,
                    total: r.get(3)?,
                    award: r.get(4)?,
                },
            ))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_stores_progress() {
        let body = br#"{"Count":2,"Total":2,"Results":[
            {"GameID":1,"Title":"Sonic","ConsoleID":1,"MaxPossible":23,"NumAwarded":23,
             "NumAwardedHardcore":10,"HighestAwardKind":"mastered","HighestAwardDate":"2024-01-01T00:00:00+00:00"},
            {"GameID":2,"Title":"Mario","ConsoleID":3,"MaxPossible":40,"NumAwarded":3,
             "NumAwardedHardcore":0,"HighestAwardKind":null}]}"#;
        let page = parse(body).unwrap();
        assert_eq!(page.total, 2);
        let mut s = Store::open_in_memory().unwrap();
        s.ra_progress_store(&page.results).unwrap();
        let p = s.ra_progress().unwrap();
        assert_eq!(p[&1].award.as_deref(), Some("mastered"));
        assert_eq!(
            (p[&2].awarded, p[&2].total, p[&2].award.clone()),
            (3, 40, None)
        );
        s.ra_progress_clear().unwrap();
        assert!(s.ra_progress().unwrap().is_empty());
    }
}
