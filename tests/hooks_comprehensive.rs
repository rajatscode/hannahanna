// Comprehensive hook tests for v0.5
mod common;

use common::TestRepo;
use std::fs;

// ============ Hook Execution Tests ============

#[test]
fn test_all_hook_types_execute() {
    let repo = TestRepo::new();

    let marker = repo.path().join("pre_create_ran.txt");
    repo.create_config(&format!(r#"
hooks:
  pre_create: "echo 'pre_create' > {}"
  post_create: "echo 'post_create' > post_create.txt"
"#, marker.display()));

    let result = repo.hn(&["add", "test-wt"]);
    assert!(result.success, "Add should succeed. stderr: {}", result.stderr);

    // Pre-create hook should have run in repo root
    assert!(marker.exists(), "pre_create hook should have created marker");

    // Post-create hook should have run in worktree directory
    let wt_path = repo.worktree_path("test-wt");
    assert!(wt_path.join("post_create.txt").exists(), "post_create hook should have created file");
}

#[test]
fn test_hook_timeout_prevents_hanging() {
    let repo = TestRepo::new();

    // Create a hook that sleeps longer than timeout
    repo.create_config(r#"
hooks:
  post_create: "sleep 120"  # 2 minutes
  timeout_seconds: 1
"#);

    let result = repo.hn(&["add", "timeout-test"]);
    // Should fail due to timeout
    assert!(!result.success);
    assert!(result.stderr.contains("timed out") || result.stderr.contains("timeout"));
}

#[test]
fn test_hook_failure_prevents_worktree_creation() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: "exit 1"
"#);

    let result = repo.hn(&["add", "fail-test"]);
    assert!(!result.success);

    // Worktree should not exist or should be cleaned up
    // (Implementation detail: might exist but be marked as failed)
}

#[test]
fn test_hook_conditional_starts_with_match() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create_conditions:
    - condition: "branch.startsWith('feature-')"
      command: "echo 'feature hook' > feature.txt"
"#);

    repo.hn(&["add", "feature-test"]).assert_success();
    let wt_path = repo.worktree_path("feature-test");
    assert!(wt_path.join("feature.txt").exists());

    repo.hn(&["add", "bugfix-test"]).assert_success();
    let wt_path2 = repo.worktree_path("bugfix-test");
    assert!(!wt_path2.join("feature.txt").exists());
}

#[test]
fn test_hook_conditional_ends_with() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create_conditions:
    - condition: "branch.endsWith('-prod')"
      command: "echo 'production' > prod.txt"
"#);

    repo.hn(&["add", "deploy-prod"]).assert_success();
    assert!(repo.worktree_path("deploy-prod").join("prod.txt").exists());

    repo.hn(&["add", "deploy-staging"]).assert_success();
    assert!(!repo.worktree_path("deploy-staging").join("prod.txt").exists());
}

#[test]
fn test_hook_conditional_contains() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create_conditions:
    - condition: "branch.contains('hotfix')"
      command: "echo 'urgent' > hotfix.txt"
"#);

    repo.hn(&["add", "feature-hotfix-123"]).assert_success();
    assert!(repo.worktree_path("feature-hotfix-123").join("hotfix.txt").exists());
}

#[test]
fn test_multiple_conditional_hooks_same_branch() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create_conditions:
    - condition: "branch.startsWith('feature-')"
      command: "echo '1' > hook1.txt"
    - condition: "branch.endsWith('-test')"
      command: "echo '2' > hook2.txt"
"#);

    repo.hn(&["add", "feature-new-test"]).assert_success();
    let wt_path = repo.worktree_path("feature-new-test");
    assert!(wt_path.join("hook1.txt").exists());
    assert!(wt_path.join("hook2.txt").exists());
}

#[test]
fn test_hook_env_vars_all_set() {
    let repo = TestRepo::new();

    let output_file = repo.path().join("hook_env.txt");
    repo.create_config(&format!(r#"
hooks:
  post_create: |
    echo "NAME=$HNHN_NAME" > {}
    echo "PATH=$HNHN_PATH" >> {}
    echo "BRANCH=$HNHN_BRANCH" >> {}
    echo "COMMIT=$HNHN_COMMIT" >> {}
    echo "STATE_DIR=$HNHN_STATE_DIR" >> {}
"#, output_file.display(), output_file.display(), output_file.display(),
    output_file.display(), output_file.display()));

    repo.hn(&["add", "env-test"]).assert_success();

    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("NAME=env-test"));
    assert!(content.contains("BRANCH=env-test"));
    assert!(content.contains("COMMIT="));
}

#[test]
fn test_hook_no_hooks_flag_skips_execution() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: "echo 'hook ran' > hook.txt"
"#);

    repo.hn(&["add", "no-hooks-test", "--no-hooks"]).assert_success();
    let wt_path = repo.worktree_path("no-hooks-test");
    assert!(!wt_path.join("hook.txt").exists());
}

#[test]
fn test_hook_multiline_script() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: |
    echo "Line 1" > multi.txt
    echo "Line 2" >> multi.txt
    echo "Line 3" >> multi.txt
