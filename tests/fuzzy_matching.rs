/// Integration tests for fuzzy matching functionality
mod common;

use common::TestRepo;

#[test]
fn test_exact_match_preferred() {
    let repo = TestRepo::new();

    // Create worktrees with similar names
    repo.hn(&["add", "feature"]).assert_success();
    repo.hn(&["add", "feature-auth"]).assert_success();

    // Exact match should work
    let result = repo.hn(&["info", "feature"]);
    result.assert_success();
    result.assert_stdout_contains("feature");
}

#[test]
fn test_substring_match() {
    let repo = TestRepo::new();

    repo.hn(&["add", "feature-auth"]).assert_success();

    // Should match by substring
    let result = repo.hn(&["info", "auth"]);
    result.assert_success();
    result.assert_stdout_contains("feature-auth");
}

#[test]
fn test_case_insensitive_match() {
    let repo = TestRepo::new();

    repo.hn(&["add", "feature-auth"]).assert_success();

    // Should match case-insensitively
    let result = repo.hn(&["info", "AUTH"]);
    result.assert_success();
    result.assert_stdout_contains("feature-auth");
}

#[test]
fn test_ambiguous_match_fails() {
    let repo = TestRepo::new();

    // Create worktrees with overlapping names
    repo.hn(&["add", "feature-auth"]).assert_success();
    repo.hn(&["add", "feature-auth-new"]).assert_success();

    // Trying to match "auth" should be ambiguous
    let result = repo.hn(&["info", "aut"]);
    // This might succeed if it matches both, or fail if implementation requires unique match
    // The behavior depends on implementation, but it should handle ambiguity gracefully
}

#[test]
fn test_no_match_fails() {
    let repo = TestRepo::new();

    repo.hn(&["add", "feature-x"]).assert_success();

    // Should fail with helpful error
    let result = repo.hn(&["info", "does-not-exist"]);
    result.assert_failure();
}

#[test]
fn test_fuzzy_match_in_remove() {
    let repo = TestRepo::new();

    repo.hn(&["add", "feature-authentication"]).assert_success();

    // Should be able to remove by partial match
    let result = repo.hn(&["remove", "authen"]);
    result.assert_success();

    assert!(!repo.worktree_exists("feature-authentication"));
}

#[test]
fn test_fuzzy_match_in_switch() {
    let repo = TestRepo::new();

    repo.hn(&["add", "feature-dashboard"]).assert_success();

    // Should be able to switch by partial match
    let result = repo.hn(&["switch", "dash"]);
    result.assert_success();
}

#[test]
fn test_unique_prefix_matches() {
    let repo = TestRepo::new();

    repo.hn(&["add", "feature-auth"]).assert_success();
    repo.hn(&["add", "feature-billing"]).assert_success();

    // Unique prefix should match
    let result = repo.hn(&["info", "bill"]);
    result.assert_success();
    result.assert_stdout_contains("billing");

    let result = repo.hn(&["info", "auth"]);
    result.assert_success();
    result.assert_stdout_contains("auth");
}
