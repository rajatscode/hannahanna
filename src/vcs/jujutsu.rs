/// Jujutsu (jj) backend implementation
/// Uses native `jj workspace` commands
use crate::errors::{HnError, Result};
use crate::vcs::traits::{VcsBackend, VcsType};
use crate::vcs::{git::WorktreeStatus, Worktree};
use std::path::{Path, PathBuf};

#[allow(dead_code)] // Skeleton implementation - will be used in v0.3
pub struct JujutsuBackend {
    repo_path: PathBuf,
}

impl JujutsuBackend {
    /// Open a Jujutsu repository from the current directory
    #[allow(dead_code)] // Skeleton implementation - will be used in v0.3
    pub fn open_from_current_dir() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        Self::discover_repo(&current_dir)
    }

    /// Discover a Jujutsu repository starting from the given path
    fn discover_repo(path: &Path) -> Result<Self> {
        let mut current = path.to_path_buf();

        loop {
            if current.join(".jj").exists() {
                return Ok(Self { repo_path: current });
            }

            if !current.pop() {
                return Err(HnError::NotInRepository);
            }
        }
    }
}

impl VcsBackend for JujutsuBackend {
    fn vcs_type(&self) -> VcsType {
        VcsType::Jujutsu
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
        // TODO: Implement using `jj workspace add`
        // jj workspace add [path]
        Err(HnError::ConfigError(
            "Jujutsu backend not fully implemented yet. Coming in v0.3!".to_string(),
        ))
    }

    fn list_workspaces(&self) -> Result<Vec<Worktree>> {
        // TODO: Implement using `jj workspace list`
        // Jujutsu has native workspace support
        Err(HnError::ConfigError(
            "Jujutsu backend not fully implemented yet. Coming in v0.3!".to_string(),
        ))
    }

    fn remove_workspace(&self, _name: &str, _force: bool) -> Result<()> {
        // TODO: Implement using `jj workspace forget`
        Err(HnError::ConfigError(
            "Jujutsu backend not fully implemented yet. Coming in v0.3!".to_string(),
        ))
    }

    fn get_workspace_by_name(&self, _name: &str) -> Result<Worktree> {
        // TODO: Query from `jj workspace list`
        Err(HnError::ConfigError(
            "Jujutsu backend not fully implemented yet. Coming in v0.3!".to_string(),
        ))
    }

    fn get_current_workspace(&self) -> Result<Worktree> {
        // TODO: Determine current workspace from `jj workspace list` or path
        Err(HnError::ConfigError(
            "Jujutsu backend not fully implemented yet. Coming in v0.3!".to_string(),
        ))
    }

    fn get_workspace_status(&self, _worktree_path: &Path) -> Result<WorktreeStatus> {
        // TODO: Implement using `jj status`
        Err(HnError::ConfigError(
            "Jujutsu backend not fully implemented yet. Coming in v0.3!".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jujutsu_backend_returns_clear_error() {
        // Test that the backend gives a clear message about not being fully implemented
        let temp = tempfile::TempDir::new().unwrap();
        let repo_path = temp.path().join("jj-repo");
        std::fs::create_dir(&repo_path).unwrap();
        std::fs::create_dir(repo_path.join(".jj")).unwrap();

        let backend = JujutsuBackend::discover_repo(&repo_path);
        assert!(backend.is_ok(), "Should detect Jujutsu repo");

        let backend = backend.unwrap();
        assert_eq!(backend.vcs_type(), VcsType::Jujutsu);

        // Operations should return clear error messages
        let result = backend.create_workspace("test", None, None, false);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("not fully implemented"));
    }
}
