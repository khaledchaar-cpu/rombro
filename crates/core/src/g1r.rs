//! 1G1R ("one game, one ROM"): group releases by normalized title and pick the best one.

use crate::naming::{self, Flags, NameInfo};
use std::cmp::Reverse;
use std::collections::{BTreeMap, HashSet};

/// Selection rules (configurable; defaults per SPEC §11a).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Rules {
    /// Preferred regions, best first. Unlisted regions rank after all listed ones.
    pub regions: Vec<String>,
    /// Preferred languages (tie-breaker after region), best first.
    pub languages: Vec<String>,
    /// Releases with any of these flags are never picked.
    pub exclude: Flags,
    /// Arcade databases in placement priority (a set matching several lands in the first).
    pub arcade_order: Vec<String>,
    /// 1G1R for arcade sets; off places every matching set.
    pub arcade_g1r: bool,
    /// Sets a core's DAT marks as not working (driver `preliminary`) count as incomplete
    /// for that core; with no core left they go to the trash like other incomplete sets.
    pub arcade_working_only: bool,
    /// Systems whose matches are key files inside a game folder (moved as a whole).
    pub folder_systems: Vec<String>,
    /// Move unknown files to `_quarantine`; off leaves them where they are.
    pub quarantine: bool,
    /// Unknown files go to `_trash/unknown` (and the old `_quarantine` is emptied there);
    /// off keeps them in `_quarantine`.
    pub unknown_to_trash: bool,
    /// Move frontend metadata (gamelist.xml, scraped media) to `_trash/frontend`.
    pub frontend_trash: bool,
    /// Per-system overrides of regions, languages and excluded flags.
    pub systems: BTreeMap<String, SystemRules>,
    /// RetroArch core per system (system → core id, e.g. `snes9x`); unlisted systems use
    /// the recommendation (`retroarch::pick`).
    pub cores: BTreeMap<String, String>,
    /// Folder name → system for files without database match (cartridge conversions, games
    /// without a database); their file name is the release name (SPEC §11a).
    pub name_folders: BTreeMap<String, String>,
}

/// Overrides for one system; `None` falls back to the global value.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SystemRules {
    pub regions: Option<Vec<String>>,
    pub languages: Option<Vec<String>>,
    pub exclude: Option<Flags>,
}

impl Rules {
    /// The rules in effect for `system` (global values with its overrides applied).
    pub fn for_system(&self, system: &str) -> std::borrow::Cow<'_, Self> {
        let Some(o) = self.systems.get(system) else {
            return std::borrow::Cow::Borrowed(self);
        };
        let mut r = self.clone();
        if let Some(v) = &o.regions {
            r.regions.clone_from(v);
        }
        if let Some(v) = &o.languages {
            r.languages.clone_from(v);
        }
        if let Some(v) = &o.exclude {
            r.exclude = *v;
        }
        std::borrow::Cow::Owned(r)
    }
}

impl Default for Rules {
    fn default() -> Self {
        let s = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect();
        Self {
            regions: s(&["Europe", "World", "USA", "Germany", "Japan"]),
            languages: s(&["En", "De"]),
            exclude: Flags {
                beta: true,
                proto: true,
                demo: true,
                kiosk: true,
                sample: true,
                unlicensed: true,
                pirate: true,
                bios: true,
                hack: true,
                translation: true,
                bad_dump: true,
                ..Flags::default()
            },
            arcade_order: s(&crate::arcade::PRIORITY),
            arcade_g1r: true,
            arcade_working_only: true,
            folder_systems: s(&crate::plan::FOLDER_SYSTEMS),
            quarantine: true,
            unknown_to_trash: true,
            frontend_trash: true,
            systems: BTreeMap::new(),
            cores: BTreeMap::new(),
            name_folders: BTreeMap::from(
                [("n64dd", "Nintendo - Nintendo 64"), ("solarus", "Solarus")]
                    .map(|(f, s)| (f.to_owned(), s.to_owned())),
            ),
        }
    }
}

/// Why a release was not picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    Excluded(&'static str),
    /// Same name as another entry (e.g. reprint with identical dump, different serial).
    Duplicate,
    Region,
    Language,
    /// Alternative dump, digital re-release, Virtual Console, aftermarket.
    Variant,
    Revision,
    TieBreak,
}

#[derive(Debug)]
pub struct GroupPick<'a, T> {
    pub key: String,
    /// All media (discs) of the chosen release; empty if every release is excluded.
    pub picked: Vec<&'a T>,
    pub rejected: Vec<(&'a T, Reason)>,
    /// Another release scored exactly as well as the pick (`Reason::TieBreak`): no rule
    /// decides, so the user should confirm the pick (SPEC §11a, ambiguous matches).
    pub needs_decision: bool,
}

