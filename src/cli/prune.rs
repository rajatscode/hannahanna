use crate::config::Config;
use crate::errors::Result;
use crate::snapshot;
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

    // Clean up orphaned snapshot stashes
    println!("\nScanning for orphaned snapshot stashes...");
    let state_dir = repo_root.join(".hn-state");
    let mut total_orphaned_stashes = 0;

    for worktree in &worktrees {
        match snapshot::cleanup_orphaned_stashes(&state_dir, &worktree.path) {
            Ok(count) if count > 0 => {
                println!("  Cleaned {} orphaned stash{} from '{}'",
                    count,
                    if count == 1 { "" } else { "es" },
                    worktree.name
                );
                total_orphaned_stashes += count;
            }
            Ok(_) => {} // No orphaned stashes for this worktree
            Err(e) => {
                eprintln!("  ⚠ Warning: Failed to clean stashes for '{}': {}", worktree.name, e);
            }
        }
    }

    if total_orphaned_stashes > 0 {
        println!("\nTotal: Cleaned {} orphaned snapshot stash{}.",
            total_orphaned_stashes,
            if total_orphaned_stashes == 1 { "" } else { "es" }
        );
    } else {
        println!("No orphaned snapshot stashes found.");
    }

    Ok(())
}
