use crate::config::Config;
use crate::errors::Result;
use crate::state::StateManager;
use crate::vcs::git::GitBackend;

pub fn run() -> Result<()> {
    // Open git repository
    let git = GitBackend::open_from_current_dir()?;

    // Find repository root
    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;

    // Get list of active worktrees
    let worktrees = git.list_worktrees()?;
    let active_names: Vec<String> = worktrees.iter().map(|wt| wt.name.clone()).collect();

    // Initialize state manager
    let state_manager = StateManager::new(&repo_root)?;

    // Find orphaned state directories
    let orphaned = state_manager.list_orphaned(&active_names)?;

    if orphaned.is_empty() {
        println!("No orphaned state directories found.");
        return Ok(());
    }

    println!(
        "Found {} orphaned state director{}:",
        orphaned.len(),
        if orphaned.len() == 1 { "y" } else { "ies" }
    );
    for name in &orphaned {
        println!("  - {}", name);
    }

    // Clean orphaned directories
    let cleaned = state_manager.clean_orphaned(&active_names)?;

    println!(
        "\nCleaned {} orphaned state director{}.",
        cleaned.len(),
        if cleaned.len() == 1 { "y" } else { "ies" }
    );

    Ok(())
}
