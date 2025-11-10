use crate::errors::Result;
use crate::fuzzy;
use crate::vcs::git::GitBackend;
use std::env;

/// Show detailed information about a worktree
///
/// If no name is provided, shows info for the current worktree
pub fn run(name: Option<String>) -> Result<()> {
    let git = GitBackend::open_from_current_dir()?;

    // Determine which worktree to show info for
    let worktree = if let Some(name) = name {
        // Get all worktrees for fuzzy matching
        let worktrees = git.list_worktrees()?;
        let worktree_names: Vec<String> = worktrees.iter().map(|wt| wt.name.clone()).collect();

        // Find the best match using fuzzy matching
        let matched_name = fuzzy::find_best_match(&name, &worktree_names)?;

        // Show info for named worktree
        git.get_worktree_by_name(&matched_name)?
    } else {
        // Show info for current worktree
        let current_dir = env::current_dir()?;
        git.get_current_worktree(&current_dir)?
    };

    // Get git status
    let status = git.get_worktree_status(&worktree.path)?;

    // Print worktree information
    println!("Worktree: {}", worktree.name);
    println!("Path: {}", worktree.path.display());
    println!("Branch: {}", worktree.branch);
    println!(
        "Commit: {} {}",
        &worktree.commit[..7.min(worktree.commit.len())],
        git.get_commit_message(&worktree.path)?
            .lines()
            .next()
            .unwrap_or("")
    );
    println!();

    // Print git status
    println!("Git Status:");
    if status.is_clean() {
        println!("  Clean (no changes)");
    } else {
        if status.modified > 0 {
            println!(
                "  Modified: {} file{}",
                status.modified,
                if status.modified == 1 { "" } else { "s" }
            );
        }
        if status.added > 0 {
            println!(
                "  Added: {} file{}",
                status.added,
                if status.added == 1 { "" } else { "s" }
            );
        }
        if status.deleted > 0 {
            println!(
                "  Deleted: {} file{}",
                status.deleted,
                if status.deleted == 1 { "" } else { "s" }
            );
        }
        if status.untracked > 0 {
            println!(
                "  Untracked: {} file{}",
                status.untracked,
                if status.untracked == 1 { "" } else { "s" }
            );
        }
    }

    Ok(())
}
