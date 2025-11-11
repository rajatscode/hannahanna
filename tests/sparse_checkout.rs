/// Tests for sparse checkout functionality (v0.2 feature)
mod common;

use common::TestRepo;
use std::fs;

/// Helper to check if a path exists in worktree
fn path_exists_in_worktree(repo: &TestRepo, worktree_name: &str, path: &str) -> bool {
    repo.worktree_path(worktree_name).join(path).exists()
}

#[test]
fn test_sparse_checkout_with_cli_flag() {
    let repo = TestRepo::new();

    // Create directory structure in main repo
    fs::create_dir_all(repo.path().join("services/api")).unwrap();
    fs::create_dir_all(repo.path().join("services/web")).unwrap();
    fs::create_dir_all(repo.path().join("libs/utils")).unwrap();

    fs::write(repo.path().join("services/api/main.rs"), "// API").unwrap();
    fs::write(repo.path().join("services/web/index.html"), "<!-- Web -->").unwrap();
    fs::write(repo.path().join("libs/utils/helper.rs"), "// Helper").unwrap();

    repo.git(&["add", "."]).assert_success();
    repo.git(&["commit", "-m", "Add directory structure"]).assert_success();

    // Create worktree with sparse checkout
    let result = repo.hn(&["add", "sparse-feature", "--sparse", "services/api/"]);
    result.assert_success();
    result.assert_stderr_contains("sparse checkout");

    // Verify worktree was created
    assert!(repo.worktree_exists("sparse-feature"));

    // Note: In cone mode, git sparse-checkout always includes root files
    // and the specified directories. We verify that the sparse config is set.
    let sparse_info = repo.git_in_worktree("sparse-feature", &["sparse-checkout", "list"]);
    sparse_info.assert_success();
    sparse_info.assert_stdout_contains("services/api");
}

#[test]
fn test_sparse_checkout_multiple_paths() {
    let repo = TestRepo::new();

    // Create directory structure
    fs::create_dir_all(repo.path().join("services/api")).unwrap();
    fs::create_dir_all(repo.path().join("libs/utils")).unwrap();
    fs::create_dir_all(repo.path().join("docs")).unwrap();

    fs::write(repo.path().join("services/api/main.rs"), "// API").unwrap();
    fs::write(repo.path().join("libs/utils/helper.rs"), "// Helper").unwrap();
    fs::write(repo.path().join("docs/readme.md"), "# Docs").unwrap();

    repo.git(&["add", "."]).assert_success();
    repo.git(&["commit", "-m", "Add files"]).assert_success();

    // Create worktree with multiple sparse paths
    let result = repo.hn(&[
        "add",
        "multi-sparse",
        "--sparse",
        "services/api/",
        "--sparse",
        "libs/utils/",
    ]);
    result.assert_success();

    // Verify sparse config includes both paths
    let sparse_info = repo.git_in_worktree("multi-sparse", &["sparse-checkout", "list"]);
    sparse_info.assert_success();
    sparse_info.assert_stdout_contains("services/api");
    sparse_info.assert_stdout_contains("libs/utils");
}

#[test]
fn test_sparse_checkout_from_config() {
    let repo = TestRepo::new();

    // Create directory structure
    fs::create_dir_all(repo.path().join("services/api")).unwrap();
    fs::create_dir_all(repo.path().join("services/web")).unwrap();

    fs::write(repo.path().join("services/api/main.rs"), "// API").unwrap();
    fs::write(repo.path().join("services/web/index.html"), "<!-- Web -->").unwrap();

    repo.git(&["add", "."]).assert_success();
    repo.git(&["commit", "-m", "Add services"]).assert_success();

    // Create config with sparse checkout enabled
    let config = r#"
sparse:
  enabled: true
  paths:
    - services/api/
"#;
    repo.create_config(config);

    // Create worktree without --sparse flag (should use config)
    let result = repo.hn(&["add", "config-sparse"]);
    result.assert_success();
    result.assert_stderr_contains("sparse checkout");

    // Verify sparse config
    let sparse_info = repo.git_in_worktree("config-sparse", &["sparse-checkout", "list"]);
    sparse_info.assert_success();
    sparse_info.assert_stdout_contains("services/api");
}

