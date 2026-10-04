mod db;
mod g1r;
mod scan;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "rombro",
    version,
    about = "ROM curator based on RetroArch databases"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// RetroArch database commands
    Db {
        #[command(subcommand)]
        cmd: db::DbCmd,
    },
    /// Scan a directory and verify ROMs against the database
    Scan {
        dir: PathBuf,
        /// Database file (default: $XDG_DATA_HOME/rombro/rombro.db)
        #[arg(long)]
        db: Option<PathBuf>,
        /// Only list unknown files
        #[arg(long)]
        unknown: bool,
    },
    /// Show 1G1R picks for a system from the database (default rules)
    G1r {
        /// System name as in the RDB, e.g. "Nintendo - Super Nintendo Entertainment System"
        system: String,
        /// Only show groups whose key contains this text
        #[arg(long)]
        filter: Option<String>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Db { cmd } => db::run(cmd),
        Cmd::Scan { dir, db, unknown } => scan::run(dir, db, unknown),
        Cmd::G1r { system, filter, db } => g1r::run(&system, filter.as_deref(), db),
    }
}
