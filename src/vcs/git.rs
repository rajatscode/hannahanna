use crate::errors::{HnError, Result};
use crate::vcs::Worktree;
use git2::Repository;
use std::path::{Path, PathBuf};

/// Git status information for a worktree
#[derive(Debug, Clone)]
pub struct WorktreeStatus {
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub untracked: usize,
}

impl WorktreeStatus {
    pub fn is_clean(&self) -> bool {
        self.modified == 0 && self.added == 0 && self.deleted == 0 && self.untracked == 0
    }
}

pub struct GitBackend {
    repo: Repository,
}

impl GitBackend {
    /// Open a git repository from the current directory
    pub fn open_from_current_dir() -> Result<Self> {
        let repo = Repository::discover(std::env::current_dir()?)
            .map_err(|_| HnError::NotInRepository)?;
        Ok(Self { repo })
    }

    /// Create a new git worktree
    pub fn create_worktree(&self, name: &str, branch: Option<&str>) -> Result<Worktree> {
        // Get the repository's worktree directory (parent of .git)
        let repo_path = self.repo.path().parent().ok_or_else(|| {
            HnError::Git(git2::Error::from_str("Could not determine repository path"))
        })?;

        // Determine the worktree path (sibling directory)
        let worktree_path = repo_path
            .parent()
            .ok_or_else(|| {
                HnError::Git(git2::Error::from_str("Could not determine worktree parent directory"))
            })?
            .join(name);

        // Check if worktree already exists
        if worktree_path.exists() {
            return Err(HnError::WorktreeAlreadyExists(name.to_string()));
        }

        // Determine branch name (use provided branch or create new with same name as worktree)
        let branch_name = branch.unwrap_or(name);

        // Check if we need to create a new branch
        let branch_ref = format!("refs/heads/{}", branch_name);
        let branch_exists = self.repo.find_reference(&branch_ref).is_ok();

        // If branch doesn't exist, we'll create it from HEAD
        let commit_id = if !branch_exists {
            // Get the current HEAD commit
            let head = self.repo.head()?;
            head.target().ok_or_else(|| {
                HnError::Git(git2::Error::from_str("HEAD does not point to a commit"))
            })?
        } else {
            // Branch exists, get its commit
            let branch_ref = self.repo.find_reference(&branch_ref)?;
            branch_ref.target().ok_or_else(|| {
                HnError::Git(git2::Error::from_str("Branch does not point to a commit"))
            })?
        };

        // Create the worktree using libgit2
        // Note: libgit2's worktree API is limited, so we'll use git command for now
        self.create_worktree_via_command(name, &worktree_path, branch_name)?;

        // Return the worktree info
        self.get_worktree_info(name)
    }

    /// Create worktree using git command (libgit2's worktree API is limited)
    fn create_worktree_via_command(
        &self,
        name: &str,
        path: &Path,
        branch: &str,
    ) -> Result<()> {
        use std::process::Command;

        let output = Command::new("git")
            .arg("worktree")
            .arg("add")
            .arg("-b")
            .arg(branch)
            .arg(path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Check if branch already exists error
            if stderr.contains("already exists") {
                // Try without -b flag (checkout existing branch)
                let output = Command::new("git")
                    .arg("worktree")
                    .arg("add")
                    .arg(path)
                    .arg(branch)
                    .output()?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(HnError::Git(git2::Error::from_str(&stderr)));
                }
            } else {
                return Err(HnError::Git(git2::Error::from_str(&stderr)));
            }
        }

        Ok(())
    }

    /// List all git worktrees
    pub fn list_worktrees(&self) -> Result<Vec<Worktree>> {
        let mut worktrees = Vec::new();

        // Get the list of worktree names from git
        let worktree_names = self.repo.worktrees()?;

        for name_bytes in worktree_names.iter() {
            if let Some(name) = name_bytes {
                if let Ok(wt) = self.get_worktree_info(name) {
                    worktrees.push(wt);
                }
            }
        }

        Ok(worktrees)
    }

    /// Remove a git worktree
    pub fn remove_worktree(&self, name: &str, force: bool) -> Result<()> {
        use std::process::Command;

        // Check if worktree exists
        if self.repo.find_worktree(name).is_err() {
            return Err(HnError::WorktreeNotFound(name.to_string()));
        }

        // Get worktree path for checking uncommitted changes
        let worktree_info = self.get_worktree_info(name)?;

        // If not force, check for uncommitted changes
        if !force {
            let has_changes = self.has_uncommitted_changes(&worktree_info.path)?;
            if has_changes {
                return Err(HnError::Git(git2::Error::from_str(
                    &format!("Worktree '{}' has uncommitted changes. Use --force to remove anyway.", name)
                )));
            }
        }

        // Remove the worktree using git command
        let mut cmd = Command::new("git");
        cmd.arg("worktree").arg("remove");

        if force {
            cmd.arg("--force");
        }

        cmd.arg(name);

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
                "Failed to check git status"
            )));
        }

        // If output is not empty, there are uncommitted changes
        Ok(!output.stdout.is_empty())
    }

    /// Get a worktree by name (public API for switch command)
    pub fn get_worktree_by_name(&self, name: &str) -> Result<Worktree> {
        self.get_worktree_info(name)
    }

    /// Get the current worktree based on the current directory
    pub fn get_current_worktree(&self, current_dir: &Path) -> Result<Worktree> {
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
    pub fn get_worktree_status(&self, worktree_path: &Path) -> Result<WorktreeStatus> {
        use std::process::Command;

        let output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .arg("status")
            .arg("--porcelain")
            .output()?;

        if !output.status.success() {
            return Err(HnError::Git(git2::Error::from_str(
                "Failed to get git status"
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut status = WorktreeStatus {
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
    pub fn get_commit_message(&self, worktree_path: &Path) -> Result<String> {
        let wt_repo = Repository::open(worktree_path)?;
        let head = wt_repo.head()?;

        if let Some(commit_oid) = head.target() {
            let commit = wt_repo.find_commit(commit_oid)?;
            Ok(commit.message().unwrap_or("").to_string())
        } else {
            Ok("(no commit message)".to_string())
        }
    }

    /// Get information about a specific worktree
    fn get_worktree_info(&self, name: &str) -> Result<Worktree> {
        let worktree = self.repo.find_worktree(name)?;
        let path = worktree.path().to_path_buf();

        // Open the worktree's repository to get branch and commit info
        let wt_repo = Repository::open(&path)?;

        // Get the branch name
        let head = wt_repo.head()?;
        let branch = if head.is_branch() {
            head.shorthand().unwrap_or("(detached)").to_string()
        } else {
            "(detached)".to_string()
        };

        // Get the commit hash
        let commit = head
            .target()
            .map(|oid| oid.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(Worktree {
            name: name.to_string(),
            path,
            branch,
            commit,
        })
    }
}
