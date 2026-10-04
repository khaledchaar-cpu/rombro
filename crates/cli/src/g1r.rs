use crate::db::open_store;
use anyhow::Result;
use rombro_core::g1r::{Rules, select};
use std::path::PathBuf;

/// Prints the 1G1R selection for one system; with a filter, every candidate and reason.
pub fn run(system: &str, filter: Option<&str>, db: Option<PathBuf>) -> Result<()> {
    let store = open_store(db)?;
    let mut entries = store.by_system(system)?;
    entries.retain(|r| !r.name.is_empty());
    let picks = select(&entries, |r| &r.name, &Rules::default());
    let filter = filter.map(str::to_lowercase);
    let (mut games, mut none, mut ties) = (0, 0, 0);
    for g in &picks {
        if g.picked.is_empty() {
            none += 1;
        } else {
            games += 1;
        }
        ties += usize::from(g.needs_decision);
        if filter.as_ref().is_some_and(|f| g.key.contains(f.as_str())) {
            let tie = if g.needs_decision {
                "  [? needs decision]"
            } else {
                ""
            };
            println!("{}{tie}", g.key);
            for p in &g.picked {
                println!("  + {}", p.name);
            }
            for (r, why) in &g.rejected {
                println!("  - {} ({why:?})", r.name);
            }
        }
    }
    println!(
        "\n{system}: {} entries → {games} games picked, {none} groups without an eligible release, {ties} need a decision",
        entries.len()
    );
    Ok(())
}
