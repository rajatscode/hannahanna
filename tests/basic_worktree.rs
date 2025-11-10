/// Basic integration tests for currently implemented features
mod common;

use common::TestRepo;

#[test]
fn test_add_simple_worktree() {
    let repo = TestRepo::new();

    // Add a worktree with simple name
    let result = repo.hn(&["add", "my-feature"]);
    result.assert_success();

    // Verify worktree was created
    assert!(repo.worktree_exists("my-feature"));
}

#[test]
fn test_add_and_list() {
    let repo = TestRepo::new();

    // Add a few worktrees
    repo.hn(&["add", "feature-one"]).assert_success();
    repo.hn(&["add", "feature-two"]).assert_success();

    // List should show them
    let result = repo.hn(&["list"]);
    result.assert_success();
    result.assert_stdout_contains("feature-one");
    result.assert_stdout_contains("feature-two");
}

#[test]
fn test_remove_worktree() {
    let repo = TestRepo::new();

    // Add and remove
    repo.hn(&["add", "temp-feature"]).assert_success();
    assert!(repo.worktree_exists("temp-feature"));

    let result = repo.hn(&["remove", "temp-feature"]);
    result.assert_success();

    assert!(!repo.worktree_exists("temp-feature"));
}

#[test]
fn test_info_command() {
    let repo = TestRepo::new();

    repo.hn(&["add", "info-test"]).assert_success();

    let result = repo.hn(&["info", "info-test"]);
    result.assert_success();
    result.assert_stdout_contains("info-test");
}

#[test]
fn test_switch_command() {
    let repo = TestRepo::new();

    repo.hn(&["add", "switch-test"]).assert_success();

    let result = repo.hn(&["switch", "switch-test"]);
    result.assert_success();
}

#[test]
fn test_prune_command() {
    let repo = TestRepo::new();

    // Just test that prune runs without error
    let result = repo.hn(&["prune"]);
    result.assert_success();
}

#[test]
fn test_cannot_add_duplicate() {
    let repo = TestRepo::new();

    repo.hn(&["add", "duplicate"]).assert_success();

    // Try to add again
    let result = repo.hn(&["add", "duplicate"]);
    result.assert_failure();
}

#[test]
fn test_remove_nonexistent_fails() {
    let repo = TestRepo::new();

    let result = repo.hn(&["remove", "nonexistent"]);
    result.assert_failure();
}
