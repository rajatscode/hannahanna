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
///
/// Safety guarantees:
/// - Atomic operation: if any step fails, the state is rolled back
/// - Stable stash references: uses unique message-based stash identification
/// - Non-destructive: working directory is not modified until snapshot is confirmed saved
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

    // Generate snapshot name and timestamp FIRST (before any git operations)
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let name = if let Some(n) = snapshot_name {
        n.to_string()
    } else {
        format!("snapshot-{}", created_at)
    };

    // Check if snapshot name already exists
    let index_path = get_snapshot_index_path(state_dir);
    let existing_index = SnapshotIndex::load(&index_path)?;
    if existing_index.find(worktree_name, &name).is_some() {
        return Err(HnError::ConfigError(format!(
            "Snapshot '{}' already exists for worktree '{}'",
            name, worktree_name
        )));
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

    // Create snapshot object BEFORE any destructive operations
    let snapshot = Snapshot {
        name: name.clone(),
        worktree: worktree_name.to_string(),
        branch,
        commit,
        stash_ref: None, // Will be set if we create a stash
        has_uncommitted,
        created_at,
        description: description.map(|s| s.to_string()),
    };

    // Save snapshot metadata FIRST (before creating stash)
    // This ensures we have a record even if stash creation fails
    let mut index = SnapshotIndex::load(&index_path)?;
    index.add(snapshot.clone());
    index.save(&index_path)?;

    // Now create stash if needed (with rollback capability)
    let stash_ref = if has_uncommitted {
        // Create unique stash message for reliable lookup
        let stash_message = format!(
            "hannahanna-snapshot:{}:{}:{}",
            worktree_name, name, created_at
        );

        // Include untracked files in stash
        let stash_output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .arg("stash")
            .arg("push")
            .arg("--include-untracked")
            .arg("-m")
            .arg(&stash_message)
            .output()
            .map_err(|e| {
                // Rollback: remove snapshot from index
                let _ = delete_snapshot(worktree_name, &name, state_dir);
                HnError::CommandFailed(format!("Failed to stash changes: {}", e))
            })?;

        if !stash_output.status.success() {
            let stderr = String::from_utf8_lossy(&stash_output.stderr);
            // Rollback: remove snapshot from index
            let _ = delete_snapshot(worktree_name, &name, state_dir);
            return Err(HnError::CommandFailed(format!("Failed to stash changes: {}", stderr)));
        }

        // Use the stash message as the reference (stable across operations)
        // We'll look it up by message when restoring
        Some(stash_message)
    } else {
        None
    };

    // Update snapshot with stash reference
    if stash_ref.is_some() {
        let mut updated_snapshot = snapshot;
        updated_snapshot.stash_ref = stash_ref;

        // Update index with stash ref
        let mut index = SnapshotIndex::load(&index_path)?;
        index.remove(worktree_name, &name);
        index.add(updated_snapshot.clone());
        index.save(&index_path)?;

        Ok(updated_snapshot)
    } else {
        Ok(snapshot)
    }
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
///
/// Safety guarantees:
/// - Verifies clean working directory before making changes
/// - Uses message-based stash lookup for reliability
/// - Provides clear error messages if restoration fails
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
        })?
        .clone(); // Clone to avoid lifetime issues

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
    if let Some(ref stash_message) = snapshot.stash_ref {
        // Find stash by message (stable reference)
        let stash_list_output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .arg("stash")
            .arg("list")
            .output()
            .map_err(|e| HnError::CommandFailed(format!("Failed to list stashes: {}", e)))?;

        let stash_list = String::from_utf8(stash_list_output.stdout).unwrap_or_default();

        // Find the stash entry by looking for our unique message
        let mut stash_index: Option<usize> = None;
        for (idx, line) in stash_list.lines().enumerate() {
            if line.contains(stash_message) {
                stash_index = Some(idx);
                break;
            }
        }

        if let Some(idx) = stash_index {
            // Apply stash by index (stash@{N})
            let stash_ref = format!("stash@{{{}}}", idx);
            let stash_output = Command::new("git")
                .arg("-C")
                .arg(worktree_path)
                .arg("stash")
                .arg("apply")
                .arg(&stash_ref)
                .output()
                .map_err(|e| HnError::CommandFailed(format!("Failed to apply stash: {}", e)))?;

            if !stash_output.status.success() {
                let stderr = String::from_utf8_lossy(&stash_output.stderr);
                eprintln!("Warning: Failed to restore uncommitted changes: {}", stderr);
                eprintln!("The snapshot commit was restored successfully.");
                eprintln!("You can try manually: git stash apply {}", stash_ref);
            }
        } else {
            eprintln!("Warning: Stash for snapshot not found in git stash list");
            eprintln!("The snapshot commit was restored successfully.");
            eprintln!("Uncommitted changes from snapshot time may have been lost.");
            eprintln!("Stash message: {}", stash_message);
        }
    }

    Ok(())
}

