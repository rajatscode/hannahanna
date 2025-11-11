use crate::config::SharedResource;
use crate::env::compatibility::CompatibilityChecker;
use crate::env::validation;
use crate::errors::Result;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
#[allow(dead_code)]
pub enum SymlinkAction {
    Created { source: PathBuf, target: PathBuf },
    Skipped { resource: String, reason: String },
}

pub struct SymlinkManager;

impl SymlinkManager {
    /// Setup symlinks for shared resources in a worktree
    pub fn setup(
        shared_resources: &[SharedResource],
        main_repo: &Path,
        worktree: &Path,
    ) -> Result<Vec<SymlinkAction>> {
        let mut actions = Vec::new();

        for resource in shared_resources {
            match Self::setup_symlink(resource, main_repo, worktree) {
                Ok(action) => actions.push(action),
                Err(e) => {
                    // Log error but continue with other symlinks
                    actions.push(SymlinkAction::Skipped {
                        resource: resource.source.clone(),
                        reason: format!("Error: {}", e),
                    });
                }
            }
        }

        Ok(actions)
    }

    /// Setup a single symlink
    fn setup_symlink(
        resource: &SharedResource,
        main_repo: &Path,
        worktree: &Path,
    ) -> Result<SymlinkAction> {
        let source_path = main_repo.join(&resource.source);
        let target_path = worktree.join(&resource.target);

        // Check compatibility if configured
        if let Some(ref lockfile) = resource.compatibility {
            let compatible =
                CompatibilityChecker::is_compatible_fast(lockfile, main_repo, worktree)?;

            if !compatible {
                return Ok(SymlinkAction::Skipped {
                    resource: resource.source.clone(),
                    reason: format!("Incompatible lockfile: {}", lockfile),
                });
            }
        }

        // Check if source exists
        if !source_path.exists() {
            return Ok(SymlinkAction::Skipped {
                resource: resource.source.clone(),
                reason: "Source does not exist in main repository".to_string(),
            });
        }

        // If target already exists, skip
        if target_path.exists() {
            return Ok(SymlinkAction::Skipped {
                resource: resource.source.clone(),
                reason: "Target already exists".to_string(),
            });
        }

        // Create parent directory if needed
        validation::ensure_parent_dir(&target_path)?;

        // Create symlink first
        unix_fs::symlink(&source_path, &target_path)?;

        // Then validate the created symlink is within repo boundaries (TOCTOU-safe)
        // If validation fails, we clean up the symlink
        if let Err(e) = validation::validate_path_within_repo(&target_path, worktree) {
            // Clean up the symlink we just created
            let _ = fs::remove_file(&target_path);
            return Err(e);
        }

        // Also validate source is within main repo
        if let Err(e) = validation::validate_path_within_repo(&source_path, main_repo) {
            // Clean up the symlink
            let _ = fs::remove_file(&target_path);
            return Err(e);
        }

        Ok(SymlinkAction::Created {
            source: source_path,
            target: target_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SharedResource;
    use tempfile::TempDir;

    #[test]
    fn test_create_symlink_compatible() {
        let temp = TempDir::new().unwrap();
        let main_dir = temp.path().join("main");
        let wt_dir = temp.path().join("worktree");
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&wt_dir).unwrap();

        // Create source directory
        fs::create_dir_all(main_dir.join("node_modules")).unwrap();

        // Create identical lockfiles
        let lockfile_content = "dependencies: foo@1.0.0";
        fs::write(main_dir.join("package-lock.json"), lockfile_content).unwrap();
        fs::write(wt_dir.join("package-lock.json"), lockfile_content).unwrap();

        let resource = SharedResource {
            source: "node_modules".to_string(),
            target: "node_modules".to_string(),
            compatibility: Some("package-lock.json".to_string()),
        };

        let actions = SymlinkManager::setup(&[resource], &main_dir, &wt_dir).unwrap();

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            SymlinkAction::Created { .. } => {}
            _ => panic!("Expected symlink to be created"),
        }

        // Verify symlink was created
        assert!(wt_dir.join("node_modules").exists());
    }

    #[test]
    fn test_skip_incompatible_lockfile() {
        let temp = TempDir::new().unwrap();
        let main_dir = temp.path().join("main");
        let wt_dir = temp.path().join("worktree");
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&wt_dir).unwrap();

        // Create source directory
        fs::create_dir_all(main_dir.join("node_modules")).unwrap();

        // Create different lockfiles
        fs::write(
            main_dir.join("package-lock.json"),
            "dependencies: foo@1.0.0",
        )
        .unwrap();
        fs::write(wt_dir.join("package-lock.json"), "dependencies: foo@2.0.0").unwrap();

        let resource = SharedResource {
            source: "node_modules".to_string(),
            target: "node_modules".to_string(),
            compatibility: Some("package-lock.json".to_string()),
        };

        let actions = SymlinkManager::setup(&[resource], &main_dir, &wt_dir).unwrap();

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            SymlinkAction::Skipped { reason, .. } => {
                assert!(reason.contains("Incompatible lockfile"));
            }
            _ => panic!("Expected symlink to be skipped"),
        }
    }

    #[test]
    fn test_reject_traversal() {
        let temp = TempDir::new().unwrap();
        let main_dir = temp.path().join("main");
        let wt_dir = temp.path().join("worktree");
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&wt_dir).unwrap();

        // Try to create a symlink pointing outside repo
        let outside_dir = temp.path().join("outside");
        fs::create_dir_all(&outside_dir).unwrap();

        let resource = SharedResource {
            source: "../outside".to_string(),
            target: "node_modules".to_string(),
            compatibility: None,
        };

        let result = SymlinkManager::setup(&[resource], &main_dir, &wt_dir);

        // Should either fail or skip with error message
        match result {
            Ok(actions) => match &actions[0] {
                SymlinkAction::Skipped { reason, .. } => {
                    assert!(reason.contains("Error"));
                }
                _ => panic!("Expected symlink to be skipped or error"),
            },
            Err(_) => {
                // Also acceptable - operation failed entirely
            }
        }
    }
}
