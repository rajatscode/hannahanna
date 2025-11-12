/// Mercurial (hg) backend implementation
/// Uses `hg share` for workspace creation and registry for tracking
use crate::errors::{HnError, Result};
use crate::vcs::traits::{VcsBackend, VcsType};
use crate::vcs::{WorkspaceStatus, Worktree};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Registry entry for a Mercurial share
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // Will be used when --vcs flag is implemented
struct ShareEntry {
    name: String,
    path: PathBuf,
    branch: String,
    parent: Option<String>,
}

/// Registry of all Mercurial shares (workspaces)
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)] // Will be used when --vcs flag is implemented
struct ShareRegistry {
    shares: Vec<ShareEntry>,
}

#[allow(dead_code)] // Will be used when --vcs flag is implemented
impl ShareRegistry {
    fn new() -> Self {
        Self { shares: Vec::new() }
    }

    fn load(registry_path: &Path) -> Result<Self> {
        if !registry_path.exists() {
            return Ok(Self::new());
        }

        let contents = fs::read_to_string(registry_path)?;
        serde_json::from_str(&contents)
            .map_err(|e| HnError::ConfigError(format!("Failed to parse share registry: {}", e)))
    }

    fn save(&self, registry_path: &Path) -> Result<()> {
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(registry_path, contents)?;
        Ok(())
    }

    fn add(&mut self, entry: ShareEntry) {
        self.shares.push(entry);
    }

    fn remove(&mut self, name: &str) -> bool {
        let original_len = self.shares.len();
        self.shares.retain(|e| e.name != name);
        self.shares.len() < original_len
    }

    fn find(&self, name: &str) -> Option<&ShareEntry> {
        self.shares.iter().find(|e| e.name == name)
    }

    fn find_by_path(&self, path: &Path) -> Option<&ShareEntry> {
        self.shares.iter().find(|e| path.starts_with(&e.path))
    }
}

#[allow(dead_code)] // Will be used when --vcs flag is implemented
pub struct MercurialBackend {
    repo_path: PathBuf,
}

#[allow(dead_code)] // Will be used when --vcs flag is implemented
impl MercurialBackend {
    /// Open a Mercurial repository from the current directory
    pub fn open_from_current_dir() -> Result<Self> {
        let current_dir = std::env::current_dir()?;
        Self::discover_repo(&current_dir)
    }

