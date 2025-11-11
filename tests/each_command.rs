/// Integration tests for the `hn each` command
mod common;

use common::TestRepo;
use std::fs;

#[test]
fn test_each_basic_command() {
    let repo = TestRepo::new();

    // Create multiple worktrees
    repo.hn(&["add", "wt1"]).assert_success();
    repo.hn(&["add", "wt2"]).assert_success();
    repo.hn(&["add", "wt3"]).assert_success();

    // Run a simple command in each worktree
    let result = repo.hn(&["each", "pwd"]);
    result.assert_success();

    // Should show output from all worktrees
    assert!(result.stdout.contains("wt1") || result.stderr.contains("wt1"));
    assert!(result.stdout.contains("wt2") || result.stderr.contains("wt2"));
    assert!(result.stdout.contains("wt3") || result.stderr.contains("wt3"));
}

#[test]
fn test_each_with_filter() {
    let repo = TestRepo::new();

    // Create worktrees with different name patterns
    repo.hn(&["add", "feature-auth"]).assert_success();
    repo.hn(&["add", "feature-billing"]).assert_success();
    repo.hn(&["add", "hotfix-123"]).assert_success();

    // Run command only on feature worktrees
    let result = repo.hn(&["each", "--filter=^feature", "echo", "test"]);
    result.assert_success();

    // Output should mention feature worktrees
    let output = format!("{}{}", result.stdout, result.stderr);
    assert!(output.contains("feature-auth") || output.contains("feature-billing"));
}

#[test]
fn test_each_stop_on_error() {
    let repo = TestRepo::new();

    // Create multiple worktrees
    repo.hn(&["add", "wt1"]).assert_success();
    repo.hn(&["add", "wt2"]).assert_success();

    // Run a command that will fail
    let result = repo.hn(&["each", "--stop-on-error", "false"]);

    // Should fail when stop-on-error is set
    assert!(!result.success);
}

#[test]
fn test_each_creates_files() {
    let repo = TestRepo::new();

    // Create multiple worktrees
    repo.hn(&["add", "wt1"]).assert_success();
    repo.hn(&["add", "wt2"]).assert_success();

    // Create a test file in each worktree
    repo.hn(&["each", "touch", "test-file.txt"]).assert_success();

    // Verify files were created
    assert!(repo.worktree_path("wt1").join("test-file.txt").exists());
    assert!(repo.worktree_path("wt2").join("test-file.txt").exists());
}

#[test]
fn test_each_no_worktrees() {
    let repo = TestRepo::new();

    // Run command when no worktrees exist
    let result = repo.hn(&["each", "echo", "test"]);

    // Should succeed - message may vary but output should indicate no action taken
    result.assert_success();
    // Just verify it completes without error
}

#[test]
fn test_each_empty_command() {
    let repo = TestRepo::new();
    repo.hn(&["add", "wt1"]).assert_success();

    // Try to run empty command
    let result = repo.hn(&["each"]);

    // Should fail with error about missing command
    assert!(!result.success);
    assert!(
        result.stderr.contains("command") || result.stderr.contains("required")
    );
}
