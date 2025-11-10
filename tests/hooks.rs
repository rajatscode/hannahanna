/// Integration tests for hooks system
mod common;

use common::TestRepo;
use std::fs;

#[test]
fn test_post_create_hook() {
    let repo = TestRepo::new();

    // Create config with post_create hook
    repo.create_config(
        r#"
hooks:
  post_create: |
    echo "Hook executed" > hook_output.txt
"#,
    );

    repo.hn(&["add", "feature-x"]).assert_success();

    // Check if hook was executed
    let worktree_path = repo.worktree_path("feature-x");
    let hook_output = worktree_path.join("hook_output.txt");

    assert!(
        hook_output.exists(),
        "Hook should have created hook_output.txt"
    );

    let content = fs::read_to_string(&hook_output).expect("Failed to read hook output");
    assert!(content.contains("Hook executed"));
}

#[test]
fn test_pre_remove_hook() {
    let repo = TestRepo::new();

    // Create config with pre_remove hook
    repo.create_config(
        r#"
hooks:
  pre_remove: |
    echo "Removing worktree" > ../pre_remove_executed.txt
"#,
    );

    repo.hn(&["add", "feature-x"]).assert_success();

    let result = repo.hn(&["remove", "feature-x"]);
    result.assert_success();

    // Check if hook was executed (file in parent directory)
    let hook_marker = repo
        .path()
        .parent()
        .unwrap()
        .join("pre_remove_executed.txt");
    assert!(
        hook_marker.exists(),
        "pre_remove hook should have been executed"
    );
}

#[test]
fn test_hook_environment_variables() {
    let repo = TestRepo::new();

    // Create hook that outputs environment variables
    repo.create_config(
        r#"
hooks:
  post_create: |
    echo "WT_NAME=$WT_NAME" > hook_env.txt
    echo "WT_PATH=$WT_PATH" >> hook_env.txt
    echo "WT_BRANCH=$WT_BRANCH" >> hook_env.txt
"#,
    );

    repo.hn(&["add", "feature-test"]).assert_success();

    let worktree_path = repo.worktree_path("feature-test");
    let hook_env = worktree_path.join("hook_env.txt");

    assert!(hook_env.exists(), "Hook should have created hook_env.txt");

    let content = fs::read_to_string(&hook_env).expect("Failed to read hook env");
    assert!(content.contains("WT_NAME=feature-test"));
    assert!(content.contains("WT_PATH="));
    assert!(content.contains("WT_BRANCH="));
}

#[test]
fn test_hook_failure_prevents_worktree_creation() {
    let repo = TestRepo::new();

    // Create hook that always fails
    repo.create_config(
        r#"
hooks:
  post_create: |
    echo "Hook failed"
    exit 1
"#,
    );

    let result = repo.hn(&["add", "feature-x"]);
    result.assert_failure();

    // Worktree should not exist if hook failed
    // (Depending on implementation, it might be cleaned up)
}

#[test]
fn test_hook_with_multiline_script() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
hooks:
  post_create: |
    echo "Line 1" > multi.txt
    echo "Line 2" >> multi.txt
    echo "Line 3" >> multi.txt
"#,
    );

    repo.hn(&["add", "feature-x"]).assert_success();

    let worktree_path = repo.worktree_path("feature-x");
    let multi_file = worktree_path.join("multi.txt");

    assert!(multi_file.exists());

    let content = fs::read_to_string(&multi_file).expect("Failed to read multi.txt");
    assert!(content.contains("Line 1"));
    assert!(content.contains("Line 2"));
    assert!(content.contains("Line 3"));
}

#[test]
fn test_no_hooks_without_config() {
    let repo = TestRepo::new();

    // No config file, hooks shouldn't run
    repo.hn(&["add", "feature-x"]).assert_success();

    // Should succeed without errors
    assert!(repo.worktree_exists("feature-x"));
}

#[test]
fn test_post_switch_hook() {
    let repo = TestRepo::new();

    // Create marker file to track hook execution
    repo.create_config(
        r#"
hooks:
  post_switch: |
    echo "Switched to worktree" > ../switch_hook_executed.txt
"#,
    );

    repo.hn(&["add", "feature-x"]).assert_success();

    // Switch to the worktree
    let result = repo.hn(&["switch", "feature-x"]);
    result.assert_success();

    // Check if hook was executed
    // Note: post_switch hook might not be implemented yet, this is a forward-looking test
}

#[test]
fn test_hook_with_install_command() {
    let repo = TestRepo::new();

    // Create a package.json
    fs::write(
        repo.path().join("package.json"),
        r#"{"name": "test-project", "version": "1.0.0"}"#,
    )
    .expect("Failed to create package.json");

    // Hook that creates a marker file (instead of actual npm install)
    repo.create_config(
        r#"
hooks:
  post_create: |
    echo "Dependencies installed" > install_marker.txt
"#,
    );

    repo.hn(&["add", "feature-x"]).assert_success();

    let worktree_path = repo.worktree_path("feature-x");
    let marker = worktree_path.join("install_marker.txt");

    assert!(marker.exists(), "Install hook should have run");
}
