pub mod backend_init;
pub mod git;
pub mod jujutsu;
pub mod mercurial;
pub mod traits;

// Re-export for convenience
pub use backend_init::{init_backend_from_current_dir, init_backend_with_detection};
pub use traits::VcsType;

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

/// VCS-agnostic workspace/worktree status
/// Represents the state of working directory changes across all VCS types
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceStatus {
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub untracked: usize,
}

impl WorkspaceStatus {
    /// Returns true if the workspace has no uncommitted changes
    pub fn is_clean(&self) -> bool {
        self.modified == 0 && self.added == 0 && self.deleted == 0 && self.untracked == 0
    }
}

/// Safely shorten a commit hash to 7 characters
/// Returns the shortened hash, or the full hash if it's shorter than 7 chars
pub fn short_commit(hash: &str) -> String {
    hash.chars().take(7).collect()
}
