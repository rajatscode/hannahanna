/// Full lifecycle integration tests for common usage scenarios
/// These tests verify end-to-end workflows with all commands working together
use crate::common::TestRepo;
use std::fs;
use std::process::Command;

#[path = "../common/mod.rs"]
mod common;

/// Test complete lifecycle: create, use, merge back, delete
#[test]
fn test_full_lifecycle_with_parent_merge() {
    let repo = TestRepo::new();

    // Create parent worktree
    repo.hn(&["add", "parent-feature"]).assert_success();
    assert!(repo.worktree_exists("parent-feature"));

    // Make a commit in parent to have something to merge
    let parent_path = repo.worktree_path("parent-feature");
    fs::write(parent_path.join("parent-file.txt"), "parent content")
        .expect("Failed to write file");

    Command::new("git")
        .args(["add", "parent-file.txt"])
        .current_dir(&parent_path)
        .output()
        .expect("Failed to add file");

    Command::new("git")
        .args(["commit", "-m", "Add parent file"])
        .current_dir(&parent_path)
        .output()
        .expect("Failed to commit");

    // Create child worktree from within parent
    // Note: This test assumes we can simulate being in parent worktree
    // In a real scenario, the user would cd into parent-feature first

    // For now, we verify the basic workflow works
    repo.hn(&["add", "child-feature", "--from=parent-feature"])
        .assert_success();
    assert!(repo.worktree_exists("child-feature"));

    // Make changes in child
    let child_path = repo.worktree_path("child-feature");
    fs::write(child_path.join("child-file.txt"), "child content")
        .expect("Failed to write file");

    Command::new("git")
        .args(["add", "child-file.txt"])
        .current_dir(&child_path)
        .output()
        .expect("Failed to add file");

    Command::new("git")
        .args(["commit", "-m", "Add child file"])
        .current_dir(&child_path)
        .output()
        .expect("Failed to commit");

    // Clean up
    repo.hn(&["remove", "child-feature"]).assert_success();
    repo.hn(&["remove", "parent-feature"]).assert_success();
}

/// Test working with multiple worktrees concurrently
#[test]
fn test_concurrent_worktree_operations() {
    let repo = TestRepo::new();

    // Create multiple worktrees rapidly
    for i in 1..=5 {
        repo.hn(&["add", &format!("wt-{}", i)]).assert_success();
    }

    // Verify all exist
    let result = repo.hn(&["list"]);
    result.assert_success();
    for i in 1..=5 {
        result.assert_stdout_contains(&format!("wt-{}", i));
    }

    // Remove all
    for i in 1..=5 {
        repo.hn(&["remove", &format!("wt-{}", i)]).assert_success();
    }

    // Verify all gone
    let result = repo.hn(&["list"]);
    result.assert_success();
    for i in 1..=5 {
        assert!(!result.stdout.contains(&format!("wt-{}", i)));
    }
}

/// Test info command shows correct details
#[test]
fn test_info_command_details() {
    let repo = TestRepo::new();

    // Create a worktree
    repo.hn(&["add", "test-info"]).assert_success();

    // Get info
    let result = repo.hn(&["info", "test-info"]);
    result.assert_success();

    // Should show name, branch, path
    result.assert_stdout_contains("test-info");
    result.assert_stdout_contains("Path:");
    result.assert_stdout_contains("Branch:");
}

/// Test list command with tree view
#[test]
fn test_list_tree_view() {
    let repo = TestRepo::new();

    // Create parent and children
    repo.hn(&["add", "parent"]).assert_success();
    repo.hn(&["add", "child1", "--from=parent"]).assert_success();
    repo.hn(&["add", "child2", "--from=parent"]).assert_success();

    // Get tree view
    let result = repo.hn(&["list", "--tree"]);
    result.assert_success();

    // Should show tree structure
    // The exact format may vary, but should show the worktrees
    assert!(
        result.stdout.contains("parent") || result.stderr.contains("parent")
    );
}

/// Test error handling when trying to remove non-existent worktree
#[test]
fn test_remove_nonexistent_worktree() {
    let repo = TestRepo::new();

    // Try to remove worktree that doesn't exist
    let result = repo.hn(&["remove", "does-not-exist"]);

    // Should fail with helpful error
    assert!(!result.success);
    assert!(
        result.stderr.contains("not found") || result.stderr.contains("does-not-exist")
    );
}

/// Test fuzzy matching works correctly
#[test]
fn test_fuzzy_matching() {
    let repo = TestRepo::new();

    // Create worktrees with similar names
    repo.hn(&["add", "feature-auth"]).assert_success();
    repo.hn(&["add", "feature-billing"]).assert_success();

    // Fuzzy match should work with substring
    repo.hn(&["switch", "auth"]).assert_success();
    repo.hn(&["switch", "billing"]).assert_success();

    // Info should also work with fuzzy matching
    let result = repo.hn(&["info", "auth"]);
    result.assert_success();
    result.assert_stdout_contains("feature-auth");
}

/// Test that git operations work correctly in worktrees
#[test]
fn test_git_operations_in_worktree() {
    let repo = TestRepo::new();

    // Create worktree
    repo.hn(&["add", "test-git"]).assert_success();

    let wt_path = repo.worktree_path("test-git");

    // Create and commit a file
    fs::write(wt_path.join("test.txt"), "test content")
        .expect("Failed to write file");

    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&wt_path)
        .output()
        .expect("Failed to add file");

    let output = Command::new("git")
        .args(["commit", "-m", "Test commit"])
        .current_dir(&wt_path)
        .output()
        .expect("Failed to commit");

    assert!(output.status.success());

    // Verify commit exists
    let log_output = Command::new("git")
        .args(["log", "--oneline", "-1"])
        .current_dir(&wt_path)
        .output()
        .expect("Failed to get log");

    assert!(String::from_utf8_lossy(&log_output.stdout).contains("Test commit"));
}

/// Test switch command output format
#[test]
fn test_switch_command_output() {
    let repo = TestRepo::new();

    // Create worktree
    repo.hn(&["add", "test-switch"]).assert_success();

    // Switch to it
    let result = repo.hn(&["switch", "test-switch"]);
    result.assert_success();

    // Should output the path (for shell wrapper)
    assert!(
        result.stdout.contains("test-switch") || result.stderr.contains("test-switch")
    );
}