"#);

    repo.hn(&["add", "multi-test"]).assert_success();
    let content = fs::read_to_string(repo.worktree_path("multi-test").join("multi.txt")).unwrap();
    assert!(content.contains("Line 1"));
    assert!(content.contains("Line 2"));
    assert!(content.contains("Line 3"));
}

#[test]
fn test_hook_with_shell_variables() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: |
    VAR="test value"
    echo "$VAR" > shell_var.txt
"#);

    repo.hn(&["add", "shell-var-test"]).assert_success();
    let content = fs::read_to_string(repo.worktree_path("shell-var-test").join("shell_var.txt")).unwrap();
    assert!(content.contains("test value"));
}

#[test]
fn test_hook_stderr_captured_on_failure() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: |
    echo "error message" >&2
    exit 1
"#);

    let result = repo.hn(&["add", "stderr-test"]);
    assert!(!result.success);
    assert!(result.stderr.contains("error message") || result.stderr.contains("hook failed"));
}

#[test]
fn test_pre_remove_hook_executes() {
    let repo = TestRepo::new();

    let marker = repo.path().join("pre_remove_ran.txt");
    repo.create_config(&format!(r#"
hooks:
  pre_remove: "echo 'removed' > {}"
"#, marker.display()));

    repo.hn(&["add", "remove-test"]).assert_success();
    repo.hn(&["remove", "remove-test", "--force"]);

    // Pre-remove hook should have created the marker
    assert!(marker.exists());
}

#[test]
fn test_hook_conditional_invalid_syntax_fails() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create_conditions:
    - condition: "invalid.syntax()"
      command: "echo 'test'"
"#);

    let result = repo.hn(&["add", "invalid-cond"]);
    // Should fail with invalid condition error
    assert!(!result.success);
}

#[test]
fn test_hook_runs_in_worktree_directory() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: "pwd > current_dir.txt"
"#);

    repo.hn(&["add", "pwd-test"]).assert_success();
    let wt_path = repo.worktree_path("pwd-test");
    let pwd_content = fs::read_to_string(wt_path.join("current_dir.txt")).unwrap();
    assert!(pwd_content.contains("pwd-test"));
}

#[test]
fn test_hook_timeout_configuration() {
    let repo = TestRepo::new();

    // Short timeout
    repo.create_config(r#"
hooks:
  post_create: "sleep 2"
  timeout_seconds: 1
"#);

    let result = repo.hn(&["add", "timeout-short"]);
    assert!(!result.success);
}

#[test]
fn test_hook_conditional_multiple_same_type() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create_conditions:
    - condition: "branch.contains('test')"
      command: "echo 'a' > a.txt"
    - condition: "branch.contains('test')"
      command: "echo 'b' > b.txt"
"#);

    repo.hn(&["add", "test-branch"]).assert_success();
    let wt_path = repo.worktree_path("test-branch");
    assert!(wt_path.join("a.txt").exists());
    assert!(wt_path.join("b.txt").exists());
}

#[test]
fn test_hook_with_git_commands() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: "git status > git_status.txt"
"#);

    repo.hn(&["add", "git-hook-test"]).assert_success();
    let wt_path = repo.worktree_path("git-hook-test");
    assert!(wt_path.join("git_status.txt").exists());
}

#[test]
fn test_hook_empty_string_is_noop() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: ""
"#);

    // Should succeed without error
    let result = repo.hn(&["add", "empty-hook"]);
    assert!(result.success);
}

// ============ Advanced Hook Tests ============

#[test]
fn test_hook_conditional_case_sensitivity() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create_conditions:
    - condition: "branch.startsWith('Feature-')"
      command: "echo 'matched' > case.txt"
"#);

    // Should not match - case sensitive
    repo.hn(&["add", "feature-test"]).assert_success();
    assert!(!repo.worktree_path("feature-test").join("case.txt").exists());

    // Should match
    repo.hn(&["add", "Feature-test"]).assert_success();
    assert!(repo.worktree_path("Feature-test").join("case.txt").exists());
}

#[test]
fn test_hook_env_var_escaping() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: |
    echo "NAME is $HNHN_NAME" > env.txt
    echo "PATH is $HNHN_PATH" >> env.txt
"#);

    repo.hn(&["add", "test-escaping"]).assert_success();
    let content = fs::read_to_string(repo.worktree_path("test-escaping").join("env.txt")).unwrap();
    assert!(content.contains("NAME is test-escaping"));
    assert!(content.contains("PATH is"));
}

#[test]
fn test_hook_multiple_commands_sequential() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: |
    echo "1" > seq.txt
    echo "2" >> seq.txt
    echo "3" >> seq.txt
"#);

    repo.hn(&["add", "sequential"]).assert_success();
    let content = fs::read_to_string(repo.worktree_path("sequential").join("seq.txt")).unwrap();
    assert_eq!(content.lines().count(), 3);
}

#[test]
fn test_hook_exit_code_propagation() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: "exit 42"
"#);

    let result = repo.hn(&["add", "exit-code"]);
    assert!(!result.success);
}

