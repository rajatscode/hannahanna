use crate::errors::Result;
use crate::fuzzy;
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

    // Remove the worktree
    git.remove_worktree(&matched_name, force)?;

    // Print success message
    println!("Removed worktree '{}'", matched_name);

    Ok(())
}
