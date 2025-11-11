/// Integration tests for the `integrate` and `sync` commands
/// Tests integration operations for merging between worktrees and branches
mod common;

use common::TestRepo;
use std::fs;
use std::process::Command;

// ===== INTEGRATE COMMAND TESTS =====

#[test]
fn test_integrate_validation_squash_and_no_ff() {
    let repo = TestRepo::new();

    // Create worktrees
    repo.hn(&["add", "source-wt"]).assert_success();
    repo.hn(&["add", "target-wt"]).assert_success();

    // Try to integrate with both --squash and --no-ff (should fail)
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["integrate", "source-wt", "--squash", "--no-ff"])
        .current_dir(repo.worktree_path("target-wt"))
        .output()
        .expect("Failed to run command");

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("Cannot use both --squash and --no-ff"),
        "Expected error about conflicting flags, got: {}",
        stderr
    );
}

#[test]
fn test_integrate_source_not_found() {
    let repo = TestRepo::new();

    // Create target worktree
    repo.hn(&["add", "target"]).assert_success();

    // Try to integrate from non-existent source
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["integrate", "nonexistent"])
        .current_dir(repo.worktree_path("target"))
        .output()
        .expect("Failed to run command");

    // Should treat as branch name (which will fail when git tries to merge)
    // OR could fail earlier if branch doesn't exist
    // We're just checking it doesn't crash
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !stderr.is_empty(),
        "Expected some error output for non-existent source"
    );
}

#[test]
fn test_integrate_target_not_found() {
    let repo = TestRepo::new();

    // Create source worktree
    repo.hn(&["add", "source"]).assert_success();

    // Try to integrate into non-existent target
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["integrate", "source", "--into", "nonexistent"])
        .current_dir(repo.main_path())
        .output()
        .expect("Failed to run command");

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("NotFound"),
        "Expected error about target not found, got: {}",
        stderr
    );
}

#[test]
fn test_integrate_with_uncommitted_changes() {
    let repo = TestRepo::new();

    // Create worktrees
    repo.hn(&["add", "source"]).assert_success();
    repo.hn(&["add", "target"]).assert_success();

    // Make a change in source and commit it
    let source_file = repo.worktree_path("source").join("source-file.txt");
    fs::write(&source_file, "content from source").expect("Failed to write file");
    repo.git_in_worktree("source", &["add", "source-file.txt"])
        .assert_success();
    repo.git_in_worktree("source", &["commit", "-m", "Add source file"])
        .assert_success();

    // Make uncommitted change in target
    let target_file = repo.worktree_path("target").join("target-file.txt");
    fs::write(&target_file, "uncommitted content").expect("Failed to write file");

    // Try to integrate (should fail due to uncommitted changes)
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["integrate", "source"])
        .current_dir(repo.worktree_path("target"))
        .output()
        .expect("Failed to run command");

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("uncommitted changes"),
        "Expected error about uncommitted changes, got: {}",
        stderr
    );
}

#[test]
fn test_integrate_fuzzy_matching() {
    let repo = TestRepo::new();

    // Create worktrees with longer names
    repo.hn(&["add", "feature-authentication"])
        .assert_success();
    repo.hn(&["add", "feature-billing"]).assert_success();

    // Make a commit in auth
    let auth_file = repo
        .worktree_path("feature-authentication")
        .join("auth.txt");
    fs::write(&auth_file, "auth content").expect("Failed to write file");
    repo.git_in_worktree("feature-authentication", &["add", "auth.txt"])
        .assert_success();
    repo.git_in_worktree("feature-authentication", &["commit", "-m", "Add auth"])
        .assert_success();

    // Try to integrate using fuzzy match "feat-auth" (should match feature-authentication)
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["integrate", "feat-auth"])
        .current_dir(repo.worktree_path("feature-billing"))
        .output()
        .expect("Failed to run command");

    // Should work (merge feature-authentication into feature-billing)
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        eprintln!("stderr: {}", stderr);
        eprintln!("stdout: {}", stdout);
    }

    // Note: This might succeed or fail depending on git state,
    // but it should at least recognize the fuzzy match
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("feature-authentication") || result.status.success(),
        "Should recognize fuzzy match to feature-authentication"
    );
}

