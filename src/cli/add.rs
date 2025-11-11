use crate::config::Config;
use crate::docker::compose::ComposeGenerator;
use crate::docker::container::ContainerManager;
use crate::docker::ports::PortAllocator;
use crate::env::copy::{CopyAction, CopyManager};
use crate::env::symlinks::{SymlinkAction, SymlinkManager};
use crate::env::validation;
use crate::errors::Result;
use crate::hooks::{HookExecutor, HookType};
use crate::state::StateManager;
use crate::vcs::{init_backend_from_current_dir, VcsType};

pub fn run(
    name: String,
    branch: Option<String>,
    from: Option<String>,
    no_branch: bool,
    no_hooks: bool,
    vcs_type: Option<VcsType>,
) -> Result<()> {
    // Validate worktree name
    validation::validate_worktree_name(&name)?;

    // Initialize VCS backend (auto-detect or use explicit type)
    let backend = if let Some(vcs) = vcs_type {
        crate::vcs::init_backend_with_detection(&std::env::current_dir()?, Some(vcs))?
    } else {
        init_backend_from_current_dir()?
    };

    // Find repository root
    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;

    // Load configuration
    let config = Config::load(&repo_root)?;

    // Create the worktree
    eprintln!("Creating worktree '{}'...", name);
    let worktree =
        backend.create_workspace(&name, branch.as_deref(), from.as_deref(), no_branch)?;
    eprintln!("✓ Worktree created at {}", worktree.path.display());

    // Create state directory
    let state_manager = StateManager::new(&repo_root)?;
    let state_dir = state_manager.create_state_dir(&name)?;

    // Setup symlinks for shared resources
    if !config.shared_resources.is_empty() {
        let actions = SymlinkManager::setup(&config.shared_resources, &repo_root, &worktree.path)?;

        for action in actions {
            match action {
                SymlinkAction::Created { source, target: _ } => {
                    eprintln!(
                        "✓ Shared {} (symlinked)",
                        source.file_name().unwrap().to_string_lossy()
                    );
                }
                SymlinkAction::Skipped { resource, reason } => {
                    eprintln!("⚠ Skipped {} ({})", resource, reason);
                }
            }
        }
    }

    // Setup file copies from shared.copy configuration
    if let Some(ref shared) = config.shared {
        if !shared.copy.is_empty() {
            let actions = CopyManager::setup(&shared.copy, &repo_root, &worktree.path)?;

            for action in actions {
                match action {
                    CopyAction::Copied { source, target: _ } => {
                        eprintln!(
                            "✓ Copied {} to worktree",
                            source.file_name().unwrap().to_string_lossy()
                        );
                    }
                    CopyAction::Skipped { resource, reason } => {
                        eprintln!("⚠ Skipped copying {} ({})", resource, reason);
                    }
                }
            }
        }
    }

    // Run post_create hook if configured
    if config.hooks.post_create.is_some() && !no_hooks {
        eprintln!("Running post_create hook...");
        let hook_executor = HookExecutor::new(config.hooks.clone(), no_hooks);
        hook_executor.run_hook(HookType::PostCreate, &worktree, &state_dir)?;
        eprintln!("✓ Hook completed successfully");
    } else if config.hooks.post_create.is_some() && no_hooks {
        eprintln!("⚠ Skipping post_create hook (--no-hooks)");
    }

    // Docker integration
    if config.docker.enabled {
        eprintln!("\nSetting up Docker...");

        // Allocate ports
        let state_dir_path = repo_root.join(".wt-state");
        let mut port_allocator = PortAllocator::new(&state_dir_path)?;

        // Get services from config or use defaults
        let services: Vec<&str> = config
            .docker
            .ports
            .base
            .keys()
            .map(|s| s.as_str())
            .collect();

        let ports = port_allocator.allocate(&name, &services)?;

        // Display allocated ports
        for (service, port) in &ports {
            eprintln!("  {} port: {}", service, port);
        }

        // Generate docker-compose.override.yml
        let compose_gen = ComposeGenerator::new(&config.docker, &state_dir_path);
        compose_gen.save(&name, &worktree.path, &ports)?;
        eprintln!("✓ Generated docker-compose.override.yml");

        // Auto-start containers if configured
        if config.docker.auto_start {
            eprintln!("Starting Docker containers...");
            let container_mgr = ContainerManager::new(&config.docker, &state_dir_path)?;

            match container_mgr.start(&name, &worktree.path) {
                Ok(_) => eprintln!("✓ Containers started"),
                Err(e) => eprintln!("⚠ Failed to start containers: {}", e),
            }
        }
    }

    eprintln!("\nDone! Switch to the worktree with:");
    eprintln!("  hn switch {}", name);

    Ok(())
}
