//! Gathers gamification inputs (1G1R sets, metadata, journals) and persists unlocked achievements.

use crate::{Result, Store};
use rombro_core::g1r::select;
use rombro_core::gamify::{self, Achievement, Inputs, Kpis, Level, Owned};
use rombro_core::plan::{Op, TRASH_DIR, journal_from_json};
use rusqlite::params;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Genre and release year of an owned game.
type Meta = (Option<String>, Option<u64>);

#[derive(Debug, Clone, Serialize)]
pub struct Stats {
    pub kpis: Kpis,
    pub level: Level,
    /// All achievements; unlocked ones carry their unlock time.
    pub achievements: Vec<(Achievement, Option<i64>)>,
    /// Ids unlocked by this call.
    pub new: Vec<String>,
}

impl Store {
    /// Computes the stats for the identified library games `(system, name)`;
    /// newly met achievements are stored with `now` (unix seconds) and stay unlocked.
    pub fn gamify(
        &self,
        owned: &[(String, String)],
        unknown: u64,
        ambiguous: u64,
        now: i64,
    ) -> Result<Stats> {
        let rules = self.rules()?;
        let mut by_system: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (s, n) in owned {
            by_system.entry(s).or_default().push(n);
        }
        let mut systems = Vec::new();
        let mut meta: HashMap<(&str, &str), Meta> = HashMap::new();
        for (system, names) in &by_system {
            let mut entries = self.by_system(system)?;
            entries.retain(|r| !r.name.is_empty());
            let keys: HashSet<String> = select(&entries, |r| &r.name, &rules)
                .into_iter()
                .filter(|g| !g.picked.is_empty())
                .map(|g| g.key)
                .collect();
            systems.push(gamify::progress(system, names.iter().copied(), &keys));
            let wanted: HashSet<&str> = names.iter().copied().collect();
            for r in entries {
                if let Some(n) = wanted.get(r.name.as_str()) {
                    let m = meta.entry((system, n)).or_default();
                    m.0 = m.0.take().or(r.genre);
                    m.1 = m.1.or(r.release_year);
                }
            }
        }
        let games: Vec<Owned> = owned
            .iter()
            .map(|(s, n)| {
                let m = meta.get(&(s.as_str(), n.as_str()));
                Owned {
                    system: s,
                    name: n,
                    genre: m.and_then(|m| m.0.as_deref()),
                    year: m.and_then(|m| m.1),
                }
            })
            .collect();
        let mut inp = Inputs {
            unknown,
            ambiguous,
            today: now.div_euclid(86_400),
            ..Inputs::default()
        };
        for (j, state) in self.journals(usize::MAX >> 1)? {
            if state != "done" {
                continue;
            }
            inp.runs += 1;
            inp.run_days.push(j.ts.div_euclid(86_400));
            let done = journal_from_json(&j.done).unwrap_or_default();
            for d in &done {
                if let Op::Move { to, .. } = &d.op
                    && to.components().any(|c| c.as_os_str() == TRASH_DIR)
                {
                    inp.trashed += 1;
                    inp.trashed_bytes += std::fs::metadata(to).map(|m| m.len()).unwrap_or(0);
                }
            }
        }
        let kpis = gamify::kpis(&games, systems, &inp);
        let mut achs = gamify::achievements(&kpis);
        let mut stored = self.unlocked()?;
        let mut new = Vec::new();
        for a in &mut achs {
            if a.unlocked && !stored.contains_key(&a.id) {
                self.conn.execute(
                    "INSERT OR IGNORE INTO achievement (id, unlocked_at) VALUES (?1, ?2)",
                    params![a.id, now],
                )?;
                stored.insert(a.id.clone(), now);
                new.push(a.id.clone());
            }
            // once earned, an achievement stays unlocked
            a.unlocked = stored.contains_key(&a.id);
        }
        let level = gamify::level(&kpis, &achs);
        let achievements = achs
            .into_iter()
            .map(|a| {
                let t = stored.get(&a.id).copied();
                (a, t)
            })
            .collect();
        Ok(Stats {
            kpis,
            level,
            achievements,
            new,
        })
    }

    fn unlocked(&self) -> Result<HashMap<String, i64>> {
        let mut st = self
            .conn
            .prepare("SELECT id, unlocked_at FROM achievement")?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}
