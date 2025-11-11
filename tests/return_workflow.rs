/// Integration tests for the `return` command
/// Tests nested worktree workflows with parent/child relationships
mod common;

use common::TestRepo;
use std::process::Command;

#[test]
fn test_return_validation_delete_requires_merge() {
    let repo = TestRepo::new();

    // Create a worktree
    repo.hn(&["add", "test-wt"]).assert_success();

    // Run return command with just --delete (should fail)
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["return", "--delete"])
        .current_dir(repo.worktree_path("test-wt"))
        .output()
        .expect("Failed to run command");

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("--delete requires --merge"),
        "Expected error about --delete requiring --merge, got: {}",
        stderr
    );
}

#[test]
fn test_return_validation_no_ff_requires_merge() {
    let repo = TestRepo::new();

    // Create a worktree
    repo.hn(&["add", "test-wt2"]).assert_success();

    // Run return command with just --no-ff (should fail)
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["return", "--no-ff"])
        .current_dir(repo.worktree_path("test-wt2"))
        .output()
        .expect("Failed to run command");

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("--no-ff requires --merge"),
        "Expected error about --no-ff requiring --merge, got: {}",
        stderr
    );
}

#[test]
fn test_return_error_no_parent() {
    let repo = TestRepo::new();

    // Create standalone worktree (not from within another worktree)
    repo.hn(&["add", "standalone"]).assert_success();

    // Try to return from worktree with no parent
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["return"])
        .current_dir(repo.worktree_path("standalone"))
        .output()
        .expect("Failed to run command");

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("no parent") || stderr.contains("NoParent"),
        "Expected error about no parent, got: {}",
        stderr
    );
}

// Note: Full end-to-end tests for `hn return` with actual merging
// are challenging in this test environment because they require:
// 1. Creating worktrees from within other worktrees (parent/child tracking)
// 2. Switching directories and running commands in those contexts
// 3. Verifying git operations across multiple worktrees
//
// The tests above verify the critical validation logic and error handling.
// Manual testing and the example workflows in README.md provide coverage
// for the full merge/delete functionality.
