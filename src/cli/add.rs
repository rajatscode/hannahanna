use crate::errors::Result;
use crate::vcs::git::GitBackend;

pub fn run(name: String, branch: Option<String>) -> Result<()> {
    // Validate name (basic validation for now)
    if name.is_empty() {
        return Err(crate::errors::HnError::InvalidWorktreeName(
            "Worktree name cannot be empty".to_string(),
        ));
    }

    // Open git repository
    let git = GitBackend::open_from_current_dir()?;

    // Create the worktree
    let worktree = git.create_worktree(&name, branch.as_deref())?;

    // Print success message
    println!("Created worktree '{}'", name);
    println!("  Path: {}", worktree.path.display());
    println!("  Branch: {}", worktree.branch);

    Ok(())
}
