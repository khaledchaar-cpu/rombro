//! ScummVM targets in the core's `scummvm.ini`. Without a target the core detects the game
//! folder on every start and stops at "Multiple targets found" when files fit several
//! variants (e.g. DOS CD and FM Towns). A target named like the launcher's id with engine,
//! game id and path makes it start directly; ScummVM fills in platform and language itself.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Library folder of ScummVM games.
pub const SYSTEM: &str = "ScummVM";
pub const INI: &str = "scummvm.ini";

const ENGINES: &str = include_str!("../../data/scummvm-engines.tsv");

/// A game folder in the library with its launcher id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub id: String,
    pub engine: String,
    pub path: PathBuf,
    pub description: String,
}

/// Game id → engine; ids listed for several engines are left out (ambiguous).
fn engines() -> HashMap<&'static str, Option<&'static str>> {
    let mut map: HashMap<&str, Option<&str>> = HashMap::new();
    for line in ENGINES.lines().filter(|l| !l.starts_with('#')) {
        let Some((id, engine)) = line.split_once('\t') else {
            continue;
        };
        map.entry(id)
            .and_modify(|e| *e = None)
            .or_insert(Some(engine));
    }
    map
}

/// Targets for every `<lib>/ScummVM/<Game>/` holding a launcher with a known engine. The id
/// is the launcher's content (`engine:id` names the engine), else its file name.
pub fn targets(library: &Path) -> Vec<Target> {
    let engines = engines();
    let Ok(rd) = std::fs::read_dir(library.join(SYSTEM)) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    for dir in dirs {
        let Some(launcher) = launcher(&dir) else {
            continue;
        };
        let content = std::fs::read_to_string(&launcher).unwrap_or_default();
        let content = content.trim();
        let stem = launcher
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase());
        let raw = if crate::scummvm::repaired_id(&launcher, content.as_bytes()).is_none()
            && !content.is_empty()
        {
            content.to_owned()
        } else {
            stem.unwrap_or_default()
        };
        let (engine, id) = match raw.split_once(':') {
            Some((e, id)) => (Some(e.to_owned()), id.to_owned()),
            None => (None, raw),
        };
        let Some(engine) = engine.or_else(|| {
            engines
                .get(id.as_str())
                .copied()
                .flatten()
                .map(str::to_owned)
        }) else {
            continue;
        };
        let description = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        out.push(Target {
            id,
            engine,
            path: dir,
            description,
        });
    }
    out
}

/// The single `*.scummvm` file directly in a game folder.
fn launcher(dir: &Path) -> Option<PathBuf> {
    let mut found = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("scummvm"))
        });
    let first = found.next()?;
    found.next().is_none().then_some(first)
}

/// Section of the ini: name and the line range of its body.
struct Section {
    name: String,
    start: usize,
    end: usize,
}

fn sections(lines: &[&str]) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if let Some(name) = l.trim().strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if let Some(last) = out.last_mut() {
                last.end = i;
            }
            out.push(Section {
                name: name.to_owned(),
                start: i + 1,
                end: lines.len(),
            });
        }
    }
    out
}

/// Adds missing targets and moves the path of targets that point into the library (game
/// moved); targets pointing elsewhere are the user's and stay. Returns the new ini text and
/// the ids written, or `None` if nothing changes. One target per id: the first folder wins.
pub fn merge(ini: &str, targets: &[Target], library: &Path) -> Option<(String, Vec<String>)> {
    let lines: Vec<&str> = ini.lines().collect();
    let secs = sections(&lines);
    let mut out: Vec<String> = lines.iter().map(|l| (*l).to_owned()).collect();
    let mut appended = String::new();
    let mut written = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for t in targets {
        if !seen.insert(t.id.as_str()) || t.id == "scummvm" {
            continue;
        }
        let path = t.path.to_string_lossy();
        match secs.iter().find(|s| s.name == t.id) {
            Some(s) => {
                let line = (s.start..s.end).find(|&i| lines[i].trim_start().starts_with("path="));
                let Some(i) = line else { continue };
                let old = lines[i].trim_start().trim_start_matches("path=");
                if old != path && Path::new(old).starts_with(library) {
                    out[i] = format!("path={path}");
                    written.push(t.id.clone());
                }
            }
            None => {
                appended.push_str(&format!(
                    "\n[{}]\ndescription={}\nengineid={}\ngameid={}\npath={path}\n",
                    t.id, t.description, t.engine, t.id
                ));
                written.push(t.id.clone());
            }
        }
    }
    if written.is_empty() {
        return None;
    }
    let mut text = out.join("\n");
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&appended);
    Some((text, written))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn game(lib: &Path, folder: &str, launcher: &str, content: &str) {
        let d = lib.join(SYSTEM).join(folder);
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join(launcher), content).unwrap();
        fs::write(d.join("DATA.000"), "x").unwrap();
    }

    #[test]
    fn targets_take_id_from_content_or_name_and_engine_from_table() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path();
        game(lib, "Atlantis", "indy4.scummvm", "atlantis\n");
        game(lib, "Monkey 2", "monkey2.scummvm", "{\\rtf1}");
        game(lib, "Sky", "sky.scummvm", "sky:sky");
        game(lib, "Unknown", "zzzz.scummvm", "zzzz");
        let t = targets(lib);
        let got: Vec<_> = t
            .iter()
            .map(|t| (t.id.as_str(), t.engine.as_str()))
            .collect();
        assert_eq!(
            got,
            [("atlantis", "scumm"), ("monkey2", "scumm"), ("sky", "sky")]
        );
    }

    #[test]
    fn merge_adds_missing_and_moves_own_targets_only() {
        let lib = Path::new("/lib");
        let t = |id: &str, dir: &str| Target {
            id: id.into(),
            engine: "scumm".into(),
            path: lib.join(SYSTEM).join(dir),
            description: dir.into(),
        };
        let ini = "[scummvm]\nversioninfo=1\n\n[dig]\ngameid=dig\npath=/lib/ScummVM/Old Dig\n\n[mine]\npath=/home/me/mine\n";
        let (text, ids) = merge(
            ini,
            &[
                t("dig", "The Dig"),
                t("mine", "Mine"),
                t("monkey2", "Monkey 2"),
            ],
            lib,
        )
        .unwrap();
        assert_eq!(ids, ["dig", "monkey2"]);
        assert!(text.contains("[dig]\ngameid=dig\npath=/lib/ScummVM/The Dig\n"));
        assert!(text.contains("path=/home/me/mine"));
        assert!(text.ends_with("[monkey2]\ndescription=Monkey 2\nengineid=scumm\ngameid=monkey2\npath=/lib/ScummVM/Monkey 2\n"));
        assert_eq!(merge(&text, &[t("dig", "The Dig")], lib), None);
    }
}
