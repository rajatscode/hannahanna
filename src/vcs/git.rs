use crate::errors::{HnError, Result};
use crate::vcs::Worktree;
use git2::Repository;
use std::path::Path;
use std::process::Output;

/// Helper to extract meaningful error message from git command output
fn git_error_from_output(output: &Output, context: &str) -> HnError {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let exit_code = output.status.code().unwrap_or(-1);

    let error_msg = if !stderr.is_empty() {
        format!("{}: {}", context, stderr.trim())
    } else if !stdout.is_empty() {
        format!("{}: {}", context, stdout.trim())
    } else {
        format!("{} (exit code: {})", context, exit_code)
    };

    HnError::Git(git2::Error::from_str(&error_msg))
}

/// Git status information for a worktree
// Re-export the common WorkspaceStatus type for compatibility
pub use crate::vcs::WorkspaceStatus;

pub struct GitBackend {
    repo: Repository,
}

impl GitBackend {
    /// Open a git repository from the current directory
    pub fn open_from_current_dir() -> Result<Self> {
        let repo =
            Repository::discover(std::env::current_dir()?).map_err(|_| HnError::NotInRepository)?;
        Ok(Self { repo })
    }

    /// Open a git repository from a specific path
    #[allow(dead_code)] // Public API, may be used by external crates
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let repo = Repository::discover(path.as_ref()).map_err(|_| HnError::NotInRepository)?;
        Ok(Self { repo })
    }

    /// Get the repository root path
    pub fn repo_root(&self) -> Result<std::path::PathBuf> {
        Ok(self
            .repo
            .workdir()
            .ok_or_else(|| HnError::Git(git2::Error::from_str("Could not get repo workdir")))?
            .to_path_buf())
    }

    /// Create a new git worktree
    pub fn create_worktree(
        &self,
        name: &str,
        branch: Option<&str>,
        from: Option<&str>,
        no_branch: bool,
    ) -> Result<Worktree> {
        // Get the repository's worktree directory (parent of .git)
        let repo_path = self.repo.path().parent().ok_or_else(|| {
            HnError::Git(git2::Error::from_str("Could not determine repository path"))
        })?;

        // Determine the worktree path (sibling directory)
        let worktree_path = repo_path
            .parent()
            .ok_or_else(|| {
                HnError::Git(git2::Error::from_str(
                    "Could not determine worktree parent directory",
                ))
            })?
            .join(name);

        // Check if worktree already exists
        if worktree_path.exists() {
            return Err(HnError::WorktreeAlreadyExists(name.to_string()));
        }

        // Determine branch name (use provided branch or create new with same name as worktree)
        let branch_name = branch.unwrap_or(name);

        // Create the worktree using git command
        self.create_worktree_via_command(&worktree_path, branch_name, from, no_branch)?;

        // Get commit hash from the worktree
        let commit = if let Ok(wt_repo) = Repository::open(&worktree_path) {
            if let Ok(head) = wt_repo.head() {
                head.target()
                    .map(|oid| oid.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                "unknown".to_string()
            }
        } else {
            "unknown".to_string()
        };

        // Detect current worktree and set parent
        let parent = self.detect_and_set_parent(name, &worktree_path)?;

        // Return the worktree info directly
        // Note: We construct this directly because libgit2's find_worktree doesn't see
        // worktrees created by external git commands without reloading
        Ok(Worktree {
            name: name.to_string(),
            path: worktree_path,
            branch: branch_name.to_string(),
            commit,
            parent,
        })
    }

    /// Create worktree using git command (libgit2's worktree API is limited)
    fn create_worktree_via_command(
        &self,
        path: &Path,
        branch: &str,
        from: Option<&str>,
        no_branch: bool,
    ) -> Result<()> {
        use std::process::Command;

        // Get the repository's working directory
        let repo_workdir = self.repo.workdir().ok_or_else(|| {
            HnError::Git(git2::Error::from_str("Repository has no working directory"))
        })?;

        let mut cmd = Command::new("git");
        cmd.current_dir(repo_workdir).arg("worktree").arg("add");

        if no_branch {
            // Checkout existing branch without creating new one
            cmd.arg(path).arg(branch);
        } else {
            // Create new branch
            cmd.arg("-b").arg(branch).arg(path);

            // If from is specified, use it as the base
            if let Some(base_branch) = from {
                cmd.arg(base_branch);
            }
        }

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // If not using no_branch and branch already exists, try without -b flag
            if !no_branch && stderr.contains("already exists") {
                let mut fallback_cmd = Command::new("git");
                fallback_cmd
                    .current_dir(repo_workdir)
                    .arg("worktree")
                    .arg("add")
                    .arg(path)
                    .arg(branch);

                let fallback_output = fallback_cmd.output()?;

                if !fallback_output.status.success() {
                    return Err(git_error_from_output(
                        &fallback_output,
                        "Failed to create worktree",
                    ));
                }
            } else {
                return Err(git_error_from_output(&output, "Failed to create worktree"));
            }
        }

        Ok(())
    }

    /// List all git worktrees
    pub fn list_worktrees(&self) -> Result<Vec<Worktree>> {
        use std::process::Command;

        let repo_workdir = self.repo.workdir().ok_or_else(|| {
            HnError::Git(git2::Error::from_str("Repository has no working directory"))
        })?;

        // Use git worktree list --porcelain to get accurate list
        let output = Command::new("git")
            .current_dir(repo_workdir)
            .arg("worktree")
            .arg("list")
            .arg("--porcelain")
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::Git(git2::Error::from_str(&stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_worktree_list(&stdout)
    }

    /// Parse the output of git worktree list --porcelain
    fn parse_worktree_list(&self, output: &str) -> Result<Vec<Worktree>> {
        let mut worktrees = Vec::new();
        let mut current_worktree: Option<(std::path::PathBuf, String, String)> = None;

        for line in output.lines() {
            if line.starts_with("worktree ") {
                // Save previous worktree if any
                if let Some((path, branch, commit)) = current_worktree.take() {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let parent = self.get_parent(&path).ok();
                    worktrees.push(Worktree {
                        name,
                        path,
                        branch,
                        commit,
                        parent,
                    });
                }
                // Start new worktree
                let path = std::path::PathBuf::from(line.trim_start_matches("worktree "));
                current_worktree = Some((path, String::new(), String::new()));
            } else if line.starts_with("HEAD ") {
                if let Some((_, _, ref mut commit)) = current_worktree {
                    *commit = line.trim_start_matches("HEAD ").to_string();
                }
            } else if line.starts_with("branch ") {
                if let Some((_, ref mut branch, _)) = current_worktree {
                    let full_branch = line.trim_start_matches("branch ");
                    // Extract short branch name (remove refs/heads/)
                    *branch = full_branch
                        .strip_prefix("refs/heads/")
                        .unwrap_or(full_branch)
                        .to_string();
                }
            } else if line.starts_with("detached") {
                if let Some((_, ref mut branch, _)) = current_worktree {
                    *branch = "(detached)".to_string();
                }
            }
        }

        // Don't forget the last worktree
        if let Some((path, branch, commit)) = current_worktree.take() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let parent = self.get_parent(&path).ok();
            worktrees.push(Worktree {
                name,
                path,
                branch,
                commit,
                parent,
            });
        }

        Ok(worktrees)
    }

    /// Remove a git worktree
    pub fn remove_worktree(&self, name: &str, force: bool) -> Result<()> {
        use std::process::Command;

        // Get worktree info (also checks if it exists)
        let worktree_info = self.get_worktree_info(name)?;

        // If not force, check for uncommitted changes
        if !force {
            let has_changes = self.has_uncommitted_changes(&worktree_info.path)?;
            if has_changes {
                return Err(HnError::Git(git2::Error::from_str(&format!(
                    "Worktree '{}' has uncommitted changes. Use --force to remove anyway.",
                    name
                ))));
            }
        }

        // Remove the worktree using git command
        let repo_workdir = self.repo.workdir().ok_or_else(|| {
            HnError::Git(git2::Error::from_str("Repository has no working directory"))
        })?;

        let mut cmd = Command::new("git");
        cmd.current_dir(repo_workdir).arg("worktree").arg("remove");

        if force {
            cmd.arg("--force");
        }

        cmd.arg(&worktree_info.path);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::Git(git2::Error::from_str(&stderr)));
        }

        Ok(())
    }

    /// Check if a worktree has uncommitted changes
    fn has_uncommitted_changes(&self, worktree_path: &Path) -> Result<bool> {
        use std::process::Command;

        let output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .arg("status")
            .arg("--porcelain")
            .output()?;

        if !output.status.success() {
            return Err(HnError::Git(git2::Error::from_str(
                "Failed to check git status",
            )));
        }

        // If output is not empty, there are uncommitted changes
        Ok(!output.stdout.is_empty())
    }

    /// Get a worktree by name (public API for switch command)
    pub fn get_worktree_by_name(&self, name: &str) -> Result<Worktree> {
        self.get_worktree_info(name)
    }

    /// Get the current worktree based on the current directory (using env::current_dir())
    pub fn get_current_worktree(&self) -> Result<Worktree> {
        let current_dir = std::env::current_dir()?;
        self.get_current_worktree_from_path(&current_dir)
    }

    /// Get the current worktree based on a given directory path
    pub fn get_current_worktree_from_path(&self, current_dir: &Path) -> Result<Worktree> {
        // Get all worktrees and find the one containing current_dir
        let worktrees = self.list_worktrees()?;

        for wt in worktrees {
            if current_dir.starts_with(&wt.path) {
                return Ok(wt);
            }
        }

        Err(HnError::NotInRepository)
    }

    /// Get git status for a worktree
    pub fn get_worktree_status(&self, worktree_path: &Path) -> Result<WorkspaceStatus> {
        use std::process::Command;

        let output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .arg("status")
            .arg("--porcelain")
            .output()?;

        if !output.status.success() {
            return Err(HnError::Git(git2::Error::from_str(
                "Failed to get git status",
            )));
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

            let index_status = line.chars().nth(0).unwrap();
            let worktree_status = line.chars().nth(1).unwrap();

            // Check index status (staged changes)
            match index_status {
                'M' => status.modified += 1,
                'A' => status.added += 1,
                'D' => status.deleted += 1,
                _ => {}
            }

            // Check worktree status (unstaged changes)
            match worktree_status {
                'M' => {
                    if index_status == ' ' {
                        status.modified += 1;
                    }
                }
                'D' => {
                    if index_status == ' ' {
                        status.deleted += 1;
                    }
                }
                _ => {}
            }

            // Check for untracked files
            if index_status == '?' && worktree_status == '?' {
                status.untracked += 1;
            }
        }

        Ok(status)
    }

    /// Get the commit message for a worktree
    #[allow(dead_code)] // Public API, may be used by external crates
    pub fn get_commit_message(&self, worktree_path: &Path) -> Result<String> {
        let wt_repo = Repository::open(worktree_path)?;

        // Try to get HEAD, but handle the case where it doesn't exist yet
        let head = match wt_repo.head() {
            Ok(h) => h,
            Err(_) => return Ok("(no commits yet)".to_string()),
        };

        if let Some(commit_oid) = head.target() {
            let commit = wt_repo.find_commit(commit_oid)?;
            Ok(commit.message().unwrap_or("").to_string())
        } else {
            Ok("(no commit message)".to_string())
        }
    }

    /// Get information about a specific worktree
    fn get_worktree_info(&self, name: &str) -> Result<Worktree> {
        // Get all worktrees and find the one matching the name
        let worktrees = self.list_worktrees()?;

        worktrees
            .into_iter()
            .find(|wt| wt.name == name)
            .ok_or_else(|| HnError::WorktreeNotFound(name.to_string()))
    }

    /// Detect current worktree and set parent relationship
    fn detect_and_set_parent(&self, _name: &str, worktree_path: &Path) -> Result<Option<String>> {
        // Try to detect if we're currently in a worktree
        let current_dir = std::env::current_dir()?;

        // Check if current directory is in a worktree (not the main repo)
        if let Ok(current_worktree) = self.get_current_worktree_from_path(&current_dir) {
            // Check if this is the main repo (not a worktree)
            // In the main repo, .git is a directory. In worktrees, .git is a file.
            let git_path = current_worktree.path.join(".git");
            let is_main_repo = git_path.is_dir();

            // Only set parent if we're in an actual worktree, not the main repo
            if !is_main_repo {
                let parent_name = current_worktree.name.clone();
                self.set_parent(worktree_path, &parent_name)?;
                return Ok(Some(parent_name));
            }
        }

        // Not in a worktree, or in main repo - no parent
        Ok(None)
    }

    /// Set the parent worktree using git config
    fn set_parent(&self, worktree_path: &Path, parent_name: &str) -> Result<()> {
        use std::process::Command;

        let output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .arg("config")
            .arg("worktree.parent")
            .arg(parent_name)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::Git(git2::Error::from_str(&format!(
                "Failed to set parent config: {}",
                stderr
            ))));
        }

        Ok(())
    }

    /// Get the parent worktree from git config
    fn get_parent(&self, worktree_path: &Path) -> Result<String> {
        use std::process::Command;

        let output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .arg("config")
            .arg("--get")
            .arg("worktree.parent")
            .output()?;

        if output.status.success() {
            let parent = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !parent.is_empty() {
                return Ok(parent);
            }
        }

        Err(HnError::Git(git2::Error::from_str("No parent config")))
    }

    /// Parse git version string (e.g., "git version 2.34.1" -> (2, 34, 1))
    fn parse_git_version(version_str: &str) -> Option<(u32, u32, u32)> {
        // Extract version numbers from "git version X.Y.Z"
        let parts: Vec<&str> = version_str.split_whitespace().collect();
        let version_part = parts.get(2)?;

        let nums: Vec<&str> = version_part.split('.').collect();
        if nums.len() < 2 {
            return None;
        }

        let major = nums[0].parse::<u32>().ok()?;
        let minor = nums[1].parse::<u32>().ok()?;
        let patch = nums.get(2).and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);

        Some((major, minor, patch))
    }
}

