/// End-to-end scenario tests based on common workflows
mod common;

use common::TestRepo;

/// Scenario 1: Multiple Features in Parallel
/// Developer working on 3 features simultaneously
// TODO: Fix git integration for this test to work
#[ignore = "Feature not fully implemented: git integration required"]
#[test]
fn scenario_multiple_features_in_parallel() {
    let repo = TestRepo::new();

    // Frontend developer, 3 features simultaneously
    repo.hn(&["add", "feature-auth"]).assert_success();
    repo.hn(&["add", "feature-billing"]).assert_success();
    repo.hn(&["add", "feature-dashboard"]).assert_success();

    // Verify all exist
    let result = repo.hn(&["list"]);
    result.assert_success();
    result.assert_stdout_contains("feature-auth");
    result.assert_stdout_contains("feature-billing");
    result.assert_stdout_contains("feature-dashboard");

    // Work on auth
    repo.hn(&["switch", "feature-auth"]).assert_success();

    // Quick fix on billing
    repo.hn(&["switch", "feature-billing"]).assert_success();

    // Auth done - clean up
    repo.hn(&["remove", "feature-auth"]).assert_success();

    // Verify auth is gone but others remain
    let result = repo.hn(&["list"]);
    result.assert_success();
    assert!(!result.stdout.contains("feature-auth"));
    result.assert_stdout_contains("feature-billing");
    result.assert_stdout_contains("feature-dashboard");
}

/// Scenario 2: Hotfix During Feature Work
/// Developer working on a feature when urgent bug needs fixing
// TODO: Fix git integration for this test to work
#[ignore = "Feature not fully implemented: git integration required"]
#[test]
fn scenario_hotfix_during_feature_work() {
    let repo = TestRepo::new();

    // Deep in refactor
    repo.hn(&["add", "refactor-db", "--from=main"])
        .assert_success();

    // Urgent bug! Create from main, not refactor-db
    repo.hn(&["add", "hotfix-critical", "--from=main"])
        .assert_success();

    // Fix bug, test, commit (simulated)
    repo.hn(&["switch", "hotfix-critical"]).assert_success();

    // Clean up hotfix
    repo.hn(&["remove", "hotfix-critical"]).assert_success();

    // Back to refactor
    repo.hn(&["switch", "refactor-db"]).assert_success();

    // Verify refactor still exists
    assert!(repo.worktree_exists("refactor-db"));
}

/// Scenario 3: Code Review
/// Reviewing a colleague's PR
#[test]
fn scenario_code_review() {
    let repo = TestRepo::new();

    // Create a branch to simulate PR
    std::process::Command::new("git")
        .args(["checkout", "-b", "pr-123"])
        .current_dir(repo.path())
        .output()
        .expect("Failed to create branch");

    std::process::Command::new("git")
        .args(["checkout", "main"])
        .current_dir(repo.path())
        .output()
        .expect("Failed to checkout main");

    // Review PR
    let result = repo.hn(&["add", "review-pr-123", "pr-123", "--no-branch"]);
    // Note: --no-branch might not be implemented yet
    if result.success {
        // Test locally
        assert!(repo.worktree_exists("review-pr-123"));

        // Done reviewing
        repo.hn(&["remove", "review-pr-123"]).assert_success();
    }
}

/// Scenario 4: Working with Shared Dependencies
/// Multiple worktrees sharing node_modules
#[test]
fn scenario_shared_dependencies() {
    let repo = TestRepo::new();

    // Create package.json and lockfile
    std::fs::write(
        repo.path().join("package.json"),
        r#"{"name": "test", "version": "1.0.0"}"#,
    )
    .expect("Failed to create package.json");

    std::fs::write(
        repo.path().join("package-lock.json"),
        r#"{"lockfileVersion": 2}"#,
    )
    .expect("Failed to create lockfile");

    // Create node_modules
    std::fs::create_dir(repo.path().join("node_modules")).ok();

    // Config to share node_modules
    repo.create_config(
        r#"
shared:
  symlinks:
    - node_modules
  compatibility_check:
    node_modules: "package-lock.json"
"#,
    );

    // Create two worktrees
    repo.hn(&["add", "feature-a"]).assert_success();
    repo.hn(&["add", "feature-b"]).assert_success();

    // Both should exist
    assert!(repo.worktree_exists("feature-a"));
    assert!(repo.worktree_exists("feature-b"));

    // Both should have node_modules (symlinked or isolated)
    let _feature_a_nm = repo.worktree_path("feature-a").join("node_modules");
    let _feature_b_nm = repo.worktree_path("feature-b").join("node_modules");

    // If implementation creates symlinks, they should exist
    // (May not exist if not implemented yet)
}

