use crate::errors::{HnError, Result};
use crate::vcs::Worktree;
use git2::Repository;
use std::path::{Path, PathBuf};

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