/// Delete a snapshot
///
/// Safety guarantees:
/// - Cleans up associated git stash to prevent accumulation
/// - Gracefully handles missing stashes (warns but doesn't fail)
/// - Provides detailed feedback on cleanup status
pub fn delete_snapshot(worktree_name: &str, snapshot_name: &str, state_dir: &Path) -> Result<()> {
    let index_path = get_snapshot_index_path(state_dir);
    let mut index = SnapshotIndex::load(&index_path)?;

    // Find and clone the snapshot before removing it
    let snapshot = index
        .find(worktree_name, snapshot_name)
        .cloned()
        .ok_or_else(|| {
            HnError::ConfigError(format!(
                "Snapshot '{}' not found for worktree '{}'",
                snapshot_name, worktree_name
            ))
        })?;

    // Remove from index first
    index.remove(worktree_name, snapshot_name);
    index.save(&index_path)?;

    // Clean up associated git stash if present
    if let Some(ref stash_message) = snapshot.stash_ref {
        // Find the worktree path using git worktree list
        let repo_root = state_dir.parent().ok_or_else(|| {
            HnError::ConfigError("Invalid state directory path".to_string())
        })?;

        // Use git worktree list to find the actual worktree path
        let worktree_list_output = Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .arg("worktree")
            .arg("list")
            .arg("--porcelain")
            .output();

        if let Ok(output) = worktree_list_output {
            let worktree_list = String::from_utf8(output.stdout).unwrap_or_default();

            // Parse worktree list to find our worktree
            let mut found_worktree_path: Option<std::path::PathBuf> = None;
            for line in worktree_list.lines() {
                if line.starts_with("worktree ") {
                    let path = std::path::PathBuf::from(line.trim_start_matches("worktree "));
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name == worktree_name {
                            found_worktree_path = Some(path);
                            break;
                        }
                    }
                }
            }

            // If we found the worktree, try to clean up its stash
            if let Some(worktree_path) = found_worktree_path {
                // List stashes to find our stash
                let stash_list_output = Command::new("git")
                    .arg("-C")
                    .arg(&worktree_path)
                    .arg("stash")
                    .arg("list")
                    .output();

                if let Ok(output) = stash_list_output {
                    let stash_list = String::from_utf8(output.stdout).unwrap_or_default();

                    // Find the stash index by message
                    let mut stash_index: Option<usize> = None;
                    for (idx, line) in stash_list.lines().enumerate() {
                        if line.contains(stash_message) {
                            stash_index = Some(idx);
                            break;
                        }
                    }

                    // Drop the stash if found
                    if let Some(idx) = stash_index {
                        let stash_ref = format!("stash@{{{}}}", idx);
                        let drop_output = Command::new("git")
                            .arg("-C")
                            .arg(&worktree_path)
                            .arg("stash")
                            .arg("drop")
                            .arg(&stash_ref)
                            .output();

                        match drop_output {
                            Ok(output) if output.status.success() => {
                                // Successfully dropped stash
                            }
                            Ok(output) => {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                eprintln!("Warning: Failed to drop associated git stash: {}", stderr);
                                eprintln!("You may want to manually clean up: git stash drop {}", stash_ref);
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to execute git stash drop: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Clean up orphaned stashes for deleted snapshots
///
/// This maintenance function scans git stashes and removes any that belong
/// to snapshots that no longer exist in the index.
pub fn cleanup_orphaned_stashes(state_dir: &Path, worktree_path: &Path) -> Result<usize> {
    let index_path = get_snapshot_index_path(state_dir);
    let index = SnapshotIndex::load(&index_path)?;

    // Get all hannahanna snapshot stash messages that should exist
    let valid_stash_messages: std::collections::HashSet<String> = index
        .snapshots
        .iter()
        .filter_map(|s| s.stash_ref.clone())
        .collect();

    // List all stashes
    let stash_list_output = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .arg("stash")
        .arg("list")
        .output()
        .map_err(|e| HnError::CommandFailed(format!("Failed to list stashes: {}", e)))?;

    let stash_list = String::from_utf8(stash_list_output.stdout).unwrap_or_default();

    // Find orphaned hannahanna stashes
    let mut orphaned_indices = Vec::new();
    for (idx, line) in stash_list.lines().enumerate() {
        // Check if this is a hannahanna stash
        if line.contains("hannahanna-snapshot:") {
            // Extract the message
            let message_start = line.find(": ").map(|i| i + 2);
            if let Some(start) = message_start {
                let message = &line[start..];
                // Check if this stash message is in our valid set
                if !valid_stash_messages.iter().any(|m| message.contains(m)) {
                    orphaned_indices.push(idx);
                }
            }
        }
    }

    // Drop orphaned stashes (in reverse order to maintain indices)
    let mut cleaned = 0;
    for idx in orphaned_indices.iter().rev() {
        let stash_ref = format!("stash@{{{}}}", idx);
        let drop_output = Command::new("git")
            .arg("-C")
            .arg(worktree_path)
            .arg("stash")
            .arg("drop")
            .arg(&stash_ref)
            .output();

        if let Ok(output) = drop_output {
            if output.status.success() {
                cleaned += 1;
            }
        }
    }

    Ok(cleaned)
}
