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

    // Restore original directory
    std::env::set_current_dir(original_dir).ok();

    backend_result
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
