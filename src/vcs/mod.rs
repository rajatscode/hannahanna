pub mod git;
pub mod jujutsu;
pub mod mercurial;
pub mod traits;

// Re-export for convenience
pub use traits::{create_backend, detect_vcs_type, VcsBackend, VcsType, WorkspaceStatus};

use std::path::PathBuf;

/// Represents a VCS worktree/workspace
#[derive(Debug, Clone, PartialEq)]
pub struct Worktree {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub commit: String,
    pub parent: Option<String>,
}

/// Safely shorten a commit hash to 7 characters
/// Returns the shortened hash, or the full hash if it's shorter than 7 chars
pub fn short_commit(hash: &str) -> String {
    hash.chars().take(7).collect()
}
