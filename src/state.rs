use crate::errors::Result;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct StateManager {
    state_root: PathBuf,
}

impl StateManager {
    /// Create a new StateManager with the .hn-state directory in the repo root
    pub fn new(repo_root: &Path) -> Result<Self> {
        let state_root = repo_root.join(".hn-state");

        // Create state directory if it doesn't exist
        if !state_root.exists() {
            fs::create_dir_all(&state_root)?;

            // Create .gitignore to ignore all state files
            let gitignore_path = state_root.join(".gitignore");
            let mut gitignore = fs::File::create(gitignore_path)?;
            writeln!(gitignore, "*")?;
        }

        Ok(Self { state_root })
    }

    /// Create state directory for a worktree
    pub fn create_state_dir(&self, worktree_name: &str) -> Result<PathBuf> {
        let state_dir = self.state_root.join(worktree_name);

        // create_dir_all is idempotent, so no need to check if it exists first
        // This avoids TOCTOU race condition
        fs::create_dir_all(&state_dir)?;

        Ok(state_dir)
    }

    /// Remove state directory for a worktree
    pub fn remove_state_dir(&self, worktree_name: &str) -> Result<()> {
        let state_dir = self.state_root.join(worktree_name);

        if state_dir.exists() {
            fs::remove_dir_all(&state_dir)?;
        }

        Ok(())
    }

    /// List all worktrees that have state directories
    pub fn list_worktrees(&self) -> Result<Vec<String>> {
        let mut worktrees = Vec::new();

        if !self.state_root.exists() {
            return Ok(worktrees);
        }

        for entry in fs::read_dir(&self.state_root)? {
            let entry = entry?;
            let path = entry.path();

            // Skip .gitignore and non-directories
            if !path.is_dir() {
                continue;
            }

            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    worktrees.push(name_str.to_string());
                }
            }
        }

        Ok(worktrees)
    }

    /// List orphaned state directories (directories that don't have corresponding worktrees)
    pub fn list_orphaned(&self, active_worktrees: &[String]) -> Result<Vec<String>> {
        let mut orphaned = Vec::new();

        if !self.state_root.exists() {
            return Ok(orphaned);
        }

        for entry in fs::read_dir(&self.state_root)? {
            let entry = entry?;
            let path = entry.path();

            // Skip .gitignore and non-directories
            if !path.is_dir() {
                continue;
            }

            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // If this state directory doesn't have a corresponding active worktree, it's orphaned
                if !active_worktrees.contains(&name.to_string()) {
                    orphaned.push(name.to_string());
                }
            }
        }

        Ok(orphaned)
    }

    /// Clean orphaned state directories
    pub fn clean_orphaned(&self, active_worktrees: &[String]) -> Result<Vec<String>> {
        let orphaned = self.list_orphaned(active_worktrees)?;
        let mut cleaned = Vec::new();

        for name in orphaned {
            self.remove_state_dir(&name)?;
            cleaned.push(name);
        }

        Ok(cleaned)
    }

    /// Get the state directory path for a worktree (doesn't create it)
    pub fn get_state_dir(&self, worktree_name: &str) -> PathBuf {
        self.state_root.join(worktree_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_state_manager() {
        let temp = TempDir::new().unwrap();
        let _manager = StateManager::new(temp.path()).unwrap();

        // Check that .hn-state directory was created
        assert!(temp.path().join(".hn-state").exists());

        // Check that .gitignore was created
        assert!(temp.path().join(".hn-state/.gitignore").exists());

        // Check .gitignore contents
        let gitignore_content =
            fs::read_to_string(temp.path().join(".hn-state/.gitignore")).unwrap();
        assert_eq!(gitignore_content.trim(), "*");
    }

    #[test]
    fn test_create_and_remove_state_dir() {
        let temp = TempDir::new().unwrap();
        let manager = StateManager::new(temp.path()).unwrap();

        // Create state directory
        let state_dir = manager.create_state_dir("feature-x").unwrap();
        assert!(state_dir.exists());

        // Remove state directory
        manager.remove_state_dir("feature-x").unwrap();
        assert!(!state_dir.exists());
    }

    #[test]
    fn test_list_orphaned() {
        let temp = TempDir::new().unwrap();
        let manager = StateManager::new(temp.path()).unwrap();

        // Create some state directories
        manager.create_state_dir("feature-x").unwrap();
        manager.create_state_dir("feature-y").unwrap();
        manager.create_state_dir("feature-z").unwrap();

        // Say only feature-x and feature-y are active
        let active = vec!["feature-x".to_string(), "feature-y".to_string()];

        let orphaned = manager.list_orphaned(&active).unwrap();

        // feature-z should be orphaned
        assert_eq!(orphaned.len(), 1);
        assert!(orphaned.contains(&"feature-z".to_string()));
    }

    #[test]
    fn test_clean_orphaned() {
        let temp = TempDir::new().unwrap();
        let manager = StateManager::new(temp.path()).unwrap();

        // Create some state directories
        manager.create_state_dir("feature-x").unwrap();
        manager.create_state_dir("feature-y").unwrap();
        manager.create_state_dir("feature-z").unwrap();

        // Say only feature-x is active
        let active = vec!["feature-x".to_string()];

        let cleaned = manager.clean_orphaned(&active).unwrap();

        // feature-y and feature-z should be cleaned
        assert_eq!(cleaned.len(), 2);
        assert!(cleaned.contains(&"feature-y".to_string()));
        assert!(cleaned.contains(&"feature-z".to_string()));

        // Verify they were actually removed
        assert!(!temp.path().join(".hn-state/feature-y").exists());
        assert!(!temp.path().join(".hn-state/feature-z").exists());
        assert!(temp.path().join(".hn-state/feature-x").exists());
    }
}