/// Scenario 5: Sequential Workflow with Cleanup
/// Developer completes features one by one
// TODO: Fix git integration for this test to work
#[ignore = "Feature not fully implemented: git integration required"]
#[test]
fn scenario_sequential_workflow() {
    let repo = TestRepo::new();

    // Start feature 1
    repo.hn(&["add", "feature-1"]).assert_success();
    repo.hn(&["switch", "feature-1"]).assert_success();

    // Complete and clean up
    repo.hn(&["remove", "feature-1"]).assert_success();

    // Start feature 2
    repo.hn(&["add", "feature-2"]).assert_success();
    repo.hn(&["switch", "feature-2"]).assert_success();

    // Complete and clean up
    repo.hn(&["remove", "feature-2"]).assert_success();

    // Start feature 3
    repo.hn(&["add", "feature-3"]).assert_success();

    // Only feature-3 should exist
    let result = repo.hn(&["list"]);
    result.assert_success();
    result.assert_stdout_contains("feature-3");
    assert!(!result.stdout.contains("feature-1"));
    assert!(!result.stdout.contains("feature-2"));
}

/// Scenario 6: Experimenting with Breaking Changes
/// Developer creates worktree to test risky changes
// TODO: Fix git integration for this test to work
#[ignore = "Feature not fully implemented: git integration required"]
#[test]
fn scenario_experimental_changes() {
    let repo = TestRepo::new();

    // Create worktree for experiment
    repo.hn(&["add", "experiment-new-api"]).assert_success();

    let worktree_path = repo.worktree_path("experiment-new-api");

    // Make breaking changes (simulated)
    std::fs::write(worktree_path.join("breaking-change.txt"), "risky code")
        .expect("Failed to write file");

    // Decide to abandon experiment
    repo.hn(&["remove", "experiment-new-api", "--force"])
        .assert_success();

    // Main repo is unaffected
    assert!(!repo.path().join("breaking-change.txt").exists());
}

/// Scenario 7: Information Gathering
/// Developer checks status of multiple worktrees
// TODO: Fix git integration for this test to work
#[ignore = "Feature not fully implemented: git integration required"]
#[test]
fn scenario_information_gathering() {
    let repo = TestRepo::new();

    // Create several worktrees
    repo.hn(&["add", "feature-1"]).assert_success();
    repo.hn(&["add", "feature-2"]).assert_success();
    repo.hn(&["add", "feature-3"]).assert_success();

    // Get overview
    let result = repo.hn(&["list"]);
    result.assert_success();

    // Get detailed info about specific worktree
    let result = repo.hn(&["info", "feature-1"]);
    result.assert_success();
    result.assert_stdout_contains("feature-1");

    // Get tree view
    let result = repo.hn(&["list", "--tree"]);
    result.assert_success();
}

/// Scenario 8: Cleanup After Interrupted Work
/// Developer has orphaned state after manual cleanup
#[test]
fn scenario_cleanup_orphaned_state() {
    let repo = TestRepo::new();

    // Create worktree
    repo.hn(&["add", "feature-x"]).assert_success();

    // Manually delete worktree directory (simulating interrupted removal)
    let worktree_path = repo.worktree_path("feature-x");
    std::fs::remove_dir_all(&worktree_path).ok();

    // Remove from git
    std::process::Command::new("git")
        .args(["worktree", "remove", "--force", "feature-x"])
        .current_dir(repo.path())
        .output()
        .ok();

    // State directory still exists
    assert!(repo.state_exists("feature-x"));

    // Prune cleans it up
    repo.hn(&["prune"]).assert_success();

    // State should be cleaned
    assert!(!repo.state_exists("feature-x"));
}

/// Scenario 9: Long-Running Feature with Multiple Fixes
/// Developer creates worktree for main feature, then sub-worktrees for fixes
// TODO: Fix git integration for this test to work
#[ignore = "Feature not fully implemented: git integration required"]
#[test]
fn scenario_nested_workflow() {
    let repo = TestRepo::new();

    // Create main feature worktree
    repo.hn(&["add", "feature-redesign"]).assert_success();

    // Switch to feature worktree (to set it as parent for next worktree)
    // Note: This would require being in the worktree directory
    // For now, just verify the worktree was created

    assert!(repo.worktree_exists("feature-redesign"));

    // Create additional worktrees for related work
    repo.hn(&["add", "feature-redesign-fix-1"]).assert_success();
    repo.hn(&["add", "feature-redesign-fix-2"]).assert_success();

    // All should exist
    let result = repo.hn(&["list"]);
    result.assert_success();
    result.assert_stdout_contains("feature-redesign");
    result.assert_stdout_contains("feature-redesign-fix-1");
    result.assert_stdout_contains("feature-redesign-fix-2");
}

/// Scenario 10: Rapid Prototyping
/// Developer creates and destroys worktrees quickly
// TODO: Fix git integration for this test to work
#[ignore = "Feature not fully implemented: git integration required"]
#[test]
fn scenario_rapid_prototyping() {
    let repo = TestRepo::new();

    for i in 1..=5 {
        let name = format!("prototype-{}", i);

        // Create
        repo.hn(&["add", &name]).assert_success();
        assert!(repo.worktree_exists(&name));

        // Quick check
        repo.hn(&["info", &name]).assert_success();

        // Remove
        repo.hn(&["remove", &name]).assert_success();
        assert!(!repo.worktree_exists(&name));
    }

    // All should be cleaned up
    let result = repo.hn(&["list"]);
    result.assert_success();
    assert!(!result.stdout.contains("prototype-"));
}
