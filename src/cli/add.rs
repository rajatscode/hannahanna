use crate::config::Config;
use crate::env::symlinks::{SymlinkAction, SymlinkManager};
use crate::errors::Result;
use crate::hooks::{HookExecutor, HookType};
use crate::state::StateManager;
use crate::vcs::git::GitBackend;

pub fn run(name: String, branch: Option<String>, from: Option<String>, no_branch: bool) -> Result<()> {
    // Validate name (basic validation for now)
    if name.is_empty() {
        return Err(crate::errors::HnError::InvalidWorktreeName(
            "Worktree name cannot be empty".to_string(),
        ));
    }

    // Open git repository
    let git = GitBackend::open_from_current_dir()?;

    // Find repository root
    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;

    // Load configuration
    let config = Config::load(&repo_root)?;

    // Create the worktree
    println!("Creating worktree '{}'...", name);
    let worktree = git.create_worktree(&name, branch.as_deref(), from.as_deref(), no_branch)?;
    println!("✓ Git worktree created at {}", worktree.path.display());

    // Create state directory
    let state_manager = StateManager::new(&repo_root)?;
    let state_dir = state_manager.create_state_dir(&name)?;

    // Setup symlinks for shared resources
    if !config.shared_resources.is_empty() {
        let actions = SymlinkManager::setup(&config.shared_resources, &repo_root, &worktree.path)?;

        for action in actions {
            match action {
                SymlinkAction::Created { source, target: _ } => {
                    println!(
                        "✓ Shared {} (symlinked)",
                        source.file_name().unwrap().to_string_lossy()
                    );
                }
                SymlinkAction::Skipped { resource, reason } => {
                    println!("⚠ Skipped {} ({})", resource, reason);
                }
            }
        }
    }

    // Run post_create hook if configured
    if config.hooks.post_create.is_some() {
        println!("Running post_create hook...");
        let hook_executor = HookExecutor::new(config.hooks);
        hook_executor.run_hook(HookType::PostCreate, &worktree, &state_dir)?;
        println!("✓ Hook completed successfully");
    }

    println!("\nDone! Switch to the worktree with:");
    println!("  hn switch {}", name);

    Ok(())
}
