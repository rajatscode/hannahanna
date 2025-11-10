use crate::errors::Result;
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

    // Remove the worktree
    git.remove_worktree(&name, force)?;

    // Print success message
    println!("Removed worktree '{}'", name);

    Ok(())
}
