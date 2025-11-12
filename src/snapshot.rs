// Snapshot and restore functionality for worktrees
// Allows saving uncommitted changes and repository state

use crate::errors::{HnError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Deserialize, Clone)]
pub struct Snapshot {
    pub name: String,
    pub worktree: String,
    pub branch: String,
    pub commit: String,
    pub stash_ref: Option<String>,
    pub has_uncommitted: bool,
    pub created_at: u64,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SnapshotIndex {
    pub snapshots: Vec<Snapshot>,
}

impl SnapshotIndex {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)?;
        let index: SnapshotIndex = serde_json::from_str(&content)?;
        Ok(index)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(&self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn add(&mut self, snapshot: Snapshot) {
        self.snapshots.push(snapshot);
    }

    pub fn list_for_worktree(&self, worktree: &str) -> Vec<&Snapshot> {
        self.snapshots
            .iter()
            .filter(|s| s.worktree == worktree)
            .collect()
    }

    pub fn find(&self, worktree: &str, name: &str) -> Option<&Snapshot> {
        self.snapshots
            .iter()
            .find(|s| s.worktree == worktree && s.name == name)
    }

    pub fn remove(&mut self, worktree: &str, name: &str) -> bool {
        let original_len = self.snapshots.len();
        self.snapshots.retain(|s| !(s.worktree == worktree && s.name == name));
        self.snapshots.len() < original_len
    }
}

/// Get snapshot index path for a worktree
fn get_snapshot_index_path(state_dir: &Path) -> PathBuf {
    state_dir.join("snapshots.json")
}

/// Create a snapshot of a worktree
pub fn create_snapshot(
    worktree_path: &Path,
    worktree_name: &str,
    snapshot_name: Option<&str>,
    description: Option<&str>,
    state_dir: &Path,
) -> Result<Snapshot> {
    // Verify worktree exists
    if !worktree_path.exists() {
        return Err(HnError::WorktreeNotFound(worktree_name.to_string()));
    }

    // Get current branch
    let branch_output = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .map_err(|e| HnError::CommandFailed(format!("Failed to get branch: {}", e)))?;

    if !branch_output.status.success() {
        return Err(HnError::CommandFailed("Failed to get current branch".to_string()));
    }

    let branch = String::from_utf8(branch_output.stdout)
        .map_err(|e| HnError::CommandFailed(format!("Invalid UTF-8 in branch name: {}", e)))?
        .trim()
        .to_string();

    // Get current commit
    let commit_output = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .map_err(|e| HnError::CommandFailed(format!("Failed to get commit: {}", e)))?;

    if !commit_output.status.success() {
        return Err(HnError::CommandFailed("Failed to get current commit".to_string()));
    }

    let commit = String::from_utf8(commit_output.stdout)
        .map_err(|e| HnError::CommandFailed(format!("Invalid UTF-8 in commit hash: {}", e)))?
        .trim()
        .to_string();

    // Check for uncommitted changes
    let status_output = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .arg("status")
        .arg("--short")
        .output()
        .map_err(|e| HnError::CommandFailed(format!("Failed to get status: {}", e)))?;

    let status = String::from_utf8(status_output.stdout).unwrap_or_default();
    let has_uncommitted = !status.trim().is_empty();

    // Create stash if there are uncommitted changes
    let stash_ref = if has_uncommitted {
        // Include untracked files in stash
        let stash_output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .arg("stash")
            .arg("push")
            .arg("--include-untracked")
            .arg("-m")
            .arg(format!(
                "hannahanna snapshot: {}",
                snapshot_name.unwrap_or("unnamed")
            ))
            .output()
            .map_err(|e| HnError::CommandFailed(format!("Failed to stash changes: {}", e)))?;

        if !stash_output.status.success() {
            let stderr = String::from_utf8_lossy(&stash_output.stderr);
            return Err(HnError::CommandFailed(format!("Failed to stash changes: {}", stderr)));
        }

        // Get the stash ref
        let stash_list_output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .arg("stash")
            .arg("list")
            .arg("--max-count=1")
            .arg("--format=%H")
            .output()
            .map_err(|e| HnError::CommandFailed(format!("Failed to get stash ref: {}", e)))?;

        Some(String::from_utf8(stash_list_output.stdout)
            .map_err(|e| HnError::CommandFailed(format!("Invalid UTF-8 in stash ref: {}", e)))?
            .trim()
            .to_string())
    } else {
        None
    };

    // Generate snapshot name if not provided
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let name = if let Some(n) = snapshot_name {
        n.to_string()
    } else {
        format!("snapshot-{}", created_at)
    };

    let snapshot = Snapshot {
        name,
        worktree: worktree_name.to_string(),
        branch,
        commit,
        stash_ref,
        has_uncommitted,
        created_at,
        description: description.map(|s| s.to_string()),
    };

    // Save to index
    let index_path = get_snapshot_index_path(state_dir);
    let mut index = SnapshotIndex::load(&index_path)?;
    index.add(snapshot.clone());
    index.save(&index_path)?;

    Ok(snapshot)
}

/// List snapshots for a worktree or all worktrees
pub fn list_snapshots(state_dir: &Path, worktree: Option<&str>) -> Result<Vec<Snapshot>> {
    let index_path = get_snapshot_index_path(state_dir);
    let index = SnapshotIndex::load(&index_path)?;

    let snapshots = if let Some(wt) = worktree {
        index.list_for_worktree(wt).into_iter().cloned().collect()
    } else {
        index.snapshots.clone()
    };

    Ok(snapshots)
}

/// Restore a snapshot
pub fn restore_snapshot(
    worktree_path: &Path,
    worktree_name: &str,
    snapshot_name: &str,
    state_dir: &Path,
) -> Result<()> {
    // Load snapshot from index
    let index_path = get_snapshot_index_path(state_dir);
    let index = SnapshotIndex::load(&index_path)?;

    let snapshot = index
        .find(worktree_name, snapshot_name)
        .ok_or_else(|| {
            HnError::ConfigError(format!(
                "Snapshot '{}' not found for worktree '{}'",
                snapshot_name, worktree_name
            ))
        })?;

    // Verify worktree exists
    if !worktree_path.exists() {
        return Err(HnError::WorktreeNotFound(worktree_name.to_string()));
    }

    // Check for uncommitted changes
    let status_output = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .arg("status")
        .arg("--short")
        .output()
        .map_err(|e| HnError::CommandFailed(format!("Failed to get status: {}", e)))?;

    let status = String::from_utf8(status_output.stdout).unwrap_or_default();
    if !status.trim().is_empty() {
        return Err(HnError::ConfigError(
            "Worktree has uncommitted changes. Commit or stash them first.".to_string(),
        ));
    }

    // Checkout the branch
    let checkout_output = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .arg("checkout")
        .arg(&snapshot.branch)
        .output()
        .map_err(|e| HnError::CommandFailed(format!("Failed to checkout branch: {}", e)))?;

    if !checkout_output.status.success() {
        let stderr = String::from_utf8_lossy(&checkout_output.stderr);
        return Err(HnError::CommandFailed(format!("Failed to checkout branch: {}", stderr)));
    }

    // Reset to the commit
    let reset_output = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .arg("reset")
        .arg("--hard")
        .arg(&snapshot.commit)
        .output()
        .map_err(|e| HnError::CommandFailed(format!("Failed to reset to commit: {}", e)))?;

    if !reset_output.status.success() {
        let stderr = String::from_utf8_lossy(&reset_output.stderr);
        return Err(HnError::CommandFailed(format!("Failed to reset to commit: {}", stderr)));
    }

    // Restore stash if present
    if let Some(ref stash_ref) = snapshot.stash_ref {
        let stash_output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .arg("stash")
            .arg("apply")
            .arg(stash_ref)
            .output()
            .map_err(|e| HnError::CommandFailed(format!("Failed to apply stash: {}", e)))?;

        if !stash_output.status.success() {
            let stderr = String::from_utf8_lossy(&stash_output.stderr);
            eprintln!("Warning: Failed to restore uncommitted changes: {}", stderr);
            eprintln!("The snapshot commit was restored successfully.");
        }
    }

    Ok(())
}

/// Delete a snapshot
pub fn delete_snapshot(worktree_name: &str, snapshot_name: &str, state_dir: &Path) -> Result<()> {
    let index_path = get_snapshot_index_path(state_dir);
    let mut index = SnapshotIndex::load(&index_path)?;

    if !index.remove(worktree_name, snapshot_name) {
        return Err(HnError::ConfigError(format!(
            "Snapshot '{}' not found for worktree '{}'",
            snapshot_name, worktree_name
        )));
    }

    index.save(&index_path)?;
    Ok(())
}