/// Groups `items` (all of one system) and picks one release per game.
/// Groups are returned sorted by key; multi-disc releases stay together.
pub fn select<'a, T>(
    items: &'a [T],
    name: impl Fn(&T) -> &str,
    rules: &Rules,
) -> Vec<GroupPick<'a, T>> {
    // group key → release (name without disc tag) → media
    let mut groups: BTreeMap<String, Group<'a, T>> = BTreeMap::new();
    let mut seen = HashSet::new();
    for it in items {
        let n = name(it);
        let g = groups
            .entry(naming::group_key(naming::parse(n).title))
            .or_default();
        if seen.insert(n) {
            g.releases
                .entry(naming::release_name(n))
                .or_default()
                .push(it);
        } else {
            g.dups.push(it);
        }
    }
    groups
        .into_iter()
        .map(|(key, g)| {
            let mut pick = pick_group(key, g.releases, rules);
            pick.rejected
                .extend(g.dups.into_iter().map(|d| (d, Reason::Duplicate)));
            pick
        })
        .collect()
}

struct Group<'a, T> {
    /// release (name without disc tag) → media
    releases: BTreeMap<String, Vec<&'a T>>,
    dups: Vec<&'a T>,
}

impl<T> Default for Group<'_, T> {
    fn default() -> Self {
        Self {
            releases: BTreeMap::new(),
            dups: Vec::new(),
        }
    }
}

/// region, language, variant penalty, revision (newer first), language count (more first)
type Score = (usize, usize, u8, Reverse<u32>, Reverse<usize>);

fn pick_group<'a, T>(
    key: String,
    releases: BTreeMap<String, Vec<&'a T>>,
    rules: &Rules,
) -> GroupPick<'a, T> {
    let mut rejected = Vec::new();
    let mut ranked: Vec<(Score, Vec<&'a T>)> = Vec::new();
    for (rel, media) in releases {
        let info = naming::parse(&rel);
        if let Some(flag) = excluded_by(&info.flags, &rules.exclude) {
            rejected.extend(media.into_iter().map(|m| (m, Reason::Excluded(flag))));
        } else {
            ranked.push((score(&info, rules), media));
        }
    }
    // Stable sort keeps BTreeMap (name) order as the final tie-breaker.
    ranked.sort_by_key(|(s, _)| *s);
    let mut it = ranked.into_iter();
    let Some((best, picked)) = it.next() else {
        return GroupPick {
            key,
            picked: Vec::new(),
            rejected,
            needs_decision: false,
        };
    };
    for (s, media) in it {
        let reason = if s.0 != best.0 {
            Reason::Region
        } else if s.1 != best.1 {
            Reason::Language
        } else if s.2 != best.2 {
            Reason::Variant
        } else if s.3 != best.3 {
            Reason::Revision
        } else if s.4 != best.4 {
            Reason::Language
        } else {
            Reason::TieBreak
        };
        rejected.extend(media.into_iter().map(|m| (m, reason)));
    }
    let needs_decision = rejected.iter().any(|(_, r)| *r == Reason::TieBreak);
    GroupPick {
        key,
        picked,
        rejected,
        needs_decision,
    }
}

fn score(info: &NameInfo<'_>, rules: &Rules) -> Score {
    let rank = |prefs: &[String], have: &[&str]| {
        have.iter()
            .filter_map(|h| prefs.iter().position(|p| p == h))
            .min()
            .unwrap_or(prefs.len())
    };
    let f = &info.flags;
    let variant = u8::from(f.alt)
        + u8::from(f.rerelease)
        + u8::from(f.virtual_console)
        + u8::from(f.aftermarket);
    (
        rank(&rules.regions, &info.regions),
        rank(&rules.languages, &info.languages),
        variant,
        Reverse(info.revision),
        Reverse(info.languages.len()),
    )
}

fn excluded_by(f: &Flags, ex: &Flags) -> Option<&'static str> {
    [
        (f.beta && ex.beta, "beta"),
        (f.proto && ex.proto, "proto"),
        (f.demo && ex.demo, "demo"),
        (f.kiosk && ex.kiosk, "kiosk"),
        (f.sample && ex.sample, "sample"),
        (f.unlicensed && ex.unlicensed, "unlicensed"),
        (f.pirate && ex.pirate, "pirate"),
        (f.bios && ex.bios, "bios"),
        (f.aftermarket && ex.aftermarket, "aftermarket"),
        (f.virtual_console && ex.virtual_console, "virtual console"),
        (f.rerelease && ex.rerelease, "re-release"),
        (f.hack && ex.hack, "hack"),
        (f.translation && ex.translation, "translation"),
        (f.bad_dump && ex.bad_dump, "bad dump"),
        (f.alt && ex.alt, "alt"),
    ]
    .into_iter()
    .find_map(|(hit, n)| hit.then_some(n))
}

#[cfg(test)]
#[path = "g1r_tests.rs"]
mod tests;
