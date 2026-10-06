//! Catalog of the rules the planner applies; every planned op names the rule behind it.

use serde::Serialize;

/// A named planner rule. The id is stable (shown in CLI/GUI, used for hit counts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rule {
    G1rPick,
    ArcadeSet,
    ArcadeDat,
    Verdict,
    MultiDisc,
    MultiDiskArchive,
    GameFolder,
    ArchiveExtracted,
    Bios,
    Quarantine,
    FrontendMeta,
    Playlist,
}

impl Rule {
    pub const ALL: [Rule; 12] = [
        Rule::G1rPick,
        Rule::ArcadeSet,
        Rule::ArcadeDat,
        Rule::Verdict,
        Rule::MultiDisc,
        Rule::MultiDiskArchive,
        Rule::GameFolder,
        Rule::ArchiveExtracted,
        Rule::Bios,
        Rule::Quarantine,
        Rule::FrontendMeta,
        Rule::Playlist,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Rule::G1rPick => "g1r-pick",
            Rule::ArcadeSet => "arcade-set",
            Rule::ArcadeDat => "arcade-dat",
            Rule::Verdict => "verdict",
            Rule::MultiDisc => "multi-disc",
            Rule::MultiDiskArchive => "multi-disk-archive",
            Rule::GameFolder => "game-folder",
            Rule::ArchiveExtracted => "archive-extracted",
            Rule::Bios => "bios",
            Rule::Quarantine => "quarantine",
            Rule::FrontendMeta => "frontend-meta",
            Rule::Playlist => "playlist",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Rule::G1rPick => "1G1R pick",
            Rule::ArcadeSet => "Arcade romset",
            Rule::ArcadeDat => "Arcade DAT check",
            Rule::Verdict => "Your decision",
            Rule::MultiDisc => "Multi-disc playlist",
            Rule::MultiDiskArchive => "Multi-disk archive",
            Rule::GameFolder => "Game folder",
            Rule::ArchiveExtracted => "Archive extracted",
            Rule::Bios => "BIOS",
            Rule::Quarantine => "Quarantine",
            Rule::FrontendMeta => "Frontend metadata",
            Rule::Playlist => "RetroArch playlist",
        }
    }

    pub fn explain(self) -> &'static str {
        match self {
            Rule::G1rPick => {
                "One release per game: region order, then preferred language, then fewest variant \
                 flags (Alt, Beta, Proto…), newest revision. Excluded flags are never picked; \
                 remaining ties become a decision."
            }
            Rule::ArcadeSet => {
                "Arcade zips are matched as a whole and keep their short name. Sets are grouped by \
                 title across FBNeo/MAME databases; database order picks the target core, region \
                 order and original-before-bootleg pick the set."
            }
            Rule::ArcadeDat => {
                "When a set is only in the mixed-version MAME database or in several arcade \
                 databases, its zip is checked against each core's DAT (file names and CRCs). It \
                 goes to the first core in database order it is complete for, else to \
                 _quarantine with the reason (missing, misnamed, parent set missing)."
            }
            Rule::Verdict => {
                "A rejected release you marked keep (placed in addition) or discard (moved to \
                 _trash, undoable)."
            }
            Rule::MultiDisc => "Releases with several discs get a folder and an .m3u playlist.",
            Rule::MultiDiskArchive => {
                "An archive holding several disks of one game becomes one game folder with an .m3u."
            }
            Rule::GameFolder => {
                "Systems played from folders (DOS, ScummVM, ports) are moved as whole folders, \
                 named after the folder, not the matched file."
            }
            Rule::ArchiveExtracted => {
                "Once every member of a multi-ROM archive is placed, the archive itself is moved \
                 to _trash (move mode or inside the library)."
            }
            Rule::Bios => "BIOS files go to the system root (_bios for FBNeo, next to MAME sets).",
            Rule::Quarantine => {
                "Files without database match (wrong dumps, unsupported systems, incomplete \
                 arcade sets) go to _trash/unknown – or to _quarantine if you prefer – keeping \
                 their subfolders; undo brings them back, only emptying the trash deletes them. \
                 Unknown files in folders without any identified game are left alone."
            }
            Rule::FrontendMeta => {
                "Frontend leftovers (Batocera, EmulationStation) go to _trash/frontend: \
                 gamelist*.xml with backups, _info.txt and scraped images/videos next to them. \
                 Only folders holding a gamelist*.xml count, so game data is never touched."
            }
            Rule::Playlist => "One RetroArch .lpl playlist per system in _playlists.",
        }
    }
}

/// Planned ops per rule id (rules without ops are left out).
pub fn hits(why: &[Why]) -> std::collections::BTreeMap<&'static str, usize> {
    let mut m = std::collections::BTreeMap::new();
    for w in why {
        *m.entry(w.rule.id()).or_insert(0) += 1;
    }
    m
}

/// Reason for one planned op: the rule plus case-specific detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Why {
    pub rule: Rule,
    pub detail: String,
}

impl Why {
    pub fn new(rule: Rule, detail: impl Into<String>) -> Self {
        Self {
            rule,
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for Why {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.detail.is_empty() {
            f.write_str(self.rule.title())
        } else {
            write!(f, "{} – {}", self.rule.title(), self.detail)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_unique_and_documented() {
        let ids: std::collections::HashSet<_> = Rule::ALL.iter().map(|r| r.id()).collect();
        assert_eq!(ids.len(), Rule::ALL.len());
        assert!(Rule::ALL.iter().all(|r| !r.explain().is_empty()));
        assert_eq!(
            serde_json::to_string(&Rule::G1rPick).unwrap(),
            "\"g1r-pick\""
        );
    }
}