    /// Open a Mercurial repository from a specific path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::discover_repo(path.as_ref())
    }

    /// Discover a Mercurial repository starting from the given path
    fn discover_repo(path: &Path) -> Result<Self> {
        let mut current = path.to_path_buf();

        loop {
            if current.join(".hg").exists() {
                return Ok(Self { repo_path: current });
            }

            if !current.pop() {
                return Err(HnError::NotInRepository);
            }
        }
    }

    /// Get the path to the registry file
    fn registry_path(&self) -> PathBuf {
        self.repo_path.join(".hg").join("wt-registry.json")
    }

    /// Load the share registry
    fn load_registry(&self) -> Result<ShareRegistry> {
        ShareRegistry::load(&self.registry_path())
    }

    /// Save the share registry
    fn save_registry(&self, registry: &ShareRegistry) -> Result<()> {
        registry.save(&self.registry_path())
    }

    /// Get the current branch using `hg branch`
    fn get_current_branch(&self, path: &Path) -> Result<String> {
        let output = Command::new("hg")
            .arg("branch")
            .current_dir(path)
            .output()?;

        if !output.status.success() {
            return Err(HnError::ConfigError(
                "Failed to get current Mercurial branch".to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get the current commit hash using `hg id`
    fn get_current_commit(&self, path: &Path) -> Result<String> {
        let output = Command::new("hg")
            .arg("id")
            .arg("-i")
            .current_dir(path)
            .output()?;

        if !output.status.success() {
            return Err(HnError::ConfigError(
                "Failed to get current Mercurial commit".to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
        name: &str,
        branch: Option<&str>,
        _from: Option<&str>,
        _no_branch: bool,
    ) -> Result<Worktree> {
        // Determine the share path (sibling directory)
        let share_path = self
            .repo_path
            .parent()
            .ok_or_else(|| {
                HnError::ConfigError("Could not determine parent directory".to_string())
            })?
            .join(name);

        // Check if share already exists
        if share_path.exists() {
            return Err(HnError::WorktreeAlreadyExists(name.to_string()));
        }

        // Create share using `hg share`
        let output = Command::new("hg")
            .arg("share")
            .arg(&self.repo_path)
            .arg(&share_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::ConfigError(format!(
                "Failed to create Mercurial share: {}",
                stderr
            )));
        }

        // Update to specified branch if provided
        let branch_name = if let Some(b) = branch {
            let output = Command::new("hg")
                .arg("update")
                .arg(b)
                .current_dir(&share_path)
                .output()?;

            if !output.status.success() {
                // Try to create new branch
                let output = Command::new("hg")
                    .arg("branch")
                    .arg(b)
                    .current_dir(&share_path)
                    .output()?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(HnError::ConfigError(format!(
                        "Failed to switch to branch: {}",
                        stderr
                    )));
                }
            }
            b.to_string()
        } else {
            self.get_current_branch(&share_path)?
        };

        // Get commit hash
        let commit = self.get_current_commit(&share_path)?;

        // Detect parent
        let parent = self.get_current_workspace().ok().map(|wt| wt.name);

        // Add to registry
        let mut registry = self.load_registry()?;
        registry.add(ShareEntry {
            name: name.to_string(),
            path: share_path.clone(),
            branch: branch_name.clone(),
            parent: parent.clone(),
        });
        self.save_registry(&registry)?;

        Ok(Worktree {
            name: name.to_string(),
            path: share_path,
            branch: branch_name,
            commit,
            parent,
        })
    }

    fn list_workspaces(&self) -> Result<Vec<Worktree>> {
        let registry = self.load_registry()?;
        let mut worktrees = Vec::new();

        // Add main repository
        let main_branch = self.get_current_branch(&self.repo_path)?;
        let main_commit = self.get_current_commit(&self.repo_path)?;
        let main_name = self
            .repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("main")
            .to_string();

        worktrees.push(Worktree {
            name: main_name,
            path: self.repo_path.clone(),
            branch: main_branch,
            commit: main_commit,
            parent: None,
        });

        // Add all shares from registry
        for entry in &registry.shares {
            if entry.path.exists() {
                let commit = self
                    .get_current_commit(&entry.path)
                    .unwrap_or_else(|_| "unknown".to_string());
                worktrees.push(Worktree {
                    name: entry.name.clone(),
                    path: entry.path.clone(),
                    branch: entry.branch.clone(),
                    commit,
                    parent: entry.parent.clone(),
                });
            }
        }

        Ok(worktrees)
    }

    fn remove_workspace(&self, name: &str, force: bool) -> Result<()> {
        let mut registry = self.load_registry()?;
        let entry = registry
            .find(name)
            .ok_or_else(|| HnError::WorktreeNotFound(name.to_string()))?;

        let share_path = entry.path.clone();

        // Check for uncommitted changes if not force
        if !force && !self.get_workspace_status(&share_path)?.is_clean() {
            return Err(HnError::ConfigError(format!(
                "Share '{}' has uncommitted changes. Use --force to remove anyway.",
                name
            )));
        }

        // Remove the directory
        if share_path.exists() {
            fs::remove_dir_all(&share_path)?;
        }

        // Remove from registry
        registry.remove(name);
        self.save_registry(&registry)?;

        Ok(())
    }

    fn get_workspace_by_name(&self, name: &str) -> Result<Worktree> {
        let registry = self.load_registry()?;
        let entry = registry
            .find(name)
            .ok_or_else(|| HnError::WorktreeNotFound(name.to_string()))?;

        let commit = self.get_current_commit(&entry.path)?;
        Ok(Worktree {
            name: entry.name.clone(),
            path: entry.path.clone(),
            branch: entry.branch.clone(),
            commit,
            parent: entry.parent.clone(),
        })
    }

    fn get_current_workspace(&self) -> Result<Worktree> {
        let current_dir = std::env::current_dir()?;
        let registry = self.load_registry()?;

        // Check if we're in a share
        if let Some(entry) = registry.find_by_path(&current_dir) {
            let commit = self.get_current_commit(&entry.path)?;
            return Ok(Worktree {
                name: entry.name.clone(),
                path: entry.path.clone(),
                branch: entry.branch.clone(),
                commit,
                parent: entry.parent.clone(),
            });
        }

        // We're in the main repository
        let main_branch = self.get_current_branch(&self.repo_path)?;
        let main_commit = self.get_current_commit(&self.repo_path)?;
        let main_name = self
            .repo_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("main")
            .to_string();

        Ok(Worktree {
            name: main_name,
            path: self.repo_path.clone(),
            branch: main_branch,
            commit: main_commit,
            parent: None,
        })
    }

    fn get_workspace_status(&self, worktree_path: &Path) -> Result<WorkspaceStatus> {
        let output = Command::new("hg")
            .arg("status")
            .current_dir(worktree_path)
            .output()?;

        if !output.status.success() {
            return Err(HnError::ConfigError(
                "Failed to get Mercurial status".to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut status = WorkspaceStatus {
            modified: 0,
            added: 0,
            deleted: 0,
            untracked: 0,
        };

        for line in stdout.lines() {
            if line.len() < 2 {
                continue;
            }

            let status_char = line.chars().next().unwrap();
            match status_char {
                'M' => status.modified += 1,
                'A' => status.added += 1,
                'R' => status.deleted += 1,
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

        // Check if sparse extension is available
        let check_output = Command::new("hg")
            .arg("help")
            .arg("sparse")
            .current_dir(worktree_path)
            .output()?;

        if !check_output.status.success() {
            return Err(HnError::ConfigError(
                "Sparse checkout requires the 'sparse' extension. Enable it in your .hgrc with:\n\
                 [extensions]\n\
                 sparse =".to_string(),
            ));
        }

        // Enable sparse checkout
        let enable_output = Command::new("hg")
            .arg("sparse")
            .arg("enable")
            .current_dir(worktree_path)
            .output()?;

        if !enable_output.status.success() {
            let stderr = String::from_utf8_lossy(&enable_output.stderr);
            return Err(HnError::ConfigError(format!(
                "Failed to enable sparse checkout: {}",
                stderr
            )));
        }

        // Include specified paths
        for path in paths {
            let include_output = Command::new("hg")
                .arg("sparse")
                .arg("include")
                .arg(path)
                .current_dir(worktree_path)
                .output()?;

            if !include_output.status.success() {
                let stderr = String::from_utf8_lossy(&include_output.stderr);
                return Err(HnError::ConfigError(format!(
                    "Failed to include sparse path '{}': {}",
                    path, stderr
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mercurial_backend_discovery() {
        let temp = tempfile::TempDir::new().unwrap();
        let repo_path = temp.path().join("hg-repo");
        fs::create_dir(&repo_path).unwrap();
        fs::create_dir(repo_path.join(".hg")).unwrap();

        let backend = MercurialBackend::discover_repo(&repo_path);
        assert!(backend.is_ok(), "Should discover Mercurial repo");

        let backend = backend.unwrap();
        assert_eq!(backend.vcs_type(), VcsType::Mercurial);
        assert_eq!(backend.repo_root().unwrap(), repo_path);
    }

    #[test]
    fn test_registry_operations() {
        let temp = tempfile::TempDir::new().unwrap();
        let registry_path = temp.path().join("registry.json");

        // Create new registry
        let mut registry = ShareRegistry::new();
        assert_eq!(registry.shares.len(), 0);

        // Add entries
        registry.add(ShareEntry {
            name: "feature-x".to_string(),
            path: PathBuf::from("/tmp/feature-x"),
            branch: "default".to_string(),
            parent: None,
        });

        registry.add(ShareEntry {
            name: "feature-y".to_string(),
            path: PathBuf::from("/tmp/feature-y"),
            branch: "default".to_string(),
            parent: Some("feature-x".to_string()),
        });

        assert_eq!(registry.shares.len(), 2);

        // Save and load
        registry.save(&registry_path).unwrap();
        let loaded = ShareRegistry::load(&registry_path).unwrap();
        assert_eq!(loaded.shares.len(), 2);

        // Find operations
        assert!(loaded.find("feature-x").is_some());
        assert!(loaded.find("nonexistent").is_none());

        // Remove operation
        let mut registry = loaded;
        assert!(registry.remove("feature-x"));
        assert_eq!(registry.shares.len(), 1);
        assert!(!registry.remove("nonexistent"));
    }
}