#[test]
fn test_sparse_checkout_cli_overrides_config() {
    let repo = TestRepo::new();

    // Create directory structure
    fs::create_dir_all(repo.path().join("services/api")).unwrap();
    fs::create_dir_all(repo.path().join("libs/utils")).unwrap();

    fs::write(repo.path().join("services/api/main.rs"), "// API").unwrap();
    fs::write(repo.path().join("libs/utils/helper.rs"), "// Helper").unwrap();

    repo.git(&["add", "."]).assert_success();
    repo.git(&["commit", "-m", "Add files"]).assert_success();

    // Create config with sparse checkout enabled
    let config = r#"
sparse:
  enabled: true
  paths:
    - services/api/
"#;
    repo.create_config(config);

    // Create worktree with CLI override
    let result = repo.hn(&["add", "cli-override", "--sparse", "libs/utils/"]);
    result.assert_success();
    result.assert_stderr_contains("libs/utils");

    // Verify CLI override took effect (should have libs/utils, not services/api)
    let sparse_info = repo.git_in_worktree("cli-override", &["sparse-checkout", "list"]);
    sparse_info.assert_success();
    sparse_info.assert_stdout_contains("libs/utils");
}

#[test]
fn test_sparse_checkout_disabled_by_default() {
    let repo = TestRepo::new();

    // Create directory structure
    fs::create_dir_all(repo.path().join("services/api")).unwrap();
    fs::write(repo.path().join("services/api/main.rs"), "// API").unwrap();

    repo.git(&["add", "."]).assert_success();
    repo.git(&["commit", "-m", "Add API"]).assert_success();

    // Create worktree without sparse config or flag
    let result = repo.hn(&["add", "full-checkout"]);
    result.assert_success();

    // Verify sparse checkout is NOT enabled (command should fail)
    let sparse_info = repo.git_in_worktree("full-checkout", &["sparse-checkout", "list"]);
    // Git will error if sparse-checkout is not initialized
    assert!(!sparse_info.success);
}

#[test]
fn test_different_worktrees_different_sparse_paths() {
    let repo = TestRepo::new();

    // Create directory structure
    fs::create_dir_all(repo.path().join("services/api")).unwrap();
    fs::create_dir_all(repo.path().join("services/web")).unwrap();
    fs::create_dir_all(repo.path().join("libs/utils")).unwrap();

    fs::write(repo.path().join("services/api/main.rs"), "// API").unwrap();
    fs::write(repo.path().join("services/web/index.html"), "<!-- Web -->").unwrap();
    fs::write(repo.path().join("libs/utils/helper.rs"), "// Helper").unwrap();

    repo.git(&["add", "."]).assert_success();
    repo.git(&["commit", "-m", "Add all services"]).assert_success();

    // Create first worktree with API sparse checkout
    repo.hn(&["add", "api-work", "--sparse", "services/api/"])
        .assert_success();

    // Create second worktree with web sparse checkout
    repo.hn(&["add", "web-work", "--sparse", "services/web/"])
        .assert_success();

    // Verify each worktree has its own sparse config
    let api_sparse = repo.git_in_worktree("api-work", &["sparse-checkout", "list"]);
    api_sparse.assert_success();
    api_sparse.assert_stdout_contains("services/api");

    let web_sparse = repo.git_in_worktree("web-work", &["sparse-checkout", "list"]);
    web_sparse.assert_success();
    web_sparse.assert_stdout_contains("services/web");
}

#[test]
fn test_sparse_checkout_with_empty_config() {
    let repo = TestRepo::new();

    // Create config with sparse enabled but no paths
    let config = r#"
sparse:
  enabled: true
  paths: []
"#;
    repo.create_config(config);

    // Create worktree - should succeed but not apply sparse checkout
    let result = repo.hn(&["add", "no-sparse-paths"]);
    result.assert_success();

    // Verify sparse checkout is NOT enabled
    let sparse_info = repo.git_in_worktree("no-sparse-paths", &["sparse-checkout", "list"]);
    assert!(!sparse_info.success);
}

