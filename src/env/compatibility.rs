use crate::errors::Result;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub struct CompatibilityChecker;

impl CompatibilityChecker {
    /// Check if two directories are compatible by comparing their lockfiles
    /// Returns true if lockfiles are identical (safe to share resources)
    pub fn is_compatible(
        lockfile_name: &str,
        main_repo: &Path,
        worktree: &Path,
    ) -> Result<bool> {
        let main_lockfile = main_repo.join(lockfile_name);
        let worktree_lockfile = worktree.join(lockfile_name);

        // If either lockfile doesn't exist, consider incompatible
        if !main_lockfile.exists() || !worktree_lockfile.exists() {
            return Ok(false);
        }

        // Compare file hashes
        let main_hash = Self::compute_file_hash(&main_lockfile)?;
        let worktree_hash = Self::compute_file_hash(&worktree_lockfile)?;

        Ok(main_hash == worktree_hash)
    }

    /// Compute SHA256 hash of a file
    fn compute_file_hash(path: &Path) -> Result<String> {
        let content = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Fast compatibility check - first compares file size, then first 1KB, then full hash
    pub fn is_compatible_fast(
        lockfile_name: &str,
        main_repo: &Path,
        worktree: &Path,
    ) -> Result<bool> {
        let main_lockfile = main_repo.join(lockfile_name);
        let worktree_lockfile = worktree.join(lockfile_name);

        // If either lockfile doesn't exist, consider incompatible
        if !main_lockfile.exists() || !worktree_lockfile.exists() {
            return Ok(false);
        }

        // Quick check: compare file sizes first
        let main_metadata = fs::metadata(&main_lockfile)?;
        let worktree_metadata = fs::metadata(&worktree_lockfile)?;

        if main_metadata.len() != worktree_metadata.len() {
            return Ok(false);
        }

        // If files are small (< 1KB), compare full contents
        if main_metadata.len() < 1024 {
            return Self::is_compatible(lockfile_name, main_repo, worktree);
        }

        // Compare first 1KB
        let main_sample = Self::read_first_kb(&main_lockfile)?;
        let worktree_sample = Self::read_first_kb(&worktree_lockfile)?;

        if main_sample != worktree_sample {
            return Ok(false);
        }

        // If first 1KB matches, do full hash comparison
        Self::is_compatible(lockfile_name, main_repo, worktree)
    }

    /// Read first 1KB of a file
    fn read_first_kb(path: &Path) -> Result<Vec<u8>> {
        let content = fs::read(path)?;
        Ok(content.into_iter().take(1024).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_identical_lockfiles() {
        let temp = TempDir::new().unwrap();
        let main_dir = temp.path().join("main");
        let wt_dir = temp.path().join("worktree");
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&wt_dir).unwrap();

        let lockfile_content = "dependencies: foo@1.0.0";
        fs::write(main_dir.join("package-lock.json"), lockfile_content).unwrap();
        fs::write(wt_dir.join("package-lock.json"), lockfile_content).unwrap();

        let result = CompatibilityChecker::is_compatible(
            "package-lock.json",
            &main_dir,
            &wt_dir,
        )
        .unwrap();

        assert!(result);
    }

    #[test]
    fn test_different_lockfiles() {
        let temp = TempDir::new().unwrap();
        let main_dir = temp.path().join("main");
        let wt_dir = temp.path().join("worktree");
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&wt_dir).unwrap();

        fs::write(main_dir.join("package-lock.json"), "dependencies: foo@1.0.0").unwrap();
        fs::write(wt_dir.join("package-lock.json"), "dependencies: foo@2.0.0").unwrap();

        let result = CompatibilityChecker::is_compatible(
            "package-lock.json",
            &main_dir,
            &wt_dir,
        )
        .unwrap();

        assert!(!result);
    }

    #[test]
    fn test_missing_lockfiles() {
        let temp = TempDir::new().unwrap();
        let main_dir = temp.path().join("main");
        let wt_dir = temp.path().join("worktree");
        fs::create_dir_all(&main_dir).unwrap();
        fs::create_dir_all(&wt_dir).unwrap();

        let result = CompatibilityChecker::is_compatible(
            "package-lock.json",
            &main_dir,
            &wt_dir,
        )
        .unwrap();

        assert!(!result);
    }
}
