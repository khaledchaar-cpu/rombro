//! Gamification (SPEC F6): KPIs, completeness against the 1G1R set, achievements, XP and streaks.
//! Pure computation; the store gathers the inputs.

use crate::naming;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

/// One identified game in the library.
#[derive(Debug, Clone)]
pub struct Owned<'a> {
    pub system: &'a str,
    pub name: &'a str,
    pub genre: Option<&'a str>,
    pub year: Option<u64>,
}

/// Library-wide inputs besides the owned games.
#[derive(Debug, Clone, Default)]
pub struct Inputs {
    /// Library units that could not be identified (incl. ambiguous ones).
    pub unknown: u64,
    pub ambiguous: u64,
    /// Files moved to the trash by executed (not undone) runs.
    pub trashed: u64,
    /// Executed (not undone) import runs.
    pub runs: u64,
    /// Unix days (ts / 86400) with at least one executed run.
    pub run_days: Vec<i64>,
    /// Today as unix day.
    pub today: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SystemProgress {
    pub system: String,
    /// Distinct games (1G1R groups) owned in any release.
    pub owned: u64,
    /// Games in the 1G1R set (groups with an eligible release).
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Kpis {
    pub games: u64,
    pub unknown: u64,
    pub ambiguous: u64,
    pub trashed: u64,
    pub runs: u64,
    pub systems: Vec<SystemProgress>,
    /// Region tag → games.
    pub regions: BTreeMap<String, u64>,
    pub genres: BTreeMap<String, u64>,
    /// Decade start (e.g. 1990) → games.
    pub decades: BTreeMap<u64, u64>,
    /// Consecutive days with a run, ending today or yesterday.
    pub streak: u64,
}

/// Counts distinct owned 1G1R groups of one system against its full 1G1R set.
/// `set_keys` are the group keys with an eligible pick (see [`crate::g1r::select`]).
pub fn progress<'a>(
    system: &str,
    owned: impl IntoIterator<Item = &'a str>,
    set_keys: &HashSet<String>,
) -> SystemProgress {
    let have: HashSet<String> = owned
        .into_iter()
        .map(|n| naming::group_key(naming::parse(n).title))
        .filter(|k| set_keys.contains(k))
        .collect();
    SystemProgress {
        system: system.to_owned(),
        owned: have.len() as u64,
        total: set_keys.len() as u64,
    }
}

/// Builds the KPIs; `systems` comes from [`progress`] per owned system.
pub fn kpis(owned: &[Owned<'_>], systems: Vec<SystemProgress>, inp: &Inputs) -> Kpis {
    let mut k = Kpis {
        games: owned.len() as u64,
        unknown: inp.unknown,
        ambiguous: inp.ambiguous,
        trashed: inp.trashed,
        runs: inp.runs,
        systems,
        streak: streak(&inp.run_days, inp.today),
        ..Kpis::default()
    };
    for o in owned {
        for r in naming::parse(o.name).regions {
            *k.regions.entry(r.to_owned()).or_default() += 1;
        }
        if let Some(g) = o.genre.filter(|g| !g.is_empty()) {
            *k.genres.entry(g.to_owned()).or_default() += 1;
        }
        if let Some(y) = o.year.filter(|y| *y >= 1950) {
            *k.decades.entry(y / 10 * 10).or_default() += 1;
        }
    }
    k
}

/// Length of the run of consecutive days ending today (or yesterday, so a streak survives until midnight).
pub fn streak(days: &[i64], today: i64) -> u64 {
    let set: HashSet<i64> = days.iter().copied().collect();
    let mut d = if set.contains(&today) {
        today
    } else {
        today - 1
    };
    let mut n = 0;
    while set.contains(&d) {
        n += 1;
        d -= 1;
    }
    n
}

#[derive(Debug, Clone, Serialize)]
pub struct Achievement {
    pub id: String,
    pub title: String,
    pub description: String,
    pub xp: u64,
    pub unlocked: bool,
}

fn ach(
    id: impl Into<String>,
    title: impl Into<String>,
    desc: impl Into<String>,
    xp: u64,
    ok: bool,
) -> Achievement {
    Achievement {
        id: id.into(),
        title: title.into(),
        description: desc.into(),
        xp,
        unlocked: ok,
    }
}

/// All achievements with their current state (unlock is evaluated, persisting is the caller's job).
pub fn achievements(k: &Kpis) -> Vec<Achievement> {
    let mut v = Vec::new();
    for (n, t) in [
        (1, "First Blood"),
        (100, "Collector"),
        (1000, "Hoarder"),
        (10000, "Archivist"),
    ] {
        v.push(ach(
            format!("games-{n}"),
            t,
            format!("Own {n} verified games"),
            xp_for(n),
            k.games >= n,
        ));
    }
    v.push(ach(
        "clean-sweep",
        "Clean Sweep",
        "0 unknown or ambiguous files in a library with games",
        250,
        k.games > 0 && k.unknown == 0 && k.ambiguous == 0,
    ));
    v.push(ach(
        "first-run",
        "Curator",
        "Execute your first import",
        50,
        k.runs >= 1,
    ));
    v.push(ach(
        "declutter",
        "Declutter",
        "Trash 50 rejected or duplicate files",
        150,
        k.trashed >= 50,
    ));
    v.push(ach(
        "globetrotter",
        "Globetrotter",
        "Own games from 5 regions",
        150,
        k.regions.len() >= 5,
    ));
    v.push(ach(
        "time-traveller",
        "Time Traveller",
        "Own games from 4 decades",
        150,
        k.decades.len() >= 4,
    ));
    v.push(ach(
        "multi-system",
        "Multi-System",
        "Own games on 10 systems",
        200,
        k.systems.iter().filter(|s| s.owned > 0).count() >= 10,
    ));
    v.push(ach(
        "streak-7",
        "On Fire",
        "Curate on 7 days in a row",
        200,
        k.streak >= 7,
    ));
    for s in &k.systems {
        if s.total > 0 && s.owned >= s.total {
            v.push(ach(
                format!("full-set:{}", s.system),
                format!("Full Set: {}", s.system),
                format!("Own every game of the 1G1R set ({})", s.total),
                500 + s.total,
                true,
            ));
        } else if s.total >= 10 && s.owned * 2 >= s.total {
            v.push(ach(
                format!("half-set:{}", s.system),
                format!("Halfway: {}", s.system),
                "Own half of the 1G1R set",
                100,
                true,
            ));
        }
    }
    v
}

fn xp_for(games: u64) -> u64 {
    (games as f64).sqrt() as u64 * 20
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Level {
    pub level: u64,
    pub xp: u64,
    /// XP at which the current level started / the next one starts.
    pub floor: u64,
    pub next: u64,
}

/// XP = 10 per game + achievement XP; level n needs 100·n² XP.
pub fn level(k: &Kpis, achs: &[Achievement]) -> Level {
    let xp = k.games * 10
        + achs
            .iter()
            .filter(|a| a.unlocked)
            .map(|a| a.xp)
            .sum::<u64>();
    let level = ((xp as f64 / 100.0).sqrt()) as u64;
    Level {
        level,
        xp,
        floor: 100 * level * level,
        next: 100 * (level + 1) * (level + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(name: &str) -> Owned<'_> {
        Owned {
            system: "S",
            name,
            genre: Some("Action"),
            year: Some(1994),
        }
    }

    #[test]
    fn progress_counts_groups_not_releases() {
        let keys: HashSet<String> = ["mario", "zelda", "metroid"].map(String::from).into();
        let p = progress(
            "S",
            [
                "Mario (Europe)",
                "Mario (USA)",
                "Zelda (Japan)",
                "Other (USA)",
            ],
            &keys,
        );
        assert_eq!((p.owned, p.total), (2, 3));
    }

    #[test]
    fn streak_allows_yesterday() {
        assert_eq!(streak(&[8, 9, 10], 10), 3);
        assert_eq!(streak(&[8, 9], 10), 2);
        assert_eq!(streak(&[7, 9, 10, 10], 10), 2);
        assert_eq!(streak(&[5], 10), 0);
    }

    #[test]
    fn kpis_and_achievements() {
        let games = [owned("A (Europe)"), owned("B (USA, Europe)")];
        let sys = vec![SystemProgress {
            system: "S".into(),
            owned: 2,
            total: 2,
        }];
        let k = kpis(
            &games,
            sys,
            &Inputs {
                runs: 1,
                run_days: vec![3],
                today: 3,
                ..Inputs::default()
            },
        );
        assert_eq!(k.regions["Europe"], 2);
        assert_eq!(k.decades[&1990], 2);
        assert_eq!(k.streak, 1);
        let a = achievements(&k);
        let on: Vec<_> = a
            .iter()
            .filter(|a| a.unlocked)
            .map(|a| a.id.as_str())
            .collect();
        assert_eq!(on, ["games-1", "clean-sweep", "first-run", "full-set:S"]);
        let l = level(&k, &a);
        assert_eq!(l.xp, 20 + 20 + 250 + 50 + 502);
        assert_eq!(l.level, 2);
    }
}
