mod db;
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
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Db { cmd } => db::run(cmd),
        Cmd::Scan { dir, db, unknown } => scan::run(dir, db, unknown),
    }
}
