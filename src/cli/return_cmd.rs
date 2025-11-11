// Return command: Switch back to parent worktree with optional merge
use crate::errors::{HnError, Result};
use crate::vcs::git::GitBackend;
use std::env;
use std::process::Command;

pub fn run(merge: bool, delete: bool, no_ff: bool) -> Result<()> {
    // Validate flag combinations
    if delete && !merge {
        return Err(HnError::ConfigError(
            "--delete requires --merge. Use 'hn return --merge --delete'".to_string()
        ));
    }

    if no_ff && !merge {
        return Err(HnError::ConfigError(
            "--no-ff requires --merge. Use 'hn return --merge --no-ff'".to_string()
        ));
    }

    let git = GitBackend::open_from_current_dir()?;

    // Get current worktree
    let current_worktree = git.get_current_worktree()?;

    // Check if current worktree has a parent
    let parent_name = current_worktree
        .parent
        .ok_or_else(|| HnError::NoParent(current_worktree.name.clone()))?;

    // Get parent worktree info
    let worktrees = git.list_worktrees()?;
    let parent = worktrees
        .iter()
        .find(|wt| wt.name == parent_name)
        .ok_or_else(|| {
            HnError::WorktreeNotFound(format!(
                "Parent worktree '{}' not found. It may have been deleted.",
                parent_name
            ))
        })?;

    eprintln!("Current worktree: {}", current_worktree.name);
    eprintln!("Parent worktree: {}", parent_name);

    // If merge requested, merge current branch into parent
    if merge {
        eprintln!("\n→ Merging '{}' into '{}'...", current_worktree.branch, parent_name);

        // Change to parent worktree directory
        env::set_current_dir(&parent.path)?;

        // Perform the merge
        let mut cmd = Command::new("git");
        cmd.arg("merge");

        if no_ff {
            cmd.arg("--no-ff");
        }

        cmd.arg(&current_worktree.branch);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::Git(git2::Error::from_str(&format!(
                "Failed to merge '{}' into '{}': {}",
                current_worktree.branch, parent_name, stderr
            ))));
        }

        eprintln!("✓ Merge successful");
    }

    // If delete requested, remove the current worktree
    if delete {
        eprintln!("\n→ Deleting worktree '{}'...", current_worktree.name);

        // Need to be outside the worktree to delete it
        env::set_current_dir(&parent.path)?;

        // Remove the worktree
        crate::cli::remove::run(current_worktree.name.clone(), false)?;

        eprintln!("✓ Worktree deleted");
    }

    // Output parent path for shell wrapper
    println!("{}", parent.path.display());

    // Print info to stderr
    eprintln!("\n→ Switched to worktree '{}'", parent_name);
    eprintln!("  Path: {}", parent.path.display());
    eprintln!("  Branch: {}", parent.branch);

    Ok(())
}
