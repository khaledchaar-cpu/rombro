mod db;
mod g1r;
mod import;
mod play;
mod ra;
mod rules;
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
    /// Import an inbox into the library (1G1R, renaming, quarantine, playlists)
    Import {
        inbox: PathBuf,
        library: PathBuf,
        #[command(flatten)]
        opts: PlanOpts,
    },
    /// Check an existing library and plan fixes (renames, duplicates, non-1G1R, unknown files)
    Audit {
        library: PathBuf,
        #[command(flatten)]
        opts: PlanOpts,
    },
    /// Show planner rules (explanation, ops in the last plan, settings) or load settings
    Rules {
        /// JSON file with rules to store
        #[arg(long)]
        set: Option<PathBuf>,
        /// Never touch this path when planning (repeatable)
        #[arg(long)]
        ignore: Vec<PathBuf>,
        /// Remove a path from the ignore list (repeatable)
        #[arg(long)]
        unignore: Vec<PathBuf>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Install, update and configure the RetroArch rombro manages
    Ra {
        #[command(subcommand)]
        cmd: ra::Cmd,
    },
    /// Start a game in the managed RetroArch (installs RetroArch and the core if missing)
    Play(play::Args),
    /// Revert the most recent import/audit execution
    Undo {
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// List candidates of an ambiguous file, or choose one by number
    Resolve {
        file: PathBuf,
        pick: Option<usize>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
}

#[derive(clap::Args)]
struct PlanOpts {
    /// Only show the plan
    #[arg(long)]
    dry_run: bool,
    /// How inbox files get into the library
    #[arg(long, value_enum, default_value_t = ModeArg::Move)]
    mode: ModeArg,
    /// RetroArch playlist directory (default: <library>/_playlists)
    #[arg(long)]
    playlists: Option<PathBuf>,
    /// Do not write playlists
    #[arg(long)]
    no_playlists: bool,
    #[arg(long)]
    db: Option<PathBuf>,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum ModeArg {
    Move,
    Copy,
}

impl PlanOpts {
    fn args(self, inbox: Option<PathBuf>, library: PathBuf) -> import::Args {
        import::Args {
            inbox,
            library,
            dry_run: self.dry_run,
            mode: match self.mode {
                ModeArg::Move => rombro_core::plan::Mode::Move,
                ModeArg::Copy => rombro_core::plan::Mode::Copy,
            },
            playlists: self.playlists,
            no_playlists: self.no_playlists,
            db: self.db,
        }
    }
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Db { cmd } => db::run(cmd),
        Cmd::Scan { dir, db, unknown } => scan::run(dir, db, unknown),
        Cmd::G1r { system, filter, db } => g1r::run(&system, filter.as_deref(), db),
        Cmd::Import {
            inbox,
            library,
            opts,
        } => import::run(opts.args(Some(inbox), library)),
        Cmd::Audit { library, opts } => import::run(opts.args(None, library)),
        Cmd::Rules {
            set,
            ignore,
            unignore,
            db,
        } => rules::run(set, &ignore, &unignore, db),
        Cmd::Ra { cmd } => ra::run(cmd),
        Cmd::Play(a) => play::run(a),
        Cmd::Undo { db } => import::undo(db),
        Cmd::Resolve { file, pick, db } => import::resolve(file, pick, db),
    }
}
