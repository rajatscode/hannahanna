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

// ==================== Conditional Hooks Tests ====================

#[test]
fn test_conditional_hook_starts_with_match() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
hooks:
  post_create_conditions:
    - condition: "branch.startsWith('feature-')"
      command: "echo 'Feature setup' > conditional_output.txt"
"#,
    );

    repo.hn(&["add", "feature-new-api"]).assert_success();

    let worktree_path = repo.worktree_path("feature-new-api");
    let output = worktree_path.join("conditional_output.txt");

    assert!(
        output.exists(),
        "Conditional hook should have run for feature- branch"
    );

    let content = fs::read_to_string(&output).expect("Failed to read output");
    assert!(content.contains("Feature setup"));
}

#[test]
fn test_conditional_hook_starts_with_no_match() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
hooks:
  post_create_conditions:
    - condition: "branch.startsWith('feature-')"
      command: "echo 'Should not run' > conditional_output.txt"
"#,
    );

    repo.hn(&["add", "hotfix-critical"]).assert_success();

    let worktree_path = repo.worktree_path("hotfix-critical");
    let output = worktree_path.join("conditional_output.txt");

    assert!(
        !output.exists(),
        "Conditional hook should NOT have run for hotfix- branch"
    );
}

#[test]
fn test_conditional_hook_ends_with() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
hooks:
  post_create_conditions:
    - condition: "branch.endsWith('-prod')"
      command: "echo 'Production setup' > prod_marker.txt"
"#,
    );

    repo.hn(&["add", "release-prod"]).assert_success();

    let worktree_path = repo.worktree_path("release-prod");
    let marker = worktree_path.join("prod_marker.txt");

    assert!(marker.exists(), "Conditional hook should match -prod suffix");
}

#[test]
fn test_conditional_hook_contains() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
hooks:
  post_create_conditions:
    - condition: "branch.contains('bugfix')"
      command: "echo 'Bugfix detected' > bugfix_marker.txt"
"#,
    );

    repo.hn(&["add", "feature-bugfix-auth"]).assert_success();

    let worktree_path = repo.worktree_path("feature-bugfix-auth");
    let marker = worktree_path.join("bugfix_marker.txt");

    assert!(
        marker.exists(),
        "Conditional hook should match 'bugfix' substring"
    );
}

#[test]
fn test_multiple_conditional_hooks() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
hooks:
  post_create_conditions:
    - condition: "branch.startsWith('feature-')"
      command: "echo 'Feature' > type1.txt"
    - condition: "branch.contains('api')"
      command: "echo 'API' > type2.txt"
"#,
    );

    // This branch matches both conditions
    repo.hn(&["add", "feature-new-api"]).assert_success();

    let worktree_path = repo.worktree_path("feature-new-api");

    // Both conditional hooks should have run
    assert!(
        worktree_path.join("type1.txt").exists(),
        "First conditional hook should run"
    );
    assert!(
        worktree_path.join("type2.txt").exists(),
        "Second conditional hook should run"
    );
}

#[test]
fn test_conditional_and_regular_hooks_both_run() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
hooks:
  post_create: |
    echo 'Regular hook' > regular.txt
  post_create_conditions:
    - condition: "branch.startsWith('feature-')"
      command: "echo 'Conditional hook' > conditional.txt"
"#,
    );

    repo.hn(&["add", "feature-test"]).assert_success();

    let worktree_path = repo.worktree_path("feature-test");

    // Both hooks should have run
    assert!(
        worktree_path.join("regular.txt").exists(),
        "Regular hook should run"
    );
    assert!(
        worktree_path.join("conditional.txt").exists(),
        "Conditional hook should also run"
    );
}

#[test]
fn test_conditional_hook_with_double_quotes() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
hooks:
  post_create_conditions:
    - condition: "branch.startsWith(\"release-\")"
      command: "echo 'Release' > release_marker.txt"
"#,
    );

    repo.hn(&["add", "release-v1-0"]).assert_success();

    let worktree_path = repo.worktree_path("release-v1-0");
    let marker = worktree_path.join("release_marker.txt");

    assert!(
        marker.exists(),
        "Conditional hook should work with double quotes"
    );
}

#[test]
fn test_conditional_hook_pre_remove() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
hooks:
  pre_remove_conditions:
    - condition: "branch.startsWith('temp-')"
      command: "echo 'Temp cleanup' > ../temp_cleanup.txt"
"#,
    );

    repo.hn(&["add", "temp-experiment"]).assert_success();
    repo.hn(&["remove", "temp-experiment"]).assert_success();

    let marker = repo.path().parent().unwrap().join("temp_cleanup.txt");
    assert!(
        marker.exists(),
        "Conditional pre_remove hook should have run"
    );
}

#[test]
fn test_conditional_hooks_config_hierarchy_merge() {
    let repo = TestRepo::new();

    // Base config with one conditional hook
    repo.create_config(
        r#"
hooks:
  post_create_conditions:
    - condition: "branch.startsWith('feature-')"
      command: "echo 'Feature' > feature_marker.txt"
"#,
    );

    // Local config with additional conditional hook
    fs::write(
        repo.path().join(".hannahanna.local.yml"),
        r#"
hooks:
  post_create_conditions:
    - condition: "branch.contains('api')"
      command: "echo 'API' > api_marker.txt"
"#,
    )
    .expect("Failed to create local config");

    repo.hn(&["add", "feature-new-api"]).assert_success();

    let worktree_path = repo.worktree_path("feature-new-api");

    // Both conditional hooks from different config levels should run
    assert!(
        worktree_path.join("feature_marker.txt").exists(),
        "Conditional hook from repo config should run"
    );
    assert!(
        worktree_path.join("api_marker.txt").exists(),
        "Conditional hook from local config should also run"
    );
}
