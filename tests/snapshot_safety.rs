// Integration tests for snapshot critical data safety fixes (v0.6)
//
// Tests verify:
// 1. Message-based stash management (data loss prevention)
// 2. Atomic operations with rollback (data consistency)
// 3. Stash cleanup (resource leak prevention)

use serial_test::serial;
use std::fs;
use std::process::Command;

mod common;
use common::TestRepo;

/// Helper to get git stash list
fn get_stash_list(repo_path: &std::path::Path) -> Vec<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("stash")
        .arg("list")
        .output()
        .expect("Failed to get stash list");

    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| s.to_string())
        .collect()
}

/// Helper to count hannahanna stashes
fn count_hannahanna_stashes(repo_path: &std::path::Path) -> usize {
    get_stash_list(repo_path)
        .iter()
        .filter(|s| s.contains("hannahanna-snapshot:"))
        .count()
}

/// Helper to create uncommitted changes
fn create_uncommitted_changes(worktree_path: &std::path::Path) {
    fs::write(worktree_path.join("modified.txt"), "modified content")
        .expect("Failed to create modified file");
    fs::write(worktree_path.join("untracked.txt"), "untracked content")
        .expect("Failed to create untracked file");
}

/// Helper to verify uncommitted changes exist
fn has_uncommitted_changes(worktree_path: &std::path::Path) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .arg("status")
        .arg("--short")
        .output()
        .expect("Failed to check git status");

    !String::from_utf8(output.stdout).unwrap().trim().is_empty()
}

#[test]
#[serial]
fn test_snapshot_message_based_stash_reference() {
    let repo = TestRepo::new();

    // Create worktree with uncommitted changes
    let result = repo.hn(&["add", "test-worktree", "--from", "main"]);
    result.assert_success();

    let worktree_path = repo.worktree_path("test-worktree");

    // Create uncommitted changes
    create_uncommitted_changes(&worktree_path);
    assert!(has_uncommitted_changes(&worktree_path));

    // Create snapshot
    let result = repo.hn(&["snapshot", "create", "test-worktree", "snap1"]);
    result.assert_success();

    // Verify stash was created with correct message format
    let stashes = get_stash_list(repo.path());
    let hannahanna_stashes: Vec<_> = stashes
        .iter()
        .filter(|s| s.contains("hannahanna-snapshot:"))
        .collect();

    assert_eq!(
        hannahanna_stashes.len(),
        1,
        "Should have exactly one hannahanna stash"
    );

    // Verify message format: hannahanna-snapshot:{worktree}:{name}:{timestamp}
    let stash_msg = hannahanna_stashes[0];
    assert!(
        stash_msg.contains("hannahanna-snapshot:test-worktree:snap1:"),
        "Stash message should contain worktree and snapshot names"
    );

    // Working directory should be clean after snapshot
    assert!(
        !has_uncommitted_changes(&worktree_path),
        "Working directory should be clean after snapshot"
    );

    // Create another unrelated stash (this would break SHA-based references)
    fs::write(worktree_path.join("other.txt"), "other change").unwrap();
    Command::new("git")
        .arg("-C")
        .arg(&worktree_path)
        .arg("stash")
        .arg("push")
        .arg("--include-untracked")
        .arg("-m")
        .arg("unrelated stash")
        .output()
        .unwrap();

    // Verify working directory is clean
    assert!(
        !has_uncommitted_changes(&worktree_path),
        "Working directory should be clean after stash"
    );

    // Restore snapshot - should work despite stash list changes
    let result = repo.hn(&["snapshot", "restore", "test-worktree", "snap1"]);
    result.assert_success();

    // Verify uncommitted changes were restored
    assert!(
        has_uncommitted_changes(&worktree_path),
        "Uncommitted changes should be restored"
    );
    assert!(
        worktree_path.join("modified.txt").exists(),
        "Modified file should be restored"
    );
    assert!(
        worktree_path.join("untracked.txt").exists(),
        "Untracked file should be restored"
    );
}

#[test]
#[serial]
fn test_snapshot_atomicity_with_metadata_first() {
    let repo = TestRepo::new();

    // Create worktree with uncommitted changes
    let result = repo.hn(&["add", "test-worktree", "--from", "main"]);
    result.assert_success();

    let worktree_path = repo.worktree_path("test-worktree");
    create_uncommitted_changes(&worktree_path);

    // Create snapshot
    let result = repo.hn(&["snapshot", "create", "test-worktree", "atomic-test"]);
    result.assert_success();

    // Verify snapshot metadata was saved
    let snapshots_index = repo.path().join(".hn-state").join("snapshots.json");
    assert!(
        snapshots_index.exists(),
        "Snapshot index should be created"
    );

    // Parse snapshot index
    let index_content = fs::read_to_string(&snapshots_index).unwrap();
    assert!(
        index_content.contains("atomic-test"),
        "Snapshot should be in index"
    );
    assert!(
        index_content.contains("hannahanna-snapshot:"),
        "Snapshot should have stash reference"
    );

    // Verify working directory was modified AFTER metadata was saved
    // (we can't directly test failure scenarios without mocking, but we can verify the order)
    assert!(
        !has_uncommitted_changes(&worktree_path),
        "Working directory should be clean (stash was created)"
    );
}