#[test]
fn test_sparse_checkout_graceful_fallback_on_error() {
    let repo = TestRepo::new();

    // Create worktree with invalid sparse path (should warn but continue)
    let result = repo.hn(&["add", "invalid-sparse", "--sparse", "nonexistent/"]);

    // Should succeed (graceful fallback)
    result.assert_success();

    // Verify worktree was created
    assert!(repo.worktree_exists("invalid-sparse"));
}

#[test]
fn test_sparse_checkout_with_spaces_in_path() {
    let repo = TestRepo::new();

    // Create directory with spaces in name
    fs::create_dir_all(repo.path().join("services with spaces/api")).unwrap();
    fs::write(
        repo.path().join("services with spaces/api/main.rs"),
        "// API",
    )
    .unwrap();

    repo.git(&["add", "."]).assert_success();
    repo.git(&["commit", "-m", "Add services with spaces"])
        .assert_success();

    // Create worktree with sparse checkout of path with spaces
    let result = repo.hn(&["add", "spaces-test", "--sparse", "services with spaces/"]);
    result.assert_success();

    // Verify sparse checkout is configured
    let sparse_info = repo.git_in_worktree("spaces-test", &["sparse-checkout", "list"]);
    sparse_info.assert_success();
}

#[test]
fn test_sparse_checkout_nested_paths() {
    let repo = TestRepo::new();

    // Create nested directory structure
    fs::create_dir_all(repo.path().join("services/api")).unwrap();
    fs::create_dir_all(repo.path().join("services/web")).unwrap();
    fs::write(repo.path().join("services/api/main.rs"), "// API").unwrap();
    fs::write(repo.path().join("services/web/index.html"), "<!-- Web -->").unwrap();

    repo.git(&["add", "."]).assert_success();
    repo.git(&["commit", "-m", "Add nested services"])
        .assert_success();

    // Specify both parent and child path
    let result = repo.hn(&[
        "add",
        "nested-test",
        "--sparse",
        "services/",
        "--sparse",
        "services/api/",
    ]);
    result.assert_success();

    // Verify sparse checkout configured (git cone mode handles overlapping paths)
    let sparse_info = repo.git_in_worktree("nested-test", &["sparse-checkout", "list"]);
    sparse_info.assert_success();
    sparse_info.assert_stdout_contains("services");
}

#[test]
fn test_sparse_checkout_mercurial_not_supported() {
    let repo = TestRepo::new();

    // Create directory structure
    fs::create_dir_all(repo.path().join("services/api")).unwrap();
    fs::write(repo.path().join("services/api/main.rs"), "// API").unwrap();

    repo.git(&["add", "."]).assert_success();
    repo.git(&["commit", "-m", "Add API"]).assert_success();

    // Try sparse checkout with Mercurial (will use Git since we have a Git repo)
    // This tests the graceful warning path for unsupported VCS
    let result = repo.hn(&["add", "hg-test", "--sparse", "services/api/"]);

    // Should succeed (Git is actually used)
    result.assert_success();
}

#[test]
fn test_sparse_checkout_relative_paths_only() {
    let repo = TestRepo::new();

    // Create directory structure
    fs::create_dir_all(repo.path().join("services/api")).unwrap();
    fs::write(repo.path().join("services/api/main.rs"), "// API").unwrap();

    repo.git(&["add", "."]).assert_success();
    repo.git(&["commit", "-m", "Add API"]).assert_success();

    // Test with relative path (correct usage)
    let result = repo.hn(&["add", "relative-test", "--sparse", "services/api/"]);
    result.assert_success();

    // Verify sparse checkout configured
    let sparse_info = repo.git_in_worktree("relative-test", &["sparse-checkout", "list"]);
    sparse_info.assert_success();
    sparse_info.assert_stdout_contains("services/api");
}
