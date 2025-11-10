use crate::errors::Result;
use crate::vcs::git::GitBackend;

/// Switch to a worktree by name
///
/// This command is designed to work with a shell wrapper function.
/// The path is output to stdout so the shell wrapper can cd to it.
/// Info messages are printed to stderr to avoid interfering with the path output.
///
/// Example shell wrapper (for bash/zsh):
/// ```bash
/// hn() {
///     if [ "$1" = "switch" ]; then
///         local path=$(command hn switch "$2" 2>/dev/null)
///         if [ $? -eq 0 ]; then
///             cd "$path"
///             command hn switch "$2" >/dev/null  # Print info
///         else
///             command hn switch "$2"  # Print error
///         fi
///     else
///         command hn "$@"
///     fi
/// }
/// ```
pub fn run(name: String) -> Result<()> {
    // Validate name (basic validation for now)
    if name.is_empty() {
        return Err(crate::errors::HnError::InvalidWorktreeName(
            "Worktree name cannot be empty".to_string(),
        ));
    }

    // Open git repository
    let git = GitBackend::open_from_current_dir()?;

    // Find the worktree by name (exact match for MVP)
    let worktree = git.get_worktree_by_name(&name)?;

    // Verify the worktree path exists
    if !worktree.path.exists() {
        return Err(crate::errors::HnError::WorktreeNotFound(format!(
            "Worktree '{}' path does not exist: {}",
            name,
            worktree.path.display()
        )));
    }

    // Output the path to stdout (for shell wrapper to use)
    println!("{}", worktree.path.display());

    // Print helpful info to stderr (won't interfere with path capture)
    eprintln!("Switching to worktree '{}'", name);
    eprintln!("  Branch: {}", worktree.branch);
    eprintln!("  Commit: {}", &worktree.commit[..7.min(worktree.commit.len())]);

    Ok(())
}
