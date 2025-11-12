use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

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
mod templates;
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
        /// Apply a template from .hn-templates/
        #[arg(long)]
        template: Option<String>,
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
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
    /// Automate hannahanna installation and shell integration
    Setup {
        /// Shell type (bash, zsh, or fish). Auto-detects if not specified
        #[arg(long)]
        shell: Option<String>,
    },
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
    /// Manage worktree templates
    Templates {
        #[command(subcommand)]
        command: TemplatesCommands,
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
    /// Manage worktree registry cache
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Show cache statistics
    Stats,
    /// Clear the cache
    Clear,
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
    /// Reassign ports to a worktree (releases old, allocates new)
    Reassign {
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

#[derive(Subcommand)]
enum TemplatesCommands {
    /// List all available templates
    List {
        /// Output format (json or table)
        #[arg(long)]
        json: bool,
    },
    /// Show details about a specific template
    Show {
        /// Name of the template
        name: String,
    },
    /// Create a new template
    Create {
        /// Name of the template
        name: String,
        /// Template description
        #[arg(long)]
        description: Option<String>,
        /// Enable Docker in template
        #[arg(long)]
        docker: bool,
        /// Create from current worktree config
        #[arg(long)]
        from_current: bool,
    },
}

/// Resolve command aliases before parsing
fn resolve_aliases() -> Vec<String> {
    let args: Vec<String> = std::env::args().collect();

    // Need at least program name + command
    if args.len() < 2 {
        return args;
    }

    // Try to load config (silently skip if not in a repo or no config)
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(_) => return args,
    };

    let root = match config::Config::find_repo_root(&cwd) {
        Ok(r) => r,
        Err(_) => return args,
    };

    let config = match config::Config::load(&root) {
        Ok(cfg) => cfg,
        Err(_) => return args,
    };

    // If no aliases configured, return original args
    if config.aliases.is_empty() {
        return args;
    }

    // Find the command argument (skip program name and any global flags)
    let mut command_idx = 1;
    while command_idx < args.len() {
        let arg = &args[command_idx];
        // Skip global flags
        if arg == "--no-hooks" || arg == "--vcs" {
            command_idx += 1;
            // Skip --vcs value
            if arg == "--vcs" && command_idx < args.len() {
                command_idx += 1;
            }
            continue;
        }
        // Found the command
        break;
    }

    // No command found
    if command_idx >= args.len() {
        return args;
    }

    let command = &args[command_idx];

    // Check if command is an alias
    if let Some(alias_expansion) = config.aliases.get(command) {
        // Expand the alias with cycle detection
        let mut expanded = expand_alias_with_cycle_detection(
            alias_expansion,
            &config.aliases,
            &mut std::collections::HashSet::new(),
        );

        // Build new args: program name + global flags + expanded alias + remaining args
        let mut new_args = args[..command_idx].to_vec();
        new_args.append(&mut expanded);
        new_args.extend_from_slice(&args[command_idx + 1..]);

        return new_args;
    }

    args
}

/// Expand an alias with cycle detection
fn expand_alias_with_cycle_detection(
    alias: &str,
    aliases: &std::collections::HashMap<String, String>,
    seen: &mut std::collections::HashSet<String>,
) -> Vec<String> {
    // Split alias into words
    let parts: Vec<String> = alias.split_whitespace().map(String::from).collect();

    if parts.is_empty() {
        return vec![];
    }

    let first_word = &parts[0];

    // Check for cycle
    if seen.contains(first_word) {
        eprintln!("Error: Alias cycle detected involving '{}'", first_word);
        eprintln!("Alias chain: {}", seen.iter().cloned().collect::<Vec<_>>().join(" -> "));
        std::process::exit(1);
    }

    // If first word is another alias, recursively expand
    if let Some(nested_alias) = aliases.get(first_word) {
        seen.insert(first_word.clone());
        let mut expanded = expand_alias_with_cycle_detection(nested_alias, aliases, seen);
        seen.remove(first_word);

        // Append remaining parts
        expanded.extend_from_slice(&parts[1..]);
        expanded
    } else {
        // Not an alias, return as is
        parts
    }
}

fn main() {
    // Resolve aliases before parsing
    let args = resolve_aliases();
    let cli = Cli::parse_from(args);

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
            template,
        } => cli::add::run(name, branch, from, no_branch, sparse, template, cli.no_hooks, vcs_type),
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
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "hn", &mut std::io::stdout());
            Ok(())
        }
        Commands::Setup { shell } => cli::setup::run(shell),
        Commands::State { command } => match command {
            StateCommands::List => cli::state::list(),
            StateCommands::Clean => cli::state::clean(),
            StateCommands::Size { name } => cli::state::size(name),
            StateCommands::Cache { command } => match command {
                CacheCommands::Stats => cli::state::cache_stats(),
                CacheCommands::Clear => cli::state::cache_clear(),
            },
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
            PortsCommands::Reassign { name } => cli::ports::reassign(name),
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
        Commands::Templates { command } => match command {
            TemplatesCommands::List { json } => cli::templates::list(json),
            TemplatesCommands::Show { name } => cli::templates::show(&name),
            TemplatesCommands::Create { name, description, docker, from_current } => {
                cli::templates::create(&name, description.as_deref(), docker, from_current)
            }
        },
    };

    // Handle errors with suggestions
    if let Err(error) = result {
        suggestions::display_error_with_suggestions(&error);
        std::process::exit(1);
    }
}
