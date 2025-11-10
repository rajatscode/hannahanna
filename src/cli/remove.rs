use crate::config::Config;
use crate::errors::Result;
use crate::fuzzy;
use crate::hooks::{HookExecutor, HookType};
use crate::state::StateManager;
use crate::vcs::git::GitBackend;

pub fn run(name: String, force: bool) -> Result<()> {
    // Validate name (basic validation for now)
    if name.is_empty() {
        return Err(crate::errors::HnError::InvalidWorktreeName(
            "Worktree name cannot be empty".to_string(),
        ));
    }

    // Open git repository
    let git = GitBackend::open_from_current_dir()?;

    // Get all worktrees for fuzzy matching
    let worktrees = git.list_worktrees()?;
    let worktree_names: Vec<String> = worktrees.iter().map(|wt| wt.name.clone()).collect();

    // Find the best match using fuzzy matching
    let matched_name = fuzzy::find_best_match(&name, &worktree_names)?;

    if matched_name != name {
        eprintln!("Matched '{}' to '{}'", name, matched_name);
    }

    // Get worktree info for hooks
    let worktree = git.get_worktree_by_name(&matched_name)?;

    // Find repository root
    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;

    // Load configuration
    let config = Config::load(&repo_root)?;

    // Run pre_remove hook if configured (before confirming removal)
    let state_manager = StateManager::new(&repo_root)?;
    let state_dir = state_manager.get_state_dir(&matched_name);

    if config.hooks.pre_remove.is_some() {
        println!("Running pre_remove hook...");
        let hook_executor = HookExecutor::new(config.hooks);
        hook_executor.run_hook(HookType::PreRemove, &worktree, &state_dir)?;
        println!("✓ Hook completed successfully");
    }

    // Remove the worktree
    git.remove_worktree(&matched_name, force)?;

    // Clean up state directory
    state_manager.remove_state_dir(&matched_name)?;

    // Print success message
    println!("Removed worktree '{}'", matched_name);

    Ok(())
}