// ===== VcsBackend trait implementation =====

use crate::vcs::traits::{VcsBackend, VcsType};

impl VcsBackend for GitBackend {
    fn vcs_type(&self) -> VcsType {
        VcsType::Git
    }

    fn repo_root(&self) -> Result<std::path::PathBuf> {
        self.repo_root()
    }

    fn create_workspace(
        &self,
        name: &str,
        branch: Option<&str>,
        from: Option<&str>,
        no_branch: bool,
    ) -> Result<Worktree> {
        self.create_worktree(name, branch, from, no_branch)
    }

    fn list_workspaces(&self) -> Result<Vec<Worktree>> {
        self.list_worktrees()
    }

    fn remove_workspace(&self, name: &str, force: bool) -> Result<()> {
        self.remove_worktree(name, force)
    }

    fn get_workspace_by_name(&self, name: &str) -> Result<Worktree> {
        self.get_worktree_by_name(name)
    }

    fn get_current_workspace(&self) -> Result<Worktree> {
        self.get_current_worktree()
    }

    fn get_workspace_status(&self, worktree_path: &Path) -> Result<WorkspaceStatus> {
        self.get_worktree_status(worktree_path)
    }

    fn setup_sparse_checkout(&self, worktree_path: &Path, paths: &[String]) -> Result<()> {
        use std::process::Command;

        if paths.is_empty() {
            return Ok(());
        }

        // Check Git version (sparse-checkout requires Git >= 2.25)
        let version_output = Command::new("git")
            .args(["--version"])
            .output()?;

        if version_output.status.success() {
            let version_str = String::from_utf8_lossy(&version_output.stdout);
            if let Some(version) = Self::parse_git_version(&version_str) {
                if version < (2, 25, 0) {
                    return Err(HnError::Git(git2::Error::from_str(&format!(
                        "Sparse checkout requires Git >= 2.25.0 (found {})",
                        version_str.trim()
                    ))));
                }
            }
        }

        // Initialize sparse-checkout in cone mode (more efficient)
        let init_output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .args(["sparse-checkout", "init", "--cone"])
            .output()?;

        if !init_output.status.success() {
            let stderr = String::from_utf8_lossy(&init_output.stderr);
            return Err(HnError::Git(git2::Error::from_str(&format!(
                "Failed to initialize sparse checkout: {}",
                stderr
            ))));
        }

        // Set sparse checkout paths
        let mut set_cmd = Command::new("git");
        set_cmd
            .arg("-C")
            .arg(worktree_path)
            .args(["sparse-checkout", "set"]);

        // Add all paths
        for path in paths {
            set_cmd.arg(path);
        }

        let set_output = set_cmd.output()?;

        if !set_output.status.success() {
            let stderr = String::from_utf8_lossy(&set_output.stderr);
            return Err(HnError::Git(git2::Error::from_str(&format!(
                "Failed to set sparse checkout paths: {}",
                stderr
            ))));
        }

        Ok(())
    }
}
