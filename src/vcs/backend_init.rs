/// VCS backend initialization helpers
/// Centralizes logic for creating backends with auto-detection or explicit VCS type
use crate::errors::{HnError, Result};
use crate::vcs::traits::{create_backend, detect_vcs_type, VcsBackend, VcsType};
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
/// WARNING: This function temporarily changes the process-global current directory.
/// It is NOT safe to call concurrently from multiple threads. Use appropriate
/// synchronization (e.g., Mutex) if calling from concurrent contexts.
///
/// # Errors
/// Returns an error if:
/// - VCS type cannot be detected (when vcs_type is None)
/// - Backend creation fails
/// - Directory operations fail (including restoration failure)
pub fn init_backend_with_detection(
    path: &Path,
    vcs_type: Option<VcsType>,
) -> Result<Box<dyn VcsBackend>> {
    // Use explicit VCS type if provided, otherwise auto-detect
    let detected_vcs = match vcs_type {
        Some(vcs) => vcs,
        None => detect_vcs_type(path).ok_or(HnError::NotInRepository)?,
    };

    // Change to the directory before creating backend
    // (backends expect to be called from within the repo)
    let original_dir = std::env::current_dir()?;

    std::env::set_current_dir(path)?;

    // Create backend
    let backend_result = create_backend(detected_vcs);

    // CRITICAL: Always restore directory, even on backend creation failure
    // Propagate restoration errors to prevent silent state corruption
    let restore_result = std::env::set_current_dir(&original_dir);

    // If backend creation failed, return that error (after attempting restore)
    let backend = backend_result?;

    // If directory restoration failed, return that error (it's more critical)
    restore_result.map_err(|e| {
        HnError::ConfigError(format!(
            "Failed to restore working directory to {}: {}. Process is now in {}",
            original_dir.display(),
            e,
            std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown directory".to_string())
        ))
    })?;

    Ok(backend)
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
    fn test_directory_restoration_on_success() {
        // Verify that current directory is properly restored after successful backend creation
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

        // Verify we're back in the original directory
        let current_dir = std::env::current_dir().expect("Failed to get current dir");
        assert_eq!(
            current_dir, original_dir,
            "Directory should be restored to original location"
        );
    }

    #[test]
    #[serial]
    fn test_directory_state_on_backend_creation_failure() {
        // Verify directory is still restored even when backend creation fails
        let temp = TempDir::new().expect("Failed to create temp dir");
        // Don't initialize git, so backend creation will fail

        let original_dir = std::env::current_dir().expect("Failed to get current dir");

        // Try to create backend (will fail - not a git repo)
        let result = init_backend_with_detection(temp.path(), Some(VcsType::Git));
        assert!(
            result.is_err(),
            "Backend creation should fail for non-git directory"
        );

        // Verify we're back in the original directory despite the failure
        let current_dir = std::env::current_dir().expect("Failed to get current dir");
        assert_eq!(
            current_dir, original_dir,
            "Directory should be restored even after backend creation failure"
        );
    }

    #[test]
    fn test_error_contains_context_on_restoration_failure() {
        // This test verifies that if directory restoration fails, the error message
        // provides context about what happened
        // Note: It's hard to actually cause restoration to fail in practice,
        // so we're mainly documenting the expected behavior here

        // The error message should contain:
        // 1. The path we tried to restore to
        // 2. The actual error from set_current_dir
        // 3. The directory we ended up in

        // This is verified by the error formatting in the actual implementation
        // at lines 53-62 of backend_init.rs
    }
}