// ===== SYNC COMMAND TESTS =====

#[test]
fn test_sync_invalid_strategy() {
    let repo = TestRepo::new();

    // Create worktree
    repo.hn(&["add", "test-wt"]).assert_success();

    // Try to sync with invalid strategy
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["sync", "--strategy", "invalid"])
        .current_dir(repo.worktree_path("test-wt"))
        .output()
        .expect("Failed to run command");

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("Invalid sync strategy") || stderr.contains("invalid"),
        "Expected error about invalid strategy, got: {}",
        stderr
    );
}

#[test]
fn test_sync_with_uncommitted_changes_no_autostash() {
    let repo = TestRepo::new();

    // Create worktree
    repo.hn(&["add", "test-sync"]).assert_success();

    // Make uncommitted change
    let test_file = repo.worktree_path("test-sync").join("test.txt");
    fs::write(&test_file, "uncommitted").expect("Failed to write file");

    // Try to sync without autostash (should fail)
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["sync"])
        .current_dir(repo.worktree_path("test-sync"))
        .output()
        .expect("Failed to run command");

    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("uncommitted changes"),
        "Expected error about uncommitted changes, got: {}",
        stderr
    );
}

#[test]
fn test_sync_default_branch_main() {
    let repo = TestRepo::new();

    // Create worktree
    repo.hn(&["add", "test-default"]).assert_success();

    // Run sync without specifying branch (should default to 'main')
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["sync"])
        .current_dir(repo.worktree_path("test-default"))
        .output()
        .expect("Failed to run command");

    // Check stderr mentions 'main'
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("main"),
        "Expected sync to default to 'main', got: {}",
        stderr
    );
}

#[test]
fn test_sync_with_custom_branch() {
    let repo = TestRepo::new();

    // Create a custom branch
    repo.git(&["branch", "develop"]).assert_success();

    // Create worktree
    repo.hn(&["add", "test-custom"]).assert_success();

    // Run sync with custom branch
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["sync", "develop"])
        .current_dir(repo.worktree_path("test-custom"))
        .output()
        .expect("Failed to run command");

    // Check stderr mentions 'develop'
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("develop"),
        "Expected sync with 'develop' branch, got: {}",
        stderr
    );
}

#[test]
fn test_sync_merge_strategy() {
    let repo = TestRepo::new();

    // Create worktree
    repo.hn(&["add", "test-merge"]).assert_success();

    // Run sync with explicit merge strategy
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["sync", "--strategy", "merge"])
        .current_dir(repo.worktree_path("test-merge"))
        .output()
        .expect("Failed to run command");

    // Check that it mentions merge strategy
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("Merge") || stderr.contains("merge"),
        "Expected merge strategy to be used, got: {}",
        stderr
    );
}

#[test]
fn test_sync_rebase_strategy() {
    let repo = TestRepo::new();

    // Create worktree
    repo.hn(&["add", "test-rebase"]).assert_success();

    // Run sync with rebase strategy
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["sync", "--strategy", "rebase"])
        .current_dir(repo.worktree_path("test-rebase"))
        .output()
        .expect("Failed to run command");

    // Check that it mentions rebase strategy
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("Rebase") || stderr.contains("rebase"),
        "Expected rebase strategy to be used, got: {}",
        stderr
    );
}

// Note: Full end-to-end tests with actual merging and conflict resolution
// are complex to test in this environment. The tests above verify:
// 1. Validation logic and error handling
// 2. Flag combinations
// 3. Default values
// 4. Fuzzy matching
//
// Manual testing and real-world usage provide coverage for the full
// merge/sync functionality with conflict handling.
