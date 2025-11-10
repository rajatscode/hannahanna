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
    /// Create a new worktree
    Add {
        /// Name of the worktree to create
        name: String,
        /// Branch to checkout (defaults to creating new branch with same name)
        branch: Option<String>,
    },
    /// List all worktrees
    List,
    /// Remove a worktree
    Remove {
        /// Name of the worktree to remove
        name: String,
        /// Force removal even if there are uncommitted changes
        #[arg(short, long)]
        force: bool,
    },
    /// Switch to a worktree (outputs path for shell wrapper)
    Switch {
        /// Name of the worktree to switch to
        name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { name, branch } => cli::add::run(name, branch)?,
        Commands::List => cli::list::run()?,
        Commands::Remove { name, force } => cli::remove::run(name, force)?,
        Commands::Switch { name } => cli::switch::run(name)?,
    }

    Ok(())
}
