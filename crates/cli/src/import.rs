use crate::db::open_store;
use anyhow::{Context, Result, bail};
use rombro_core::plan::{self, Decision, Mode, Op, Options, PLAYLIST_DIR};
use rombro_store::{DiscMatch, Match, Record, candidates};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Args {
    pub inbox: Option<PathBuf>,
    pub library: PathBuf,
    pub dry_run: bool,
    pub mode: Mode,
    pub playlists: Option<PathBuf>,
    pub no_playlists: bool,
    pub db: Option<PathBuf>,
}

/// Scans library (+ inbox), plans, prints the plan and executes it unless `dry_run`.
pub fn run(a: Args) -> Result<()> {
    let store = open_store(a.db)?;
    let library = absolute(&a.library)?;
    let mut items = Vec::new();
    if library.is_dir() {
        items = store.items(&scan(&library), true)?;
    }
    let inbox = a.inbox.as_deref().map(absolute).transpose()?;
    if let Some(inbox) = &inbox {
        items.extend(store.items(&scan(inbox), false)?);
    }
    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
    let opts = Options {
        mode: a.mode,
        rules: store.rules()?,
        playlists: (!a.no_playlists)
            .then(|| a.playlists.unwrap_or_else(|| library.join(PLAYLIST_DIR))),
        verdicts: store.verdicts()?,
        inbox,
    };
    let p = plan::build(&items, &library, &opts);
    for op in &p.ops {
        println!("{}", describe(op, &library));
    }
    for d in &p.decisions {
        println!("{}", decision(d));
    }
    println!(
        "\n{} items: {} to place, {} unchanged, {} to quarantine, {} need attention; {} operations",
        items.len(),
        p.placed,
        p.unchanged,
        p.quarantined,
        p.decisions.len(),
        p.ops.len()
    );
    if a.dry_run || p.ops.is_empty() {
        return Ok(());
    }
    let ex = plan::execute(&p.ops);
    if !ex.done.is_empty() {
        let id = store.add_journal(
            ts,
            &library.to_string_lossy(),
            &plan::journal_to_json(&ex.done),
        )?;
        println!(
            "executed {} operations (journal #{id}; `rombro undo` reverts)",
            ex.done.len()
        );
    }
    if let Some((op, e)) = ex.error {
        bail!("stopped at {}: {e}", describe(&op, &library));
    }
    Ok(())
}

/// Reverts the most recent execution.
pub fn undo(db: Option<PathBuf>) -> Result<()> {
    let store = open_store(db)?;
    let Some(j) = store.last_journal()? else {
        println!("nothing to undo");
        return Ok(());
    };
    let done = plan::journal_from_json(&j.done).context("corrupt journal")?;
    let errors = plan::undo(&done);
    for (op, e) in &errors {
        eprintln!("ERROR   {}: {e}", describe(op, Path::new(&j.library)));
    }
    if errors.is_empty() {
        store.mark_undone(j.id)?;
        println!("reverted journal #{} ({} operations)", j.id, done.len());
        Ok(())
    } else {
        bail!(
            "{} operations could not be reverted; journal #{} kept",
            errors.len(),
            j.id
        )
    }
}

/// Lists the candidates of an ambiguous file, or stores choice `pick` (1-based).
pub fn resolve(file: PathBuf, pick: Option<usize>, db: Option<PathBuf>) -> Result<()> {
    let store = open_store(db)?;
    let report = scan(&file);
    let (records, sha1) = if let Some(d) = report.discs.first() {
        let r = match store.identify_disc(d)? {
            DiscMatch::Hash(Match::Verified(r) | Match::CrcOnly(r)) | DiscMatch::Serial(r) => r,
            _ => Vec::new(),
        };
        (r, d.tracks.first().map(|t| t.hashes.sha1))
    } else if let Some(rom) = report.roms.first() {
        (store.identify_rom(rom)?, Some(rom.hashes.sha1))
    } else {
        bail!("no ROM or disc found at {}", file.display());
    };
    let sha1 = sha1.context("disc without tracks")?;
    let c: Vec<&Record> = candidates(&records);
    match pick {
        None => {
            for (i, r) in c.iter().enumerate() {
                println!("{:>3}  [{}] {}", i + 1, r.system, r.name);
            }
            if let Some((system, name)) = store.resolution(&sha1)? {
                println!("current choice: [{system}] {name}");
            }
        }
        Some(n) => {
            let r = c.get(n.wrapping_sub(1)).context("no such candidate")?;
            store.set_resolution(&sha1, &r.system, &r.name)?;
            println!("resolved to [{}] {}", r.system, r.name);
        }
    }
    Ok(())
}

fn scan(dir: &Path) -> rombro_core::ScanReport {
    let report = rombro_core::scan(dir);
    for f in &report.failures {
        eprintln!("ERROR   {}: {}", f.path.display(), f.error);
    }
    report
}

fn absolute(p: &Path) -> Result<PathBuf> {
    Ok(std::path::absolute(p)?)
}

fn rel<'a>(p: &'a Path, base: &Path) -> std::path::Display<'a> {
    p.strip_prefix(base).unwrap_or(p).display()
}

fn describe(op: &Op, lib: &Path) -> String {
    match op {
        Op::Move { from, to } => format!("MOVE    {} -> {}", from.display(), rel(to, lib)),
        Op::Copy { from, to } => format!("COPY    {} -> {}", from.display(), rel(to, lib)),
        Op::Hardlink { from, to } => format!("LINK    {} -> {}", from.display(), rel(to, lib)),
        Op::Reflink { from, to } => format!("CLONE   {} -> {}", from.display(), rel(to, lib)),
        Op::Extract {
            archive,
            member,
            to,
        } => {
            format!("EXTRACT {}#{member} -> {}", archive.display(), rel(to, lib))
        }
        Op::Write { path, .. } => format!("WRITE   {}", rel(path, lib)),
    }
}

fn decision(d: &Decision) -> String {
    match d {
        Decision::Ambiguous { path, candidates } => {
            let mut s = format!("AMBIG   {} (`rombro resolve` to choose)", path.display());
            for g in candidates {
                s.push_str(&format!("\n          ? [{}] {}", g.system, g.name));
            }
            s
        }
        Decision::Tie { system, releases } => {
            format!("TIE     [{system}] {}", releases.join(" | "))
        }
        Decision::Rejected {
            path,
            system: _,
            name,
            kept,
            reason,
        } => match kept {
            Some(k) => format!(
                "TBD     {} [{name}] not 1G1R ({reason}); kept: {k}",
                path.display()
            ),
            None => format!("TBD     {} [{name}] excluded ({reason})", path.display()),
        },
        Decision::Skipped { path, reason } => format!("SKIP    {} ({reason})", path.display()),
        Decision::Conflict { path, target } => {
            format!("CONFLICT {} -> {} exists", path.display(), target.display())
        }
    }
}
