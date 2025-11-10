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
