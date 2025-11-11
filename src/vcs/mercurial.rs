/// Mercurial (hg) backend implementation
/// Uses `hg share` for workspace creation
use crate::errors::{HnError, Result};
use crate::vcs::traits::{VcsBackend, VcsType};
use crate::vcs::{Worktree, git::WorktreeStatus};
use std::path::{Path, PathBuf};

pub struct MercurialBackend {
    repo_path: PathBuf,
}

impl MercurialBackend {
    /// Open a Mercurial repository from the current directory
    pub fn open_from_current_dir() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        Self::discover_repo(&current_dir)
    }

    /// Discover a Mercurial repository starting from the given path
    fn discover_repo(path: &Path) -> Result<Self> {
        let mut current = path.to_path_buf();

        loop {
            if current.join(".hg").exists() {
                return Ok(Self {
                    repo_path: current,
                });
            }

            if !current.pop() {
                return Err(HnError::NotInRepository);
            }
        }
    }
}

impl VcsBackend for MercurialBackend {
    fn vcs_type(&self) -> VcsType {
        VcsType::Mercurial
    }

    fn repo_root(&self) -> Result<PathBuf> {
        Ok(self.repo_path.clone())
    }

    fn create_workspace(
        &self,
        _name: &str,
        _branch: Option<&str>,
        _from: Option<&str>,
        _no_branch: bool,
    ) -> Result<Worktree> {
        // TODO: Implement using `hg share`
        // hg share [source] [dest]
        Err(HnError::ConfigError(
            "Mercurial backend not fully implemented yet. Coming in v0.3!".to_string(),
        ))
    }

    fn list_workspaces(&self) -> Result<Vec<Worktree>> {
        // TODO: Implement using registry file (.hg/wt-registry.json)
        // Mercurial doesn't have native workspace listing
        Err(HnError::ConfigError(
            "Mercurial backend not fully implemented yet. Coming in v0.3!".to_string(),
        ))
    }

    fn remove_workspace(&self, _name: &str, _force: bool) -> Result<()> {
        // TODO: Implement workspace removal and registry update
        Err(HnError::ConfigError(
            "Mercurial backend not fully implemented yet. Coming in v0.3!".to_string(),
        ))
    }

    fn get_workspace_by_name(&self, _name: &str) -> Result<Worktree> {
        // TODO: Look up in registry
        Err(HnError::ConfigError(
            "Mercurial backend not fully implemented yet. Coming in v0.3!".to_string(),
        ))
    }

    fn get_current_workspace(&self) -> Result<Worktree> {
        // TODO: Determine current workspace from path and registry
        Err(HnError::ConfigError(
            "Mercurial backend not fully implemented yet. Coming in v0.3!".to_string(),
        ))
    }

    fn get_workspace_status(&self, _worktree_path: &Path) -> Result<WorktreeStatus> {
        // TODO: Implement using `hg status`
        Err(HnError::ConfigError(
            "Mercurial backend not fully implemented yet. Coming in v0.3!".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mercurial_backend_returns_clear_error() {
        // Test that the backend gives a clear message about not being fully implemented
        let temp = tempfile::TempDir::new().unwrap();
        let repo_path = temp.path().join("hg-repo");
        std::fs::create_dir(&repo_path).unwrap();
        std::fs::create_dir(repo_path.join(".hg")).unwrap();

        let backend = MercurialBackend::discover_repo(&repo_path);
        assert!(backend.is_ok(), "Should detect Mercurial repo");

        let backend = backend.unwrap();
        assert_eq!(backend.vcs_type(), VcsType::Mercurial);

        // Operations should return clear error messages
        let result = backend.create_workspace("test", None, None, false);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("not fully implemented"));
    }
}
