use crate::env::validation;
use crate::errors::Result;
use crate::fuzzy;
use crate::vcs::{init_backend_from_current_dir, short_commit, VcsType};

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
pub fn run(name: String, vcs_type: Option<VcsType>) -> Result<()> {
    // Validate worktree name
    validation::validate_worktree_name(&name)?;

    // Initialize VCS backend
    let backend = if let Some(vcs) = vcs_type {
        crate::vcs::init_backend_with_detection(&std::env::current_dir()?, Some(vcs))?
    } else {
        init_backend_from_current_dir()?
    };

    // Get all worktrees for fuzzy matching
    let worktrees = backend.list_workspaces()?;
    let worktree_names: Vec<String> = worktrees.iter().map(|wt| wt.name.clone()).collect();

    // Find the best match using fuzzy matching
    let matched_name = fuzzy::find_best_match(&name, &worktree_names)?;

    // Get the worktree by the matched name
    let worktree = backend.get_workspace_by_name(&matched_name)?;

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
    if matched_name != name {
        eprintln!("Matched '{}' to '{}'", name, matched_name);
    }
    eprintln!("Switching to worktree '{}'", matched_name);
    eprintln!("  Branch: {}", worktree.branch);
    eprintln!("  Commit: {}", short_commit(&worktree.commit));

    Ok(())
}
