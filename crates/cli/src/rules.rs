//! `rombro rules`: planner rules with explanation, last-plan hits and current settings.
use crate::db::open_store;
use anyhow::{Context, Result};
use rombro_core::g1r::Rules;
use rombro_core::rules::Rule;
use std::path::PathBuf;

pub fn run(
    set: Option<PathBuf>,
    ignore: &[PathBuf],
    unignore: &[PathBuf],
    db: Option<PathBuf>,
) -> Result<()> {
    let store = open_store(db)?;
    if !ignore.is_empty() || !unignore.is_empty() {
        let mut list = store.ignored()?;
        for p in ignore {
            let p = std::path::absolute(p)?;
            if !list.contains(&p) {
                list.push(p);
            }
        }
        let gone: Vec<PathBuf> = unignore
            .iter()
            .map(std::path::absolute)
            .collect::<std::io::Result<_>>()?;
        list.retain(|p| !gone.contains(p));
        store.set_ignored(&list)?;
        println!("{} ignored path(s)", list.len());
        return Ok(());
    }
    if let Some(file) = set {
        let text = std::fs::read_to_string(&file)
            .with_context(|| format!("reading {}", file.display()))?;
        let rules: Rules = serde_json::from_str(&text).context("invalid rules JSON")?;
        store.set_rules(&rules)?;
        println!("rules saved; they apply to the next plan");
        return Ok(());
    }
    let hits = store.rule_hits()?;
    for r in Rule::ALL {
        let n = hits.get(r.id()).copied().unwrap_or(0);
        println!("{:<20} {:>7} ops  {}", r.id(), n, r.title());
        println!("    {}", r.explain());
    }
    println!("\nsettings (edit and load with `rombro rules --set <file>`):");
    println!("{}", serde_json::to_string_pretty(&store.rules()?)?);
    let ignored = store.ignored()?;
    if !ignored.is_empty() {
        println!("\nignored paths (`--ignore` / `--unignore`):");
        for p in ignored {
            println!("  {}", p.display());
        }
    }
    Ok(())
}
