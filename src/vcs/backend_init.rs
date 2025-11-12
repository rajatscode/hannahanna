/// VCS backend initialization helpers
/// Centralizes logic for creating backends with auto-detection or explicit VCS type
use crate::errors::{HnError, Result};
use crate::vcs::traits::{create_backend_at_path, detect_vcs_type, VcsBackend, VcsType};
use std::path::Path;

/// Initialize a VCS backend from the current directory with auto-detection
pub fn init_backend_from_current_dir() -> Result<Box<dyn VcsBackend>> {
    let cwd = std::env::current_dir()?;
    init_backend_with_detection(&cwd, None)
}

/// Initialize a VCS backend with optional explicit VCS type
/// If vcs_type is None, auto-detects the VCS type
///
/// # Thread Safety
/// This function is thread-safe. It does not change the process-global current directory.
///
/// # Errors
/// Returns an error if:
/// - VCS type cannot be detected (when vcs_type is None)
/// - Backend creation fails
pub fn init_backend_with_detection(
    path: &Path,
    vcs_type: Option<VcsType>,
) -> Result<Box<dyn VcsBackend>> {
    // Use explicit VCS type if provided, otherwise auto-detect
    let detected_vcs = match vcs_type {
        Some(vcs) => vcs,
        None => detect_vcs_type(path).ok_or(HnError::NotInRepository)?,
    };

    // Create backend at the specified path without changing cwd
    create_backend_at_path(detected_vcs, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    #[test]
    fn test_detect_no_vcs() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let result = init_backend_with_detection(temp.path(), None);

        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("Not in a"));
        }
    }

    #[test]
    #[serial]
    fn test_explicit_vcs_type_git() {
        // This test requires a git repo to be initialized
        let temp = TempDir::new().expect("Failed to create temp dir");
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to init git");

        let result = init_backend_with_detection(temp.path(), Some(VcsType::Git));
        assert!(result.is_ok());

        let backend = result.unwrap();
        assert_eq!(backend.vcs_type(), VcsType::Git);
    }

    #[test]
    #[serial]
    fn test_auto_detect_git() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to init git");

        let result = init_backend_with_detection(temp.path(), None);
        assert!(result.is_ok());

        let backend = result.unwrap();
        assert_eq!(backend.vcs_type(), VcsType::Git);
    }

    #[test]
    #[serial]
    fn test_no_cwd_change_on_success() {
        // Verify that current directory is NOT changed during backend creation
        let temp = TempDir::new().expect("Failed to create temp dir");
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to init git");

        let original_dir = std::env::current_dir().expect("Failed to get current dir");

        // Create backend from temp directory
        let result = init_backend_with_detection(temp.path(), Some(VcsType::Git));
        assert!(result.is_ok());

        // Verify we're still in the same directory (no change)
        let current_dir = std::env::current_dir().expect("Failed to get current dir");
        assert_eq!(
            current_dir, original_dir,
            "Directory should not change during backend creation"
        );
    }

    #[test]
    #[serial]
    fn test_no_cwd_change_on_failure() {
        // Verify directory is not changed even when backend creation fails
        let temp = TempDir::new().expect("Failed to create temp dir");
        // Don't initialize git, so backend creation will fail

        let original_dir = std::env::current_dir().expect("Failed to get current dir");

        // Try to create backend (will fail - not a git repo)
        let result = init_backend_with_detection(temp.path(), Some(VcsType::Git));
        assert!(
            result.is_err(),
            "Backend creation should fail for non-git directory"
        );

        // Verify we're still in the same directory
        let current_dir = std::env::current_dir().expect("Failed to get current dir");
        assert_eq!(
            current_dir, original_dir,
            "Directory should not change even on backend creation failure"
        );
    }
}
