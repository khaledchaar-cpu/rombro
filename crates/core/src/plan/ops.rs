//! File operations: execution with a journal of what was done, and undo.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One planned file-system operation. Paths are absolute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    Move {
        from: PathBuf,
        to: PathBuf,
    },
    Copy {
        from: PathBuf,
        to: PathBuf,
    },
    Hardlink {
        from: PathBuf,
        to: PathBuf,
    },
    /// Writes a text file; replaces an existing file (its content is journaled for undo).
    Write {
        path: PathBuf,
        contents: String,
    },
}

impl Op {
    /// The path this operation creates or overwrites.
    pub fn target(&self) -> &Path {
        match self {
            Op::Move { to, .. } | Op::Copy { to, .. } | Op::Hardlink { to, .. } => to,
            Op::Write { path, .. } => path,
        }
    }

    pub fn source(&self) -> Option<&Path> {
        match self {
            Op::Move { from, .. } | Op::Copy { from, .. } | Op::Hardlink { from, .. } => Some(from),
            Op::Write { .. } => None,
        }
    }
}

/// A completed operation plus everything needed to revert it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Done {
    pub op: Op,
    /// Directories created for the target, outermost first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub created_dirs: Vec<PathBuf>,
    /// Previous content of a file overwritten by `Op::Write`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced: Option<String>,
}

/// Result of executing a plan: completed operations and, if it stopped early, why.
#[derive(Debug, Default)]
pub struct Execution {
    pub done: Vec<Done>,
    pub error: Option<(Op, io::Error)>,
}

/// Executes operations in order and stops at the first failure.
/// Never overwrites existing files except via `Op::Write`.
pub fn execute(ops: &[Op]) -> Execution {
    let mut ex = Execution::default();
    for op in ops {
        match apply(op) {
            Ok(d) => ex.done.push(d),
            Err(e) => {
                ex.error = Some((op.clone(), e));
                break;
            }
        }
    }
    ex
}

fn apply(op: &Op) -> io::Result<Done> {
    let target = op.target();
    let replaced = match op {
        Op::Write { path, .. } if path.exists() => Some(fs::read_to_string(path)?),
        Op::Write { .. } => None,
        _ if target.exists() => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} already exists", target.display()),
            ));
        }
        _ => None,
    };
    let created_dirs = create_parents(target)?;
    let res = match op {
        Op::Move { from, to } => move_file(from, to),
        Op::Copy { from, to } => fs::copy(from, to).map(|_| ()),
        Op::Hardlink { from, to } => fs::hard_link(from, to),
        Op::Write { path, contents } => fs::write(path, contents),
    };
    if let Err(e) = res {
        remove_dirs(&created_dirs);
        return Err(e);
    }
    Ok(Done {
        op: op.clone(),
        created_dirs,
        replaced,
    })
}

/// Reverts completed operations in reverse order. Continues past failures and returns them.
pub fn undo(done: &[Done]) -> Vec<(Op, io::Error)> {
    let mut errors = Vec::new();
    for d in done.iter().rev() {
        let res = match (&d.op, &d.replaced) {
            (Op::Move { from, to }, _) => create_parents(from).and_then(|_| move_file(to, from)),
            (Op::Copy { to, .. } | Op::Hardlink { to, .. }, _) => fs::remove_file(to),
            (Op::Write { path, .. }, Some(old)) => fs::write(path, old),
            (Op::Write { path, .. }, None) => fs::remove_file(path),
        };
        match res {
            Ok(()) => remove_dirs(&d.created_dirs),
            Err(e) => errors.push((d.op.clone(), e)),
        }
    }
    errors
}

/// Rename, falling back to copy + delete across file systems.
fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            fs::copy(from, to)?;
            fs::remove_file(from)
        }
        Err(e) => Err(e),
    }
}

fn create_parents(path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut missing = Vec::new();
    let mut p = path.parent();
    while let Some(dir) = p {
        if dir.as_os_str().is_empty() || dir.exists() {
            break;
        }
        missing.push(dir.to_path_buf());
        p = dir.parent();
    }
    missing.reverse();
    for d in &missing {
        fs::create_dir(d)?;
    }
    Ok(missing)
}

/// Removes created directories innermost first; non-empty ones are kept.
fn remove_dirs(dirs: &[PathBuf]) {
    for d in dirs.iter().rev() {
        let _ = fs::remove_dir(d);
    }
}
