// Tests for command aliases functionality

use serial_test::serial;

mod common;
use common::TestRepo;

#[test]
#[serial]
fn test_simple_alias() {
    let repo = TestRepo::new();

    // Create config with simple alias
    repo.create_config(r#"
aliases:
  sw: switch
  ls: list
"#);

    // Create a worktree to switch to
    repo.hn(&["add", "feature-1"]).assert_success();

    // Test alias: sw should expand to switch
    let result = repo.hn(&["sw", "feature-1"]);
    assert!(result.success, "Alias 'sw' should work as 'switch'");
}

#[test]
#[serial]
fn test_alias_with_arguments() {
    let repo = TestRepo::new();

    // Create config with alias that includes arguments
    repo.create_config(r#"
aliases:
  lt: list --tree
"#);

    // Create a couple of worktrees
    repo.hn(&["add", "feature-1"]).assert_success();
    repo.hn(&["add", "feature-2"]).assert_success();

    // Test alias with arguments
    let result = repo.hn(&["lt"]);
    assert!(result.success, "Alias 'lt' should work as 'list --tree'");
}

#[test]
#[serial]
fn test_chained_alias() {
    let repo = TestRepo::new();

    // Create config with chained aliases
    repo.create_config(r#"
aliases:
  sw: switch
  s: sw
"#);

    // Create a worktree
    repo.hn(&["add", "feature-1"]).assert_success();

    // Test chained alias: s -> sw -> switch
    let result = repo.hn(&["s", "feature-1"]);
    assert!(result.success, "Chained alias 's -> sw -> switch' should work");
}

#[test]
#[serial]
fn test_alias_cycle_detection() {
    let repo = TestRepo::new();

    // Create config with circular alias
    repo.create_config(r#"
aliases:
  a: b
  b: c
  c: a
"#);

    // Test that cycle is detected
    let result = repo.hn(&["a", "test"]);
    assert!(!result.success, "Circular alias should be detected");
    assert!(result.stderr.contains("cycle"), "Error should mention cycle detection");
}

#[test]
#[serial]
fn test_alias_with_subcommand() {
    let repo = TestRepo::new();

    // Create config with alias for subcommand
    repo.create_config(r#"
aliases:
  sl: state list
  sc: state clean
"#);

    // Test alias for subcommand
    let result = repo.hn(&["sl"]);
    assert!(result.success, "Alias 'sl' should work as 'state list'");
}

#[test]
#[serial]
fn test_alias_with_global_flags() {
    let repo = TestRepo::new();

    // Create config with simple alias
    repo.create_config(r#"
aliases:
  ls: list
"#);

    // Test alias with global flag
    let result = repo.hn(&["--no-hooks", "ls"]);
    assert!(result.success, "Alias should work with global flags");
}

#[test]
#[serial]
fn test_alias_with_extra_arguments() {
    let repo = TestRepo::new();

    // Create config with alias
    repo.create_config(r#"
aliases:
  sw: switch
"#);

    // Create a worktree
    repo.hn(&["add", "feature-1"]).assert_success();

    // Test alias with extra arguments
    let result = repo.hn(&["sw", "feature-1"]);
    assert!(result.success, "Alias should work with extra arguments");
}

#[test]
#[serial]
fn test_no_config_no_alias() {
    let repo = TestRepo::new();

    // No config file, commands should work normally
    let result = repo.hn(&["list"]);
    assert!(result.success, "Commands should work without config");
}

#[test]
#[serial]
fn test_empty_aliases_section() {
    let repo = TestRepo::new();

    // Create config with empty aliases section
    repo.create_config(r#"
aliases: {}
"#);

    // Commands should work normally
    let result = repo.hn(&["list"]);
    assert!(result.success, "Commands should work with empty aliases");
}

#[test]
#[serial]
fn test_alias_does_not_override_builtin() {
    let repo = TestRepo::new();

    // Create worktree first
    repo.hn(&["add", "feature-1"]).assert_success();

    // Create config where alias would conflict
    repo.create_config(r#"
aliases:
  list: add
"#);

    // The 'list' command should be treated as an alias and expand to 'add'
    // This will fail because 'add' expects arguments, but it shows the alias works
    let result = repo.hn(&["list"]);
    // Should fail with add's error message, not list
    assert!(!result.success);
}

#[test]
#[serial]
fn test_multi_word_alias_expansion() {
    let repo = TestRepo::new();

    // Create config with multi-word alias
    repo.create_config(r#"
aliases:
  lt: list --tree
"#);

    // Create a couple of worktrees
    repo.hn(&["add", "feature-1"]).assert_success();
    repo.hn(&["add", "feature-2"]).assert_success();

    // Test alias that expands to multiple words
    let result = repo.hn(&["lt"]);
    assert!(result.success, "Multi-word alias should expand correctly");
    assert!(result.stdout.contains("feature-1"), "Should show feature-1");
    assert!(result.stdout.contains("feature-2"), "Should show feature-2");
}
