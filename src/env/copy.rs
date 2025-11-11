use crate::config::CopyResource;
use crate::errors::{HnError, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
#[allow(dead_code)]
pub enum CopyAction {
    Copied { source: PathBuf, target: PathBuf },
    Skipped { resource: String, reason: String },
}

pub struct CopyManager;

impl CopyManager {
    /// Setup file copies for shared resources in a worktree
    pub fn setup(
        copy_resources: &[CopyResource],
        main_repo: &Path,
        worktree: &Path,
    ) -> Result<Vec<CopyAction>> {
        let mut actions = Vec::new();

        for resource in copy_resources {
            match Self::copy_file(resource, main_repo, worktree) {
                Ok(action) => actions.push(action),
                Err(e) => {
                    // Log error but continue with other copies
                    actions.push(CopyAction::Skipped {
                        resource: resource.source.clone(),
                        reason: format!("Error: {}", e),
                    });
                }
            }
        }

        Ok(actions)
    }

    /// Copy a single file
    fn copy_file(resource: &CopyResource, main_repo: &Path, worktree: &Path) -> Result<CopyAction> {
        let source_path = main_repo.join(&resource.source);
        let target_path = worktree.join(&resource.target);

        // Validate paths for security
        Self::validate_copy_target(&source_path, main_repo)?;
        Self::validate_copy_target(&target_path, worktree)?;

        // Check if source exists
        if !source_path.exists() {
            return Ok(CopyAction::Skipped {
                resource: resource.source.clone(),
                reason: "Source does not exist in main repository".to_string(),
            });
        }

        // Check if source is a file (we only copy files, not directories)
        if !source_path.is_file() {
            return Ok(CopyAction::Skipped {
                resource: resource.source.clone(),
                reason: "Source is not a file (only files can be copied)".to_string(),
            });
        }

        // If target already exists, skip
        if target_path.exists() {
            return Ok(CopyAction::Skipped {
                resource: resource.source.clone(),
                reason: "Target already exists".to_string(),
            });
        }

        // Create parent directory if needed
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Copy the file
        fs::copy(&source_path, &target_path)?;

        Ok(CopyAction::Copied {
            source: source_path,
            target: target_path,
        })
    }

    /// Validate that a copy target is within the repository boundaries
    /// Prevents path traversal attacks
    fn validate_copy_target(target: &Path, repo_root: &Path) -> Result<()> {
        // Canonicalize paths to resolve any .. or symlinks
        let canonical_repo = fs::canonicalize(repo_root)
            .map_err(|e| HnError::SymlinkError(format!("Cannot canonicalize repo root: {}", e)))?;

        // For the target, we need to check the parent directory since the target might not exist yet
        let target_to_check = if target.exists() {
            target.to_path_buf()
        } else {
            // Check the parent directory
            target
                .parent()
                .ok_or_else(|| HnError::SymlinkError("Invalid target path".to_string()))?
                .to_path_buf()
        };

        let canonical_target = fs::canonicalize(&target_to_check)
            .map_err(|e| HnError::SymlinkError(format!("Cannot canonicalize target: {}", e)))?;

        // Check if target is within repo boundaries
        if !canonical_target.starts_with(&canonical_repo) {
            return Err(HnError::SymlinkError(
                "Copy target is outside repository boundaries".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CopyResource;
    use tempfile::TempDir;

    #[test]
    fn test_copy_file() {
        let temp = TempDir::new().unwrap();
        let main_dir = temp.path().join("main");
        let wt_dir = temp.path().join("worktree");
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&wt_dir).unwrap();

        // Create source file
        fs::write(main_dir.join(".env.template"), "DATABASE_URL=test").unwrap();

        let resource = CopyResource {
            source: ".env.template".to_string(),
            target: ".env".to_string(),
        };

        let actions = CopyManager::setup(&[resource], &main_dir, &wt_dir).unwrap();

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            CopyAction::Copied { .. } => {}
            _ => panic!("Expected file to be copied"),
        }

        // Verify file was copied
        assert!(wt_dir.join(".env").exists());
        let content = fs::read_to_string(wt_dir.join(".env")).unwrap();
        assert_eq!(content, "DATABASE_URL=test");
    }

    #[test]
    fn test_skip_missing_source() {
        let temp = TempDir::new().unwrap();
        let main_dir = temp.path().join("main");
        let wt_dir = temp.path().join("worktree");
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&wt_dir).unwrap();

        let resource = CopyResource {
            source: ".env.template".to_string(),
            target: ".env".to_string(),
        };

        let actions = CopyManager::setup(&[resource], &main_dir, &wt_dir).unwrap();

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            CopyAction::Skipped { reason, .. } => {
                assert!(reason.contains("does not exist"));
            }
            _ => panic!("Expected copy to be skipped"),
        }
    }

    #[test]
    fn test_skip_existing_target() {
        let temp = TempDir::new().unwrap();
        let main_dir = temp.path().join("main");
        let wt_dir = temp.path().join("worktree");
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&wt_dir).unwrap();

        // Create both source and target
        fs::write(main_dir.join(".env.template"), "DATABASE_URL=test").unwrap();
        fs::write(wt_dir.join(".env"), "EXISTING=true").unwrap();

        let resource = CopyResource {
            source: ".env.template".to_string(),
            target: ".env".to_string(),
        };

        let actions = CopyManager::setup(&[resource], &main_dir, &wt_dir).unwrap();

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            CopyAction::Skipped { reason, .. } => {
                assert!(reason.contains("already exists"));
            }
            _ => panic!("Expected copy to be skipped"),
        }

        // Verify original content wasn't overwritten
        let content = fs::read_to_string(wt_dir.join(".env")).unwrap();
        assert_eq!(content, "EXISTING=true");
    }

    #[test]
    fn test_skip_directory() {
        let temp = TempDir::new().unwrap();
        let main_dir = temp.path().join("main");
        let wt_dir = temp.path().join("worktree");
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&wt_dir).unwrap();

        // Create a directory instead of a file
        fs::create_dir_all(main_dir.join("config")).unwrap();

        let resource = CopyResource {
            source: "config".to_string(),
            target: "config".to_string(),
        };

        let actions = CopyManager::setup(&[resource], &main_dir, &wt_dir).unwrap();

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            CopyAction::Skipped { reason, .. } => {
                assert!(reason.contains("not a file"));
            }
            _ => panic!("Expected copy to be skipped"),
        }
    }
}