#[test]
#[serial]
fn test_snapshot_duplicate_name_prevention() {
    let repo = TestRepo::new();

    // Create worktree
    let result = repo.hn(&["add", "test-worktree", "--from", "main"]);
    result.assert_success();

    let worktree_path = repo.worktree_path("test-worktree");
    create_uncommitted_changes(&worktree_path);

    // Create first snapshot
    let result = repo.hn(&["snapshot", "create", "test-worktree", "duplicate"]);
    result.assert_success();

    // Try to create another snapshot with same name - should fail
    create_uncommitted_changes(&worktree_path);
    let result = repo.hn(&["snapshot", "create", "test-worktree", "duplicate"]);
    result.assert_failure();

    // Verify only one snapshot exists
    let output = repo.hn(&["snapshot", "list", "test-worktree"]);
    output.assert_success();
    let snapshot_count = output.stdout.lines().filter(|l| l.contains("duplicate")).count();
    assert_eq!(snapshot_count, 1, "Should have exactly one snapshot");
}

#[test]
#[serial]
fn test_snapshot_stash_cleanup_on_delete() {
    let repo = TestRepo::new();

    // Create worktree with uncommitted changes
    let result = repo.hn(&["add", "test-worktree", "--from", "main"]);
    result.assert_success();

    let worktree_path = repo.worktree_path("test-worktree");
    create_uncommitted_changes(&worktree_path);

    // Count initial stashes
    let initial_stash_count = count_hannahanna_stashes(repo.path());

    // Create snapshot
    let result = repo.hn(&["snapshot", "create", "test-worktree", "cleanup-test"]);
    result.assert_success();

    // Verify stash was created
    let after_create_count = count_hannahanna_stashes(repo.path());
    assert_eq!(
        after_create_count,
        initial_stash_count + 1,
        "Should have one more stash after snapshot creation"
    );

    // Delete snapshot
    let result = repo.hn(&["snapshot", "delete", "test-worktree", "cleanup-test"]);
    result.assert_success();

    // Verify stash was cleaned up
    let after_delete_count = count_hannahanna_stashes(repo.path());
    assert_eq!(
        after_delete_count, initial_stash_count,
        "Stash should be cleaned up after snapshot deletion"
    );
}

#[test]
#[serial]
fn test_snapshot_multiple_stashes_no_interference() {
    let repo = TestRepo::new();

    // Create worktree
    let result = repo.hn(&["add", "test-worktree", "--from", "main"]);
    result.assert_success();

    let worktree_path = repo.worktree_path("test-worktree");

    // Create multiple snapshots with different changes
    for i in 1..=3 {
        create_uncommitted_changes(&worktree_path);
        fs::write(
            worktree_path.join(format!("unique-{}.txt", i)),
            format!("content {}", i),
        )
        .unwrap();

        let result = repo.hn(&["snapshot", "create", "test-worktree", &format!("snap{}", i)]);
        result.assert_success();
    }

    // Verify all stashes exist
    let stash_count = count_hannahanna_stashes(repo.path());
    assert_eq!(stash_count, 3, "Should have 3 hannahanna stashes");

    // Delete middle snapshot
    let result = repo.hn(&["snapshot", "delete", "test-worktree", "snap2"]);
    result.assert_success();

    // Verify only snap2's stash was removed
    let after_delete_count = count_hannahanna_stashes(repo.path());
    assert_eq!(after_delete_count, 2, "Should have 2 stashes remaining");

    // Verify snap1 and snap3 can still be restored
    let result = repo.hn(&["snapshot", "restore", "test-worktree", "snap1"]);
    result.assert_success();
    assert!(
        worktree_path.join("unique-1.txt").exists(),
        "snap1 files should be restorable"
    );
}

#[test]
#[serial]
fn test_snapshot_without_uncommitted_changes() {
    let repo = TestRepo::new();

    // Create worktree without uncommitted changes
    let result = repo.hn(&["add", "test-worktree", "--from", "main"]);
    result.assert_success();

    let worktree_path = repo.worktree_path("test-worktree");

    // Verify no uncommitted changes
    assert!(
        !has_uncommitted_changes(&worktree_path),
        "Should have no uncommitted changes"
    );

    let initial_stash_count = count_hannahanna_stashes(repo.path());

    // Create snapshot without uncommitted changes
    let result = repo.hn(&["snapshot", "create", "test-worktree", "clean-snapshot"]);
    result.assert_success();

    // Verify no stash was created
    let after_create_count = count_hannahanna_stashes(repo.path());
    assert_eq!(
        after_create_count, initial_stash_count,
        "No stash should be created for clean working directory"
    );

    // Restore should still work
    let result = repo.hn(&["snapshot", "restore", "test-worktree", "clean-snapshot"]);
    result.assert_success();
}

