pub mod git;

use std::path::PathBuf;

/// Represents a git worktree
#[derive(Debug, Clone)]
pub struct Worktree {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub commit: String,
}
