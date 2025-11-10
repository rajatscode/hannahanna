use clap::{Parser, Subcommand};

mod cli;
mod config;
mod env;
mod errors;
mod fuzzy;
mod hooks;
mod state;
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
        /// Base branch to create from (defaults to current branch)
        #[arg(long)]
        from: Option<String>,
        /// Checkout existing branch instead of creating new one
        #[arg(long)]
        no_branch: bool,
    },
    /// List all worktrees
    List {
        /// Show parent/child tree view
        #[arg(long)]
        tree: bool,
    },
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
    /// Show detailed information about a worktree
    Info {
        /// Name of the worktree (defaults to current)
        name: Option<String>,
    },
    /// Output shell integration code for ~/.bashrc or ~/.zshrc
    InitShell,
    /// Clean up orphaned state directories
    Prune,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { name, branch, from, no_branch } => cli::add::run(name, branch, from, no_branch)?,
        Commands::List { tree } => cli::list::run(tree)?,
        Commands::Remove { name, force } => cli::remove::run(name, force)?,
        Commands::Switch { name } => cli::switch::run(name)?,
        Commands::Info { name } => cli::info::run(name)?,
        Commands::InitShell => cli::init_shell::run()?,
        Commands::Prune => cli::prune::run()?,
    }

    Ok(())
}