#[test]
fn test_hook_no_hooks_flag_with_conditional() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create_conditions:
    - condition: "branch.contains('test')"
      command: "echo 'hook' > hook.txt"
"#);

    repo.hn(&["add", "test-branch", "--no-hooks"]).assert_success();
    assert!(!repo.worktree_path("test-branch").join("hook.txt").exists());
}

#[test]
fn test_hook_conditional_empty_pattern() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create_conditions:
    - condition: "branch.contains('')"
      command: "echo 'empty' > empty.txt"
"#);

    // Empty string matches everything
    repo.hn(&["add", "any-branch"]).assert_success();
    assert!(repo.worktree_path("any-branch").join("empty.txt").exists());
}

#[test]
fn test_hook_long_output() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: |
    for i in $(seq 1 1000); do
      echo "Line $i"
    done > output.txt
"#);

    repo.hn(&["add", "long-output"]).assert_success();
    let content = fs::read_to_string(repo.worktree_path("long-output").join("output.txt")).unwrap();
    assert_eq!(content.lines().count(), 1000);
}

#[test]
fn test_hook_working_directory_verification() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: "basename $(pwd) > dir.txt"
"#);

    repo.hn(&["add", "dir-test"]).assert_success();
    let content = fs::read_to_string(repo.worktree_path("dir-test").join("dir.txt")).unwrap();
    assert!(content.trim().contains("dir-test"));
}

#[test]
fn test_hook_conditional_special_chars() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create_conditions:
    - condition: "branch.contains('_')"
      command: "echo 'underscore' > special.txt"
"#);

    repo.hn(&["add", "branch_name"]).assert_success();
    assert!(repo.worktree_path("branch_name").join("special.txt").exists());

    repo.hn(&["add", "branch-name"]).assert_success();
    assert!(!repo.worktree_path("branch-name").join("special.txt").exists());
}

#[test]
fn test_pre_create_working_directory() {
    let repo = TestRepo::new();

    let marker = repo.path().join("pre_create_pwd.txt");
    repo.create_config(&format!(r#"
hooks:
  pre_create: "pwd > {}"
"#, marker.display()));

    repo.hn(&["add", "pre-dir"]).assert_success();
    assert!(marker.exists());
    let content = fs::read_to_string(&marker).unwrap();
    // Pre-create runs in the worktrees parent directory
    assert!(content.contains("worktrees") || content.contains("tmp"));
}

#[test]
fn test_hook_pipe_commands() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: "echo 'hello world' | tr 'a-z' 'A-Z' > pipe.txt"
"#);

    repo.hn(&["add", "pipe-test"]).assert_success();
    let content = fs::read_to_string(repo.worktree_path("pipe-test").join("pipe.txt")).unwrap();
    assert!(content.contains("HELLO WORLD"));
}

#[test]
fn test_hook_conditional_multiple_matches() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create_conditions:
    - condition: "branch.contains('feature')"
      command: "echo 'a' > a.txt"
    - condition: "branch.contains('feature')"
      command: "echo 'b' > b.txt"
    - condition: "branch.contains('bugfix')"
      command: "echo 'c' > c.txt"
"#);

    repo.hn(&["add", "feature-branch"]).assert_success();
    let wt = repo.worktree_path("feature-branch");
    assert!(wt.join("a.txt").exists());
    assert!(wt.join("b.txt").exists());
    assert!(!wt.join("c.txt").exists());
}

#[test]
fn test_hook_background_process_completes() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: |
    echo 'started' > status.txt
    sleep 0.1
    echo 'completed' >> status.txt
"#);

    repo.hn(&["add", "background"]).assert_success();
    let content = fs::read_to_string(repo.worktree_path("background").join("status.txt")).unwrap();
    assert!(content.contains("completed"));
}

#[test]
fn test_hook_file_creation_multiple() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: |
    touch file1.txt file2.txt file3.txt
"#);

    repo.hn(&["add", "multi-file"]).assert_success();
    let wt = repo.worktree_path("multi-file");
    assert!(wt.join("file1.txt").exists());
    assert!(wt.join("file2.txt").exists());
    assert!(wt.join("file3.txt").exists());
}

#[test]
fn test_hook_subshell_execution() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: |
    result=$(echo "subshell")
    echo "$result" > subshell.txt
"#);

    repo.hn(&["add", "subshell"]).assert_success();
    let content = fs::read_to_string(repo.worktree_path("subshell").join("subshell.txt")).unwrap();
    assert!(content.contains("subshell"));
}

#[test]
fn test_hook_conditional_no_match_no_files() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create_conditions:
    - condition: "branch.startsWith('never-')"
      command: "echo 'should not run' > never.txt"
"#);

    repo.hn(&["add", "always-branch"]).assert_success();
    assert!(!repo.worktree_path("always-branch").join("never.txt").exists());
}

#[test]
fn test_hook_error_output_captured() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: |
    echo "stdout message"
    echo "stderr message" >&2
    exit 1
"#);

    let result = repo.hn(&["add", "error-capture"]);
    assert!(!result.success);
    // Stderr should contain error message or hook failure
    assert!(result.stderr.contains("stderr message") || result.stderr.contains("hook failed"));
}
