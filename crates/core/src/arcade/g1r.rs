//! 1G1R for arcade sets. The databases carry no parent/clone links, so sets are grouped by
//! title (text before the first parenthesis) across every database that lists them; a set
//! named differently in one database (`Gradius III: Densetsu kara Shinwa e (Japan)` in FBNeo,
//! `Gradius III (Japan)` in MAME) still joins its group through the other name.

use crate::g1r::Reason;
use std::cmp::Reverse;
use std::collections::HashMap;

/// Normalized title: text before the first ` (`, lowercase alphanumeric words.
pub fn title(name: &str) -> String {
    let head = name.split(" (").next().unwrap_or(name);
    head.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Regions named in the first parenthesis (`World 910522`, `Japan, version 3`, `US`).
fn regions(name: &str) -> Vec<&'static str> {
    let Some(tag) = name.split_once(" (").and_then(|(_, r)| r.split(')').next()) else {
        return Vec::new();
    };
    tag.split(|c: char| !c.is_alphanumeric())
        .filter_map(|w| {
            Some(match w.to_ascii_lowercase().as_str() {
                "world" => "World",
                "usa" | "us" | "america" => "USA",
                "europe" | "euro" => "Europe",
                "japan" => "Japan",
                "asia" => "Asia",
                "korea" => "Korea",
                "taiwan" => "Taiwan",
                "hispanic" => "Spain",
                "brazil" => "Brazil",
                "germany" => "Germany",
                "france" => "France",
                "italy" => "Italy",
                "uk" => "UK",
                _ => return None,
            })
        })
        .collect()
}

/// Bootlegs, hacks, prototypes and the like lose against an original when there is one.
fn is_unofficial(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    [
        "bootleg",
        "hack",
        "prototype",
        "proto",
        "location test",
        "pirate",
        "unlicensed",
    ]
    .iter()
    .any(|w| n.contains(w))
}

/// Ranks the sets of one group: region order from `regions` (unlisted last), then original
/// over unofficial, then newest (dates and revisions sort higher, e.g. `910522` > `910204`).
fn key<'a>(name: &'a str, prefs: &[String]) -> (usize, bool, Reverse<&'a str>) {
    let region = regions(name)
        .iter()
        .filter_map(|r| prefs.iter().position(|p| p == r))
        .min()
        .unwrap_or(prefs.len());
    (region, is_unofficial(name), Reverse(name))
}

/// Groups `sets` (each with all its database names, best-ranked system's first) and picks
/// one per group. Returns, per set index, `None` if picked or the reason it was rejected,
/// together with the index of the picked set of its group.
pub fn select(sets: &[Vec<&str>], prefs: &[String]) -> Vec<(Option<Reason>, usize)> {
    // union-find over shared titles
    let mut parent: Vec<usize> = (0..sets.len()).collect();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let mut by_title: HashMap<String, usize> = HashMap::new();
    for (i, names) in sets.iter().enumerate() {
        for n in names {
            let t = title(n);
            if t.is_empty() {
                continue;
            }
            match by_title.get(&t) {
                Some(&j) => {
                    let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                    parent[a] = b;
                }
                None => {
                    by_title.insert(t, i);
                }
            }
        }
    }
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..sets.len() {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }
    let mut out = vec![(None, 0); sets.len()];
    for members in groups.values() {
        let name = |i: usize| sets[i].first().copied().unwrap_or_default();
        let Some(&best) = members.iter().min_by_key(|&&i| (key(name(i), prefs), i)) else {
            continue;
        };
        for &i in members {
            let reason = if i == best {
                None
            } else if name(i) == name(best) {
                Some(Reason::Duplicate)
            } else {
                let (a, b) = (key(name(i), prefs), key(name(best), prefs));
                Some(if a.0 != b.0 {
                    Reason::Region
                } else if a.1 != b.1 {
                    Reason::Variant
                } else {
                    Reason::Revision
                })
            };
            out[i] = (reason, best);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefs() -> Vec<String> {
        ["Europe", "World", "USA", "Germany", "Japan"]
            .map(String::from)
            .to_vec()
    }

    #[test]
    fn titles_and_regions() {
        assert_eq!(title("Gradius III (World, version R)"), "gradius iii");
        assert_eq!(
            title("Street Fighter II: The World Warrior (World 910522)"),
            "street fighter ii the world warrior"
        );
        assert_eq!(regions("Gradius III (Japan, version 3, newer)"), ["Japan"]);
        assert_eq!(regions("Street Fighter II (USA 920312)"), ["USA"]);
    }

    #[test]
    fn picks_world_and_joins_groups_through_other_database_names() {
        let sets = vec![
            vec![
                "Gradius III: Densetsu kara Shinwa e (Japan, version 3, newer)",
                "Gradius III (Japan)",
            ],
            vec![
                "Gradius III (World, version R)",
                "Gradius III (World, program code R)",
            ],
            vec!["Zero Wing (2 player)"],
        ];
        let r = select(&sets, &prefs());
        assert_eq!(r[1], (None, 1));
        assert_eq!(r[0], (Some(Reason::Region), 1));
        assert_eq!(r[2], (None, 2));
    }

    #[test]
    fn newest_original_wins_within_a_region() {
        let sets = vec![
            vec!["Street Fighter II: The World Warrior (World 910204)"],
            vec!["Street Fighter II: The World Warrior (World 910522)"],
            vec!["Street Fighter II: The World Warrior (bootleg, set 4)"],
            vec!["Street Fighter II: The World Warrior (World 910522)"],
        ];
        let r = select(&sets, &prefs());
        assert_eq!(r[1], (None, 1));
        assert_eq!(r[0], (Some(Reason::Revision), 1));
        assert_eq!(r[2].0, Some(Reason::Region));
        assert_eq!(r[3], (Some(Reason::Duplicate), 1));
    }
}
