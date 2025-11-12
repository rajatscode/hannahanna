use clap::{Parser, Subcommand};

mod cli;
mod clock;
mod config;
mod docker;
mod env;
mod errors;
mod fuzzy;
mod hooks;
mod state;
mod suggestions;
mod vcs;

#[derive(Parser)]
#[command(name = "hn")]
#[command(about = "Multi-VCS worktree manager with isolated development environments", long_about = None)]
#[command(version)]
struct Cli {
    /// Skip hook execution (for untrusted repositories)
    #[arg(long, global = true)]
    no_hooks: bool,

    /// Specify VCS type explicitly (git, hg/mercurial, jj/jujutsu). Auto-detects if not specified.
    #[arg(long, global = true, value_name = "TYPE")]
    vcs: Option<String>,

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
        /// Sparse checkout paths (can be specified multiple times)
        /// Example: --sparse services/api/ --sparse libs/utils/
        #[arg(long)]
        sparse: Option<Vec<String>>,
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
    /// Execute a command in each worktree
    Each {
        /// Command to execute (everything after 'each')
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        command: Vec<String>,
        /// Execute commands in parallel
        #[arg(long)]
        parallel: bool,
        /// Stop on first error
        #[arg(long)]
        stop_on_error: bool,
        /// Filter worktrees by name pattern (regex)
        #[arg(long)]
        filter: Option<String>,
        /// Only run on worktrees with Docker containers running
        #[arg(long)]
        docker_running: bool,
    },
    /// Merge a source worktree/branch into a target worktree/branch
    Integrate {
        /// Source worktree name or branch name
        source: String,
        /// Target worktree name (defaults to current)
        #[arg(long)]
        into: Option<String>,
        /// Force merge commit (no fast-forward)
        #[arg(long)]
        no_ff: bool,
        /// Squash commits before merging
        #[arg(long)]
        squash: bool,
        /// Merge strategy (e.g., 'recursive', 'ours', 'theirs')
        #[arg(long)]
        strategy: Option<String>,
    },
    /// Sync current worktree with another branch
    Sync {
        /// Source branch to sync with (defaults to 'main')
        source_branch: Option<String>,
        /// Sync strategy: 'merge' or 'rebase' (defaults to 'merge')
        #[arg(long)]
        strategy: Option<String>,
        /// Automatically stash and unstash changes
        #[arg(long)]
        autostash: bool,
        /// Don't automatically commit after merge
        #[arg(long)]
        no_commit: bool,
    },
    /// Output shell integration code for ~/.bashrc or ~/.zshrc
    InitShell,
    /// Clean up orphaned state directories
    Prune,
    /// Manage state directories
    State {
        #[command(subcommand)]
        command: StateCommands,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
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
enum StateCommands {
    /// List all state directories
    List,
    /// Clean orphaned state directories
    Clean,
    /// Show state directory sizes
    Size {
        /// Name of specific worktree (optional)
        name: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Create a new configuration file
    Init,
    /// Validate configuration file
    Validate,
    /// Show current configuration
    Show,
    /// Edit configuration file in $EDITOR
    Edit,
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
    /// Restart containers for a worktree
    Restart {
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
    /// Execute a command in a worktree's container
    Exec {
        /// Name of the worktree
        name: String,
        /// Service name (optional, defaults to first service)
        #[arg(long)]
        service: Option<String>,
        /// Command to execute
        command: Vec<String>,
    },
    /// Clean up orphaned containers
    Prune,
}

fn main() {
    let cli = Cli::parse();

    // Parse VCS type if provided
    let vcs_type = if let Some(ref vcs_str) = cli.vcs {
        match vcs_str.parse::<vcs::VcsType>() {
            Ok(vcs) => Some(vcs),
            Err(e) => {
                eprintln!("Error: {}", e);
                eprintln!("Supported VCS types: git, hg/mercurial, jj/jujutsu");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let result = match cli.command {
        Commands::Add {
            name,
            branch,
            from,
            no_branch,
            sparse,
        } => cli::add::run(name, branch, from, no_branch, sparse, cli.no_hooks, vcs_type),
        Commands::List { tree } => cli::list::run(tree, vcs_type),
        Commands::Remove { name, force } => cli::remove::run(name, force, cli.no_hooks, vcs_type),
        Commands::Switch { name } => cli::switch::run(name, vcs_type),
        Commands::Return {
            merge,
            delete,
            no_ff,
        } => cli::return_cmd::run(merge, delete, no_ff, cli.no_hooks, vcs_type),
        Commands::Info { name } => cli::info::run(name, vcs_type),
        Commands::Each {
            command,
            parallel,
            stop_on_error,
            filter,
            docker_running,
        } => cli::each::run(command, parallel, stop_on_error, filter, docker_running),
        Commands::Integrate {
            source,
            into,
            no_ff,
            squash,
            strategy,
        } => cli::integrate::run(source, into, no_ff, squash, strategy, vcs_type),
        Commands::Sync {
            source_branch,
            strategy,
            autostash,
            no_commit,
        } => cli::sync::run(source_branch, strategy, autostash, no_commit, vcs_type),
        Commands::InitShell => cli::init_shell::run(),
        Commands::Prune => cli::prune::run(),
        Commands::State { command } => match command {
            StateCommands::List => cli::state::list(),
            StateCommands::Clean => cli::state::clean(),
            StateCommands::Size { name } => cli::state::size(name),
        },
        Commands::Config { command } => match command {
            ConfigCommands::Init => cli::config_cmd::init(),
            ConfigCommands::Validate => cli::config_cmd::validate(),
            ConfigCommands::Show => cli::config_cmd::show(),
            ConfigCommands::Edit => cli::config_cmd::edit(),
        },
        Commands::Ports { command } => match command {
            PortsCommands::List => cli::ports::list(),
            PortsCommands::Show { name } => cli::ports::show(name),
            PortsCommands::Release { name } => cli::ports::release(name),
        },
        Commands::Docker { command } => match command {
            DockerCommands::Ps => cli::docker::ps(),
            DockerCommands::Start { name } => cli::docker::start(name),
            DockerCommands::Stop { name } => cli::docker::stop(name),
            DockerCommands::Restart { name } => cli::docker::restart(name),
            DockerCommands::Logs { name, service } => cli::docker::logs(name, service),
            DockerCommands::Exec { name, service, command } => cli::docker::exec(name, service, command),
            DockerCommands::Prune => cli::docker::prune(),
        },
    };

    // Handle errors with suggestions
    if let Err(error) = result {
        suggestions::display_error_with_suggestions(&error);
        std::process::exit(1);
    }
}
