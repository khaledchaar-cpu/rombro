mod db;

use clap::{Parser, Subcommand};

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
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Db { cmd } => db::run(cmd),
    }
}
