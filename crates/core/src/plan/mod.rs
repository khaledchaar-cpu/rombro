//! Planner: turns identified files into a reviewable list of file operations (SPEC F4).
//! Plan → dry run → execute (journal) → undo.

#[cfg(test)]
mod archive_tests;
mod build;
pub mod lpl;
mod ops;
#[cfg(test)]
mod rules_tests;
mod sheet;
#[cfg(test)]
mod tests;
pub mod trash;

pub use build::{FOLDER_SYSTEMS, build};
pub use ops::{Done, Execution, Op, execute, journal_from_json, journal_to_json, undo};

use crate::g1r::Rules;
use std::collections::HashMap;
use std::path::PathBuf;

/// How files from the inbox get into the library. Files already in the library are always moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Move,
    Copy,
    Hardlink,
    Reflink,
}

/// Files making up one importable unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Files {
    /// A ROM, a single-ROM archive or a disc image file (`.iso`).
    Single(PathBuf),
    /// A disc sheet (`.cue`/`.gdi`) and its track files.
    Sheet {
        sheet: PathBuf,
        tracks: Vec<PathBuf>,
    },
    /// An arcade romset archive, identified as a whole; placed as-is under its own name,
    /// together with the CHDs in the folder of the same name (`kinst.zip` + `kinst/*.chd`).
    Set {
        archive: PathBuf,
        chds: Vec<PathBuf>,
        /// Further arcade systems listing this exact set, best first: used when another
        /// version of the same short name already takes the preferred system's slot.
        alt: Vec<Game>,
        /// DAT check outcome when better-ranked cores were skipped (shown as rule detail).
        dat_note: String,
    },
    /// One ROM inside an archive holding several (extracted on import).
    Member { archive: PathBuf, member: String },
    /// A disc stored in an archive: sheet member and its track members (in sheet order).
    ArchivedSheet {
        archive: PathBuf,
        sheet: String,
        tracks: Vec<String>,
    },
}

impl Files {
    pub fn primary(&self) -> &PathBuf {
        match self {
            Files::Single(p)
            | Files::Set { archive: p, .. }
            | Files::Sheet { sheet: p, .. }
            | Files::Member { archive: p, .. }
            | Files::ArchivedSheet { archive: p, .. } => p,
        }
    }

    /// The archive this item is extracted from, if any.
    pub fn archive(&self) -> Option<&PathBuf> {
        match self {
            Files::Member { archive, .. } | Files::ArchivedSheet { archive, .. } => Some(archive),
            _ => None,
        }
    }

    pub fn all(&self) -> Vec<&PathBuf> {
        match self {
            Files::Single(p) | Files::Member { archive: p, .. } => vec![p],
            Files::Set { archive, chds, .. } => std::iter::once(archive).chain(chds).collect(),
            Files::ArchivedSheet { archive, .. } => vec![archive],
            Files::Sheet { sheet, tracks } => std::iter::once(sheet).chain(tracks).collect(),
        }
    }
}

/// A database entry an item was matched to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    pub system: String,
    pub name: String,
    pub crc: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ident {
    Known(Game),
    /// Several different games match; the user decides (SPEC §11a).
    Ambiguous(Vec<Game>),
    Unknown,
    /// A BIOS set (arcade or console `[BIOS]` entry): goes to the BIOS folder, ignored everywhere else.
    Bios(Game),
    /// Not handled by the planner (reason shown to the user).
    Skip(String),
    /// Arcade set matched by a database but incomplete for every core's DAT (SPEC F8):
    /// quarantined with the reason.
    Incomplete(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub files: Files,
    pub ident: Ident,
    /// Already inside the library (audit) rather than the inbox.
    pub in_library: bool,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub mode: Mode,
    pub rules: Rules,
    /// Directory for RetroArch playlists; `None` disables the export.
    pub playlists: Option<PathBuf>,
    /// User verdicts on releases 1G1R rejected, keyed by (system, name).
    pub verdicts: HashMap<(String, String), Verdict>,
    /// Inbox root; quarantined files keep their path below it (no clashes on equal names).
    pub inbox: Option<PathBuf>,
    /// Paths the user excluded: nothing at or below them is planned (left as is).
    pub ignore: Vec<PathBuf>,
}

/// What to do with a release 1G1R rejected (TBD queue).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Place it in the library next to the pick.
    Keep,
    /// Move it to `<library>/_trash` (undoable).
    Discard,
    /// Resolves a 1G1R tie in favour of this release (keyed by release name, without disc tag).
    Prefer,
}

/// Something the planner left untouched and the user should look at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Ambiguous {
        path: PathBuf,
        candidates: Vec<Game>,
    },
    /// 1G1R tie: no rule decides between these releases (release names, discs merged).
    Tie {
        system: String,
        releases: Vec<String>,
    },
    /// Not picked by 1G1R; stays in place until the user decides (keep, delete, ...).
    Rejected {
        path: PathBuf,
        system: String,
        name: String,
        /// The release picked instead (none if every release of the game is excluded).
        kept: Option<String>,
        reason: String,
    },
    Skipped {
        path: PathBuf,
        reason: String,
    },
    /// Target exists (or is claimed twice) – the item is left in place.
    Conflict {
        path: PathBuf,
        target: PathBuf,
    },
}

#[derive(Debug, Default)]
pub struct Plan {
    pub ops: Vec<Op>,
    /// Why each op is planned (parallel to `ops`).
    pub why: Vec<crate::rules::Why>,
    pub decisions: Vec<Decision>,
    /// Items already correctly placed.
    pub unchanged: usize,
    pub placed: usize,
    pub quarantined: usize,
    pub discarded: usize,
}

/// Library sub-folders the planner manages itself; their content is never re-planned.
pub const QUARANTINE_DIR: &str = "_quarantine";
/// Default playlist folder inside the library.
pub const PLAYLIST_DIR: &str = "_playlists";
/// Arcade BIOS sets, laid out like RetroArch's `system` folder.
pub const BIOS_DIR: &str = "_bios";
/// Releases the user discarded from the TBD queue.
pub const TRASH_DIR: &str = "_trash";