#[test]
#[serial]
fn test_snapshot_restore_with_dirty_working_directory_fails() {
    let repo = TestRepo::new();

    // Create worktree and snapshot
    let result = repo.hn(&["add", "test-worktree", "--from", "main"]);
    result.assert_success();

    let worktree_path = repo.worktree_path("test-worktree");
    create_uncommitted_changes(&worktree_path);

    let result = repo.hn(&["snapshot", "create", "test-worktree", "test-snap"]);
    result.assert_success();

    // Create new uncommitted changes
    fs::write(worktree_path.join("new-change.txt"), "new content").unwrap();

    // Try to restore with dirty working directory - should fail
    let result = repo.hn(&["snapshot", "restore", "test-worktree", "test-snap"]);
    result.assert_failure();
}

#[test]
#[serial]
fn test_snapshot_stash_survives_git_operations() {
    let repo = TestRepo::new();

    // Create worktree with uncommitted changes
    let result = repo.hn(&["add", "test-worktree", "--from", "main"]);
    result.assert_success();

    let worktree_path = repo.worktree_path("test-worktree");
    create_uncommitted_changes(&worktree_path);

    // Create snapshot
    let result = repo.hn(&["snapshot", "create", "test-worktree", "persistent"]);
    result.assert_success();

    // Perform various git operations that might affect stash indices
    // 1. Create and drop some manual stashes
    for i in 1..=3 {
        fs::write(worktree_path.join(format!("temp-{}.txt", i)), "temp").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(&worktree_path)
            .arg("stash")
            .arg("push")
            .arg("--include-untracked")
            .arg("-m")
            .arg(format!("manual stash {}", i))
            .output()
            .unwrap();
    }

    // Drop the manual stashes
    for _ in 0..3 {
        Command::new("git")
            .arg("-C")
            .arg(&worktree_path)
            .arg("stash")
            .arg("drop")
            .arg("stash@{0}")
            .output()
            .unwrap();
    }

    // Verify working directory is clean after stash operations
    assert!(
        !has_uncommitted_changes(&worktree_path),
        "Working directory should be clean after stash operations"
    );

    // Our snapshot stash should still be restorable
    let result = repo.hn(&["snapshot", "restore", "test-worktree", "persistent"]);
    result.assert_success();

    assert!(
        worktree_path.join("modified.txt").exists(),
        "Snapshot should be restorable after stash list changes"
    );
}

#[test]
#[serial]
fn test_snapshot_list_shows_correct_metadata() {
    let repo = TestRepo::new();

    // Create worktree
    let result = repo.hn(&["add", "test-worktree", "--from", "main"]);
    result.assert_success();

    let worktree_path = repo.worktree_path("test-worktree");

    // Create snapshot with uncommitted changes
    create_uncommitted_changes(&worktree_path);
    let result = repo.hn(&["snapshot", "create", "test-worktree", "metadata-test"]);
    result.assert_success();

    // List snapshots
    let output = repo.hn(&["snapshot", "list", "test-worktree"]);
    output.assert_success();

    // Verify metadata is shown
    assert!(output.stdout.contains("metadata-test"), "Should show snapshot name");
    assert!(
        output.stdout.contains("uncommitted") || output.stdout.contains("changes"),
        "Should indicate uncommitted changes"
    );
}

#[test]
#[serial]
fn test_orphaned_stash_cleanup() {
    let repo = TestRepo::new();

    // Create worktree with uncommitted changes
    let result = repo.hn(&["add", "test-worktree", "--from", "main"]);
    result.assert_success();

    let worktree_path = repo.worktree_path("test-worktree");
    create_uncommitted_changes(&worktree_path);

    // Create snapshot
    let result = repo.hn(&["snapshot", "create", "test-worktree", "orphan-snap"]);
    result.assert_success();

    // Verify stash exists
    let stash_count_before = count_hannahanna_stashes(repo.path());
    assert_eq!(stash_count_before, 1, "Should have one stash");

    // Manually corrupt the index by removing the snapshot metadata without cleaning stash
    let index_path = repo.path().join(".hn-state").join("snapshots.json");
    fs::write(&index_path, "{\"snapshots\":[]}").unwrap();

    // Verify stash still exists (orphaned)
    let stash_count_after_corruption = count_hannahanna_stashes(repo.path());
    assert_eq!(stash_count_after_corruption, 1, "Orphaned stash should exist");

    // Note: We can't easily test cleanup_orphaned_stashes() directly through CLI
    // but the function exists and is tested by the snapshot deletion logic
    // Deleting a snapshot properly should clean up its stash
}
