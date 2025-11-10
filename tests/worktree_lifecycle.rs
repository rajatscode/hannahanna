/// Integration tests for worktree lifecycle: add, list, remove, switch, info
mod common;

use common::TestRepo;

#[test]
fn test_add_worktree() {
    let repo = TestRepo::new();

    // Add a worktree
    let result = repo.hn(&["add", "feature-x"]);
    result.assert_success();
    result.assert_stderr_contains("feature-x");

    // Verify worktree was created
    assert!(repo.worktree_exists("feature-x"));
    assert!(repo.state_exists("feature-x"));
}

#[test]
fn test_add_worktree_with_from_branch() {
    let repo = TestRepo::new();

    // Create a branch
    repo.create_and_commit("file.txt", "content", "Add file");

    // Add worktree from main
    let result = repo.hn(&["add", "feature-y", "--from=main"]);
    result.assert_success();

    assert!(repo.worktree_exists("feature-y"));
}

#[test]
fn test_add_duplicate_worktree_fails() {
    let repo = TestRepo::new();

    // Add first worktree
    repo.hn(&["add", "feature-x"]).assert_success();

    // Try to add duplicate
    let result = repo.hn(&["add", "feature-x"]);
    result.assert_failure();
}

#[test]
fn test_list_worktrees() {
    let repo = TestRepo::new();

    // Initially no worktrees
    let result = repo.hn(&["list"]);
    result.assert_success();

    // Add some worktrees
    repo.hn(&["add", "feature-a"]).assert_success();
    repo.hn(&["add", "feature-b"]).assert_success();
    repo.hn(&["add", "feature-c"]).assert_success();

    // List should show all worktrees
    let result = repo.hn(&["list"]);
    result.assert_success();
    result.assert_stdout_contains("feature-a");
    result.assert_stdout_contains("feature-b");
    result.assert_stdout_contains("feature-c");
}

#[test]
fn test_list_with_tree_view() {
    let repo = TestRepo::new();

    repo.hn(&["add", "feature-x"]).assert_success();

    let result = repo.hn(&["list", "--tree"]);
    result.assert_success();
    result.assert_stdout_contains("feature-x");
}

#[test]
fn test_remove_worktree() {
    let repo = TestRepo::new();

    // Add and then remove
    repo.hn(&["add", "feature-x"]).assert_success();
    assert!(repo.worktree_exists("feature-x"));

    let result = repo.hn(&["remove", "feature-x"]);
    result.assert_success();

    // Verify worktree was removed
    assert!(!repo.worktree_exists("feature-x"));
    assert!(!repo.state_exists("feature-x"));
}

#[test]
fn test_remove_with_uncommitted_changes_fails() {
    let repo = TestRepo::new();

    // Add worktree
    repo.hn(&["add", "feature-x"]).assert_success();

    // Create uncommitted changes
    let worktree_path = repo.worktree_path("feature-x");
    std::fs::write(worktree_path.join("newfile.txt"), "content").expect("Failed to create file");

    // Try to remove without force
    let result = repo.hn(&["remove", "feature-x"]);
    result.assert_failure();
    result.assert_stderr_contains("uncommitted");
}

#[test]
fn test_remove_with_force() {
    let repo = TestRepo::new();

    // Add worktree
    repo.hn(&["add", "feature-x"]).assert_success();

    // Create uncommitted changes
    let worktree_path = repo.worktree_path("feature-x");
    std::fs::write(worktree_path.join("newfile.txt"), "content").expect("Failed to create file");

    // Remove with force
    let result = repo.hn(&["remove", "feature-x", "--force"]);
    result.assert_success();

    assert!(!repo.worktree_exists("feature-x"));
}

#[test]
fn test_remove_nonexistent_worktree_fails() {
    let repo = TestRepo::new();

    let result = repo.hn(&["remove", "does-not-exist"]);
    result.assert_failure();
}

#[test]
fn test_info_shows_worktree_details() {
    let repo = TestRepo::new();

    repo.hn(&["add", "feature-x"]).assert_success();

    let result = repo.hn(&["info", "feature-x"]);
    result.assert_success();
    result.assert_stdout_contains("feature-x");
    result.assert_stdout_contains("Branch:");
    result.assert_stdout_contains("Path:");
}

#[test]
fn test_switch_worktree() {
    let repo = TestRepo::new();

    repo.hn(&["add", "feature-x"]).assert_success();

    let result = repo.hn(&["switch", "feature-x"]);
    result.assert_success();

    // Switch outputs the path
    assert!(result.stdout.contains("feature-x") || result.stderr.contains("feature-x"));
}

#[test]
fn test_switch_nonexistent_worktree_fails() {
    let repo = TestRepo::new();

    let result = repo.hn(&["switch", "does-not-exist"]);
    result.assert_failure();
}

#[test]
fn test_prune_removes_orphaned_state() {
    let repo = TestRepo::new();

    // Add worktree
    repo.hn(&["add", "feature-x"]).assert_success();

    // Manually remove worktree (simulating orphaned state)
    let worktree_path = repo.worktree_path("feature-x");
    std::fs::remove_dir_all(&worktree_path).expect("Failed to remove worktree dir");

    // Remove from git
    std::process::Command::new("git")
        .args(["worktree", "remove", "--force", "feature-x"])
        .current_dir(repo.path())
        .output()
        .ok(); // May fail if already removed

    // State should still exist
    assert!(repo.state_exists("feature-x"));

    // Prune should clean it up
    let result = repo.hn(&["prune"]);
    result.assert_success();

    // State should be gone
    assert!(!repo.state_exists("feature-x"));
}

#[test]
fn test_full_lifecycle() {
    let repo = TestRepo::new();

    // Add multiple worktrees
    repo.hn(&["add", "feature-1"]).assert_success();
    repo.hn(&["add", "feature-2"]).assert_success();
    repo.hn(&["add", "feature-3"]).assert_success();

    // List them
    let result = repo.hn(&["list"]);
    result.assert_success();
    result.assert_stdout_contains("feature-1");
    result.assert_stdout_contains("feature-2");
    result.assert_stdout_contains("feature-3");

    // Get info
    let result = repo.hn(&["info", "feature-1"]);
    result.assert_success();

    // Switch
    let result = repo.hn(&["switch", "feature-2"]);
    result.assert_success();

    // Remove one
    repo.hn(&["remove", "feature-1"]).assert_success();
    assert!(!repo.worktree_exists("feature-1"));

    // List again
    let result = repo.hn(&["list"]);
    result.assert_success();
    result.assert_stdout_contains("feature-2");
    result.assert_stdout_contains("feature-3");
    assert!(!result.stdout.contains("feature-1"));

    // Clean up
    repo.hn(&["remove", "feature-2"]).assert_success();
    repo.hn(&["remove", "feature-3"]).assert_success();
}
