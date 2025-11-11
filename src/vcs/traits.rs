/// VCS abstraction layer - trait that all VCS backends must implement
use crate::errors::Result;
use crate::vcs::Worktree;
use std::path::{Path, PathBuf};

/// Enum representing the supported VCS types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsType {
    Git,
    Mercurial,
    Jujutsu,
}

impl VcsType {
    /// Parse VCS type from string (for --vcs flag)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "git" => Some(VcsType::Git),
            "hg" | "mercurial" => Some(VcsType::Mercurial),
            "jj" | "jujutsu" => Some(VcsType::Jujutsu),
            _ => None,
        }
    }

    /// Convert to string for display
    pub fn as_str(&self) -> &'static str {
        match self {
            VcsType::Git => "git",
            VcsType::Mercurial => "mercurial",
            VcsType::Jujutsu => "jujutsu",
        }
    }
}

/// Trait that all VCS backends must implement
pub trait VcsBackend {
    /// Get the VCS type
    fn vcs_type(&self) -> VcsType;

    /// Get the repository root path
    fn repo_root(&self) -> Result<PathBuf>;

    /// Create a new workspace/worktree
    ///
    /// # Arguments
    /// * `name` - Name of the workspace
    /// * `branch` - Branch name (optional, defaults to name)
    /// * `from` - Base branch to create from (optional)
    /// * `no_branch` - Don't create new branch, checkout existing
    fn create_workspace(
        &self,
        name: &str,
        branch: Option<&str>,
        from: Option<&str>,
        no_branch: bool,
    ) -> Result<Worktree>;

    /// List all workspaces
    fn list_workspaces(&self) -> Result<Vec<Worktree>>;

    /// Remove a workspace
    fn remove_workspace(&self, name: &str, force: bool) -> Result<()>;

    /// Get a workspace by name
    fn get_workspace_by_name(&self, name: &str) -> Result<Worktree>;

    /// Get the current workspace based on current directory
    fn get_current_workspace(&self) -> Result<Worktree>;

    /// Get workspace status (modified, added, deleted, untracked files)
    fn get_workspace_status(&self, worktree_path: &Path) -> Result<WorkspaceStatus>;

    /// Check if a path has uncommitted changes
    fn has_uncommitted_changes(&self, worktree_path: &Path) -> Result<bool> {
        let status = self.get_workspace_status(worktree_path)?;
        Ok(!status.is_clean())
    }
}

// Re-export WorktreeStatus from git module (will be unified)
pub use crate::vcs::git::WorktreeStatus as WorkspaceStatus;

/// Auto-detect VCS type by checking for VCS directories
///
/// Detection order:
/// 1. .jj/ → Jujutsu
/// 2. .git/ → Git
/// 3. .hg/ → Mercurial
/// 4. None found → Error
pub fn detect_vcs_type(path: &Path) -> Option<VcsType> {
    // Check for Jujutsu first (newest, most specific)
    if path.join(".jj").exists() {
        return Some(VcsType::Jujutsu);
    }

    // Check for Git
    if path.join(".git").exists() {
        return Some(VcsType::Git);
    }

    // Check for Mercurial
    if path.join(".hg").exists() {
        return Some(VcsType::Mercurial);
    }

    None
}

/// Create a VCS backend instance for the given type
pub fn create_backend(vcs_type: VcsType) -> Result<Box<dyn VcsBackend>> {
    match vcs_type {
        VcsType::Git => {
            let git = crate::vcs::git::GitBackend::open_from_current_dir()?;
            Ok(Box::new(git))
        }
        VcsType::Mercurial => {
            let hg = crate::vcs::mercurial::MercurialBackend::open_from_current_dir()?;
            Ok(Box::new(hg))
        }
        VcsType::Jujutsu => {
            let jj = crate::vcs::jujutsu::JujutsuBackend::open_from_current_dir()?;
            Ok(Box::new(jj))
        }
    }
}
