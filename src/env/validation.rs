use crate::errors::{HnError, Result};
use std::fs;
use std::path::Path;

/// Validate that a path is within repository boundaries
/// This uses a safer approach that validates after creation to avoid TOCTOU issues
/// For symlinks, this validates the symlink location itself, not where it points
pub fn validate_path_within_repo(path: &Path, repo_root: &Path) -> Result<()> {
    let canonical_repo = fs::canonicalize(repo_root)
        .map_err(|e| HnError::SymlinkError(format!("Cannot canonicalize repo root: {}", e)))?;

    // For symlinks, we want to check the symlink's location, not its target
    // So we canonicalize the parent directory and then append the filename
    let path_to_check = if path.is_symlink() {
        if let Some(parent) = path.parent() {
            let canonical_parent = fs::canonicalize(parent).map_err(|e| {
                HnError::SymlinkError(format!(
                    "Cannot canonicalize parent directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
            if let Some(filename) = path.file_name() {
                canonical_parent.join(filename)
            } else {
                path.to_path_buf()
            }
        } else {
            path.to_path_buf()
        }
    } else {
        // For non-symlinks, canonicalize normally
        fs::canonicalize(path).map_err(|e| {
            HnError::SymlinkError(format!(
                "Cannot canonicalize path {}: {}",
                path.display(),
                e
            ))
        })?
    };

    // Check if path is within repo boundaries
    if !path_to_check.starts_with(&canonical_repo) {
        return Err(HnError::SymlinkError(
            "Path is outside repository boundaries".to_string(),
        ));
    }

    Ok(())
}

/// Validate worktree name for safety and usability
pub fn validate_worktree_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(HnError::InvalidWorktreeName(
            "Worktree name cannot be empty".to_string(),
        ));
    }

    if name == "." || name == ".." {
        return Err(HnError::InvalidWorktreeName(
            "Worktree name cannot be '.' or '..'".to_string(),
        ));
    }

    if name.contains('/') || name.contains('\\') {
        return Err(HnError::InvalidWorktreeName(
            "Worktree name cannot contain path separators".to_string(),
        ));
    }

    if name.contains('\0') {
        return Err(HnError::InvalidWorktreeName(
            "Worktree name cannot contain null bytes".to_string(),
        ));
    }

    if name.starts_with('-') {
        return Err(HnError::InvalidWorktreeName(
            "Worktree name cannot start with '-' (ambiguous with command flags)".to_string(),
        ));
    }

    if name.starts_with('.') {
        return Err(HnError::InvalidWorktreeName(
            "Worktree name cannot start with '.' (hidden directory)".to_string(),
        ));
    }

    Ok(())
}

/// Ensure parent directory exists, creating it if necessary
pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_worktree_name_valid() {
        assert!(validate_worktree_name("feature-x").is_ok());
        assert!(validate_worktree_name("fix_bug").is_ok());
        assert!(validate_worktree_name("PR123").is_ok());
    }

    #[test]
    fn test_validate_worktree_name_invalid() {
        assert!(validate_worktree_name("").is_err());
        assert!(validate_worktree_name(".").is_err());
        assert!(validate_worktree_name("..").is_err());
        assert!(validate_worktree_name("feature/x").is_err());
        assert!(validate_worktree_name("feature\\x").is_err());
        assert!(validate_worktree_name("-feature").is_err());
        assert!(validate_worktree_name(".hidden").is_err());
    }

    #[test]
    fn test_validate_path_within_repo() {
        let temp = TempDir::new().unwrap();
        let repo_root = temp.path();
        let subdir = repo_root.join("subdir");
        fs::create_dir_all(&subdir).unwrap();

        // Path within repo should be valid
        assert!(validate_path_within_repo(&subdir, repo_root).is_ok());
    }

    #[test]
    fn test_validate_path_outside_repo() {
        let temp1 = TempDir::new().unwrap();
        let temp2 = TempDir::new().unwrap();

        let repo_root = temp1.path();
        let outside_path = temp2.path();

        // Path outside repo should fail
        assert!(validate_path_within_repo(outside_path, repo_root).is_err());
    }
}
