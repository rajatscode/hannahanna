use clap::{Parser, Subcommand};

mod cli;
mod config;
mod docker;
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
    /// Return to parent worktree with optional merge
    Return {
        /// Merge current branch into parent before returning
        #[arg(long)]
        merge: bool,
        /// Delete current worktree after merging (requires --merge)
        #[arg(long)]
        delete: bool,
        /// Force merge commit (no fast-forward)
        #[arg(long)]
        no_ff: bool,
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
    /// Manage Docker port allocations
    Ports {
        #[command(subcommand)]
        command: PortsCommands,
    },
    /// Manage Docker containers
    Docker {
        #[command(subcommand)]
        command: DockerCommands,
    },
}

#[derive(Subcommand)]
enum PortsCommands {
    /// List all port allocations
    List,
    /// Show port allocations for a specific worktree
    Show {
        /// Name of the worktree
        name: String,
    },
    /// Release port allocations for a worktree
    Release {
        /// Name of the worktree
        name: String,
    },
}

#[derive(Subcommand)]
enum DockerCommands {
    /// Show container status for all worktrees
    Ps,
    /// Start containers for a worktree
    Start {
        /// Name of the worktree
        name: String,
    },
    /// Stop containers for a worktree
    Stop {
        /// Name of the worktree
        name: String,
    },
    /// View logs for a worktree's containers
    Logs {
        /// Name of the worktree
        name: String,
        /// Optional service name
        service: Option<String>,
    },
    /// Clean up orphaned containers
    Prune,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add {
            name,
            branch,
            from,
            no_branch,
        } => cli::add::run(name, branch, from, no_branch)?,
        Commands::List { tree } => cli::list::run(tree)?,
        Commands::Remove { name, force } => cli::remove::run(name, force)?,
        Commands::Switch { name } => cli::switch::run(name)?,
        Commands::Return { merge, delete, no_ff } => cli::return_cmd::run(merge, delete, no_ff)?,
        Commands::Info { name } => cli::info::run(name)?,
        Commands::InitShell => cli::init_shell::run()?,
        Commands::Prune => cli::prune::run()?,
        Commands::Ports { command } => match command {
            PortsCommands::List => cli::ports::list()?,
            PortsCommands::Show { name } => cli::ports::show(name)?,
            PortsCommands::Release { name } => cli::ports::release(name)?,
        },
        Commands::Docker { command } => match command {
            DockerCommands::Ps => cli::docker::ps()?,
            DockerCommands::Start { name } => cli::docker::start(name)?,
            DockerCommands::Stop { name } => cli::docker::stop(name)?,
            DockerCommands::Logs { name, service } => cli::docker::logs(name, service)?,
            DockerCommands::Prune => cli::docker::prune()?,
        },
    }

    Ok(())
}
