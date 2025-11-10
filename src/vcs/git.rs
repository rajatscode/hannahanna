use crate::errors::{HnError, Result};
use crate::vcs::Worktree;
use git2::Repository;
use std::path::Path;

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
        let repo =
            Repository::discover(std::env::current_dir()?).map_err(|_| HnError::NotInRepository)?;
        Ok(Self { repo })
    }

    /// Create a new git worktree
    pub fn create_worktree(&self, name: &str, branch: Option<&str>, from: Option<&str>, no_branch: bool) -> Result<Worktree> {
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

        // Return the worktree info directly
        // Note: We construct this directly because libgit2's find_worktree doesn't see
        // worktrees created by external git commands without reloading
        Ok(Worktree {
            name: name.to_string(),
            path: worktree_path,
            branch: branch_name.to_string(),
            commit,
        })
    }

    /// Create worktree using git command (libgit2's worktree API is limited)
    fn create_worktree_via_command(&self, path: &Path, branch: &str, from: Option<&str>, no_branch: bool) -> Result<()> {
        use std::process::Command;

        // Get the repository's working directory
        let repo_workdir = self.repo.workdir().ok_or_else(|| {
            HnError::Git(git2::Error::from_str("Repository has no working directory"))
        })?;

        let mut cmd = Command::new("git");
        cmd.current_dir(repo_workdir)
            .arg("worktree")
            .arg("add");

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

                let output = fallback_cmd.output()?;

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
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    worktrees.push(Worktree {
                        name,
                        path,
                        branch,
                        commit,
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
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            worktrees.push(Worktree {
                name,
                path,
                branch,
                commit,
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
        cmd.current_dir(repo_workdir)
            .arg("worktree")
            .arg("remove");

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
                "Failed to get git status",
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
}
