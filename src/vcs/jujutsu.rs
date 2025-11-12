/// Jujutsu (jj) backend implementation
/// Uses native `jj workspace` commands
use crate::errors::{HnError, Result};
use crate::vcs::traits::{VcsBackend, VcsType};
use crate::vcs::{WorkspaceStatus, Worktree};
use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(dead_code)] // Will be used when --vcs flag is implemented
pub struct JujutsuBackend {
    repo_path: PathBuf,
}

#[allow(dead_code)] // Will be used when --vcs flag is implemented
impl JujutsuBackend {
    /// Open a Jujutsu repository from the current directory
    pub fn open_from_current_dir() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        Self::discover_repo(&current_dir)
    }

    /// Open a Jujutsu repository from a specific path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::discover_repo(path.as_ref())
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

    /// Parse `jj workspace list` output
    /// Format: <workspace-name>: <path>
    fn parse_workspace_list(&self, output: &str) -> Vec<(String, PathBuf)> {
        output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let name = parts[0].trim().to_string();
                    let path = PathBuf::from(parts[1].trim());
                    Some((name, path))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get current branch/change using `jj log -r @`
    fn get_current_change(&self, workspace_path: &Path) -> Result<String> {
        let output = Command::new("jj")
            .args(["log", "-r", "@", "--no-graph", "-T", "change_id"])
            .current_dir(workspace_path)
            .output()?;

        if !output.status.success() {
            return Ok("unknown".to_string());
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get current branch name
    fn get_current_branch(&self, workspace_path: &Path) -> Result<String> {
        let output = Command::new("jj")
            .args(["log", "-r", "@", "--no-graph", "-T", "branches"])
            .current_dir(workspace_path)
            .output()?;

        if !output.status.success() {
            return Ok("(no branch)".to_string());
        }

        let branches = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branches.is_empty() {
            Ok("(no branch)".to_string())
        } else {
            Ok(branches)
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
        name: &str,
        _branch: Option<&str>,
        _from: Option<&str>,
        _no_branch: bool,
    ) -> Result<Worktree> {
        // Determine workspace path (sibling directory)
        let workspace_path = self
            .repo_path
            .parent()
            .ok_or_else(|| {
                HnError::ConfigError("Could not determine parent directory".to_string())
            })?
            .join(name);

        // Check if workspace already exists
        if workspace_path.exists() {
            return Err(HnError::WorktreeAlreadyExists(name.to_string()));
        }

        // Create workspace using `jj workspace add`
        let output = Command::new("jj")
            .args(["workspace", "add", "--name", name])
            .arg(&workspace_path)
            .current_dir(&self.repo_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::ConfigError(format!(
                "Failed to create Jujutsu workspace: {}",
                stderr
            )));
        }

        // Get branch and commit
        let branch = self.get_current_branch(&workspace_path)?;
        let commit = self.get_current_change(&workspace_path)?;

        // Detect parent
        let parent = self.get_current_workspace().ok().map(|wt| wt.name);

        Ok(Worktree {
            name: name.to_string(),
            path: workspace_path,
            branch,
            commit,
            parent,
        })
    }

    fn list_workspaces(&self) -> Result<Vec<Worktree>> {
        let output = Command::new("jj")
            .args(["workspace", "list"])
            .current_dir(&self.repo_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::ConfigError(format!(
                "Failed to list Jujutsu workspaces: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let workspaces_data = self.parse_workspace_list(&stdout);

        let mut worktrees = Vec::new();
        for (name, path) in workspaces_data {
            let branch = self
                .get_current_branch(&path)
                .unwrap_or_else(|_| "(no branch)".to_string());
            let commit = self
                .get_current_change(&path)
                .unwrap_or_else(|_| "unknown".to_string());

            worktrees.push(Worktree {
                name,
                path,
                branch,
                commit,
                parent: None, // Jujutsu doesn't track parent relationships natively
            });
        }

        Ok(worktrees)
    }

    fn remove_workspace(&self, name: &str, force: bool) -> Result<()> {
        // Get workspace info first
        let workspace = self.get_workspace_by_name(name)?;

        // Check for uncommitted changes if not force
        if !force {
            let status = self.get_workspace_status(&workspace.path)?;
            if !status.is_clean() {
                return Err(HnError::ConfigError(format!(
                    "Workspace '{}' has uncommitted changes. Use --force to remove anyway.",
                    name
                )));
            }
        }

        // Remove workspace using `jj workspace forget`
        let output = Command::new("jj")
            .args(["workspace", "forget", name])
            .current_dir(&self.repo_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::ConfigError(format!(
                "Failed to remove Jujutsu workspace: {}",
                stderr
            )));
        }

        // Remove the directory
        if workspace.path.exists() {
            std::fs::remove_dir_all(&workspace.path)?;
        }

        Ok(())
    }

    fn get_workspace_by_name(&self, name: &str) -> Result<Worktree> {
        let worktrees = self.list_workspaces()?;
        worktrees
            .into_iter()
            .find(|wt| wt.name == name)
            .ok_or_else(|| HnError::WorktreeNotFound(name.to_string()))
    }

    fn get_current_workspace(&self) -> Result<Worktree> {
        let current_dir = std::env::current_dir()?;
        let worktrees = self.list_workspaces()?;

        // Find the workspace containing the current directory
        for wt in worktrees {
            if current_dir.starts_with(&wt.path) {
                return Ok(wt);
            }
        }

        Err(HnError::NotInRepository)
    }

    fn get_workspace_status(&self, worktree_path: &Path) -> Result<WorkspaceStatus> {
        let output = Command::new("jj")
            .args(["status"])
            .current_dir(worktree_path)
            .output()?;

        if !output.status.success() {
            return Err(HnError::ConfigError(
                "Failed to get Jujutsu status".to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut status = WorkspaceStatus {
            modified: 0,
            added: 0,
            deleted: 0,
            untracked: 0,
        };

        // Parse jj status output
        // Format varies but typically:
        // Working copy changes:
        // M file.txt
        // A new-file.txt
        // D deleted.txt
        for line in stdout.lines() {
            if line.len() < 2 {
                continue;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let status_char = trimmed.chars().next().unwrap();
            match status_char {
                'M' => status.modified += 1,
                'A' => status.added += 1,
                'D' => status.deleted += 1,
                '?' => status.untracked += 1,
                _ => {}
            }
        }

        Ok(status)
    }

    fn setup_sparse_checkout(&self, worktree_path: &Path, paths: &[String]) -> Result<()> {
        if paths.is_empty() {
            return Ok(());
        }

        // Jujutsu uses `jj sparse set` to configure sparse checkout
        let mut cmd = Command::new("jj");
        cmd.arg("sparse")
            .arg("set")
            .current_dir(worktree_path);

        // Add all paths
        for path in paths {
            cmd.arg(path);
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::ConfigError(format!(
                "Failed to set sparse checkout for Jujutsu: {}",
                stderr
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jujutsu_backend_discovery() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_path = temp.path().join("jj-repo");
        std::fs::create_dir(&repo_path).unwrap();
        std::fs::create_dir(repo_path.join(".jj")).unwrap();

        let backend = JujutsuBackend::discover_repo(&repo_path);
        assert!(backend.is_ok(), "Should discover Jujutsu repo");

        let backend = backend.unwrap();
        assert_eq!(backend.vcs_type(), VcsType::Jujutsu);
        assert_eq!(backend.repo_root().unwrap(), repo_path);
    }

    #[test]
    fn test_parse_workspace_list() {
        let backend = JujutsuBackend {
            repo_path: PathBuf::from("/test"),
        };

        let output = "default: /home/user/repo\nfeature-x: /home/user/feature-x\n";
        let workspaces = backend.parse_workspace_list(output);

        assert_eq!(workspaces.len(), 2);
        assert_eq!(workspaces[0].0, "default");
        assert_eq!(workspaces[0].1, PathBuf::from("/home/user/repo"));
        assert_eq!(workspaces[1].0, "feature-x");
        assert_eq!(workspaces[1].1, PathBuf::from("/home/user/feature-x"));
    }
}
