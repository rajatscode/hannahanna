use clap::{Parser, Subcommand};

mod cli;
mod errors;
mod vcs;

use errors::Result;

#[derive(Parser)]
#[command(name = "hn")]
#[command(about = "Git worktree manager with isolated development environments", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all worktrees
    List,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => cli::list::run()?,
    }

    Ok(())
}
