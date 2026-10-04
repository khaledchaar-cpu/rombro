//! Planner: turns identified files into a reviewable list of file operations (SPEC F4).
//! Plan → dry run → execute (journal) → undo.

mod build;
pub mod lpl;
mod ops;
#[cfg(test)]
mod tests;

pub use build::build;
pub use ops::{Done, Execution, Op, execute, undo};

use crate::g1r::Rules;
use std::path::PathBuf;

/// How files from the inbox get into the library. Files already in the library are always moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Move,
    Copy,
    Hardlink,
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
}

impl Files {
    pub fn primary(&self) -> &PathBuf {
        match self {
            Files::Single(p) | Files::Sheet { sheet: p, .. } => p,
        }
    }

    pub fn all(&self) -> Vec<&PathBuf> {
        match self {
            Files::Single(p) => vec![p],
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
    /// Not handled by the planner (reason shown to the user).
    Skip(String),
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
    /// Name of this run's trash folder (`_trash/<stamp>`).
    pub stamp: String,
    /// Directory for RetroArch playlists; `None` disables the export.
    pub playlists: Option<PathBuf>,
}

/// Something the planner left untouched and the user should look at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Ambiguous {
        path: PathBuf,
        candidates: Vec<Game>,
    },
    /// 1G1R tie: no rule decides between these releases.
    Tie {
        system: String,
        releases: Vec<String>,
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
    pub decisions: Vec<Decision>,
    /// Items already correctly placed.
    pub unchanged: usize,
    pub placed: usize,
    pub trashed: usize,
    pub quarantined: usize,
}

/// Library sub-folders the planner manages itself; their content is never re-planned.
pub const TRASH_DIR: &str = "_trash";
pub const QUARANTINE_DIR: &str = "_quarantine";
