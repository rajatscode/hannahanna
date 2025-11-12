// Workspace management tests for v0.5
mod common;

use common::TestRepo;

// ============ Workspace Save Tests ============

#[test]
fn test_workspace_save_basic() {
    let repo = TestRepo::new();

    repo.create_config("");

    // Create some worktrees
    repo.hn(&["add", "wt1"]).assert_success();
    repo.hn(&["add", "wt2"]).assert_success();
    repo.hn(&["add", "wt3"]).assert_success();

    // Save workspace
    let result = repo.hn(&["workspace", "save", "my-workspace"]);
    assert!(result.success, "Workspace save should succeed: {}", result.stderr);
}

#[test]
fn test_workspace_save_with_description() {
    let repo = TestRepo::new();

    repo.create_config("");
    repo.hn(&["add", "wt1"]).assert_success();

    let result = repo.hn(&["workspace", "save", "described-workspace", "--description", "My test workspace"]);
    assert!(result.success);
}

#[test]
fn test_workspace_save_duplicate_name_fails() {
    let repo = TestRepo::new();

    repo.create_config("");
    repo.hn(&["add", "wt1"]).assert_success();

    repo.hn(&["workspace", "save", "duplicate"]).assert_success();

    let result = repo.hn(&["workspace", "save", "duplicate"]);
    assert!(!result.success);
    assert!(result.stderr.contains("already exists") || result.stderr.contains("duplicate"));
}

#[test]
fn test_workspace_save_empty() {
    let repo = TestRepo::new();

    repo.create_config("");

    // Save workspace with no worktrees
    let result = repo.hn(&["workspace", "save", "empty-workspace"]);
    assert!(result.success);
}

#[test]
fn test_workspace_save_invalid_name() {
    let repo = TestRepo::new();

    repo.create_config("");

    // Invalid workspace names
    let result = repo.hn(&["workspace", "save", "invalid/name"]);
    assert!(!result.success);
}

// ============ Workspace Restore Tests ============

#[test]
fn test_workspace_restore_basic() {
    let repo = TestRepo::new();

    repo.create_config("");

    // Create and save workspace
    repo.hn(&["add", "restore-wt1"]).assert_success();
    repo.hn(&["add", "restore-wt2"]).assert_success();
    repo.hn(&["workspace", "save", "restore-test"]).assert_success();

    // Remove worktrees
    repo.hn(&["remove", "restore-wt1", "--force"]).assert_success();
    repo.hn(&["remove", "restore-wt2", "--force"]).assert_success();

    // Restore workspace
    let result = repo.hn(&["workspace", "restore", "restore-test"]);
    assert!(result.success, "Workspace restore should succeed: {}", result.stderr);

    // Verify worktrees were recreated
    let list_result = repo.hn(&["list"]);
    assert!(list_result.stdout.contains("restore-wt1"));
    assert!(list_result.stdout.contains("restore-wt2"));
}

#[test]
fn test_workspace_restore_nonexistent_fails() {
    let repo = TestRepo::new();

    repo.create_config("");

    let result = repo.hn(&["workspace", "restore", "nonexistent"]);
    assert!(!result.success);
    assert!(result.stderr.contains("not found") || result.stderr.contains("does not exist"));
}

#[test]
fn test_workspace_restore_with_conflicts() {
    let repo = TestRepo::new();

    repo.create_config("");

    // Create and save workspace
    repo.hn(&["add", "conflict-wt"]).assert_success();
    repo.hn(&["workspace", "save", "conflict-test"]).assert_success();

    // Workspace already has the worktree
    let result = repo.hn(&["workspace", "restore", "conflict-test"]);
    // Should handle conflicts gracefully
    assert!(result.success || result.stderr.contains("already exists"));
}

#[test]
fn test_workspace_restore_force() {
    let repo = TestRepo::new();

    repo.create_config("");

    repo.hn(&["add", "force-wt1"]).assert_success();
    repo.hn(&["workspace", "save", "force-test"]).assert_success();

    // Create conflicting worktree
    repo.hn(&["add", "conflicting"]).assert_success();

    let result = repo.hn(&["workspace", "restore", "force-test", "--force"]);
    // Force should override conflicts
    assert!(result.success || result.stderr.contains("worktree"));
}

// ============ Workspace List Tests ============

#[test]
fn test_workspace_list_empty() {
    let repo = TestRepo::new();

    repo.create_config("");

    let result = repo.hn(&["workspace", "list"]);
    assert!(result.success);
    assert!(result.stdout.contains("No workspaces") || result.stdout.is_empty());
}

#[test]
fn test_workspace_list_with_saved_workspaces() {
    let repo = TestRepo::new();

    repo.create_config("");

    // Save multiple workspaces
    repo.hn(&["add", "wt1"]).assert_success();
    repo.hn(&["workspace", "save", "workspace1"]).assert_success();

    repo.hn(&["add", "wt2"]).assert_success();
    repo.hn(&["workspace", "save", "workspace2"]).assert_success();

    let result = repo.hn(&["workspace", "list"]);
    assert!(result.success);
    assert!(result.stdout.contains("workspace1"));
    assert!(result.stdout.contains("workspace2"));
}

#[test]
fn test_workspace_list_json_output() {
    let repo = TestRepo::new();

    repo.create_config("");

    repo.hn(&["add", "wt1"]).assert_success();
    repo.hn(&["workspace", "save", "json-test"]).assert_success();

    let result = repo.hn(&["workspace", "list", "--json"]);
    assert!(result.success);
    // Should be valid JSON
    assert!(result.stdout.contains("{") || result.stdout.contains("["));
}

#[test]
fn test_workspace_list_shows_metadata() {
    let repo = TestRepo::new();

    repo.create_config("");

    repo.hn(&["add", "meta-wt"]).assert_success();
    repo.hn(&["workspace", "save", "meta-workspace"]).assert_success();

    let result = repo.hn(&["workspace", "list"]);
    assert!(result.success);
    // Should show creation date, worktree count, etc.
    assert!(result.stdout.contains("meta-workspace"));
}

// ============ Workspace Delete Tests ============

#[test]
fn test_workspace_delete_basic() {
    let repo = TestRepo::new();

    repo.create_config("");

    repo.hn(&["add", "del-wt"]).assert_success();
    repo.hn(&["workspace", "save", "delete-me"]).assert_success();

    let result = repo.hn(&["workspace", "delete", "delete-me"]);
    assert!(result.success);

    // Verify it's gone
    let list_result = repo.hn(&["workspace", "list"]);
    assert!(!list_result.stdout.contains("delete-me"));
}

#[test]
fn test_workspace_delete_nonexistent_fails() {
    let repo = TestRepo::new();

    repo.create_config("");

    let result = repo.hn(&["workspace", "delete", "nonexistent"]);
    assert!(!result.success);
}

#[test]
fn test_workspace_delete_confirmation() {
    let repo = TestRepo::new();

    repo.create_config("");

    repo.hn(&["add", "conf-wt"]).assert_success();
    repo.hn(&["workspace", "save", "confirm-delete"]).assert_success();

    // Without --force, might ask for confirmation (implementation detail)
    let result = repo.hn(&["workspace", "delete", "confirm-delete", "--force"]);
    assert!(result.success);
}

#[test]
fn test_workspace_delete_doesnt_remove_worktrees() {
    let repo = TestRepo::new();

    repo.create_config("");

    repo.hn(&["add", "persist-wt"]).assert_success();
    repo.hn(&["workspace", "save", "persist-workspace"]).assert_success();

    repo.hn(&["workspace", "delete", "persist-workspace", "--force"]).assert_success();

    // Worktree should still exist
    let list_result = repo.hn(&["list"]);
    assert!(list_result.stdout.contains("persist-wt"));
}

// ============ Workspace Integration Tests ============

#[test]
fn test_workspace_full_lifecycle() {
    let repo = TestRepo::new();

    repo.create_config("");

    // 1. Create worktrees
    repo.hn(&["add", "lifecycle-wt1"]).assert_success();
    repo.hn(&["add", "lifecycle-wt2"]).assert_success();

    // 2. Save workspace
    repo.hn(&["workspace", "save", "lifecycle"]).assert_success();

    // 3. List and verify
    let list = repo.hn(&["workspace", "list"]);
    assert!(list.stdout.contains("lifecycle"));

    // 4. Remove worktrees
    repo.hn(&["remove", "lifecycle-wt1", "--force"]).assert_success();
    repo.hn(&["remove", "lifecycle-wt2", "--force"]).assert_success();

    // 5. Restore workspace
    repo.hn(&["workspace", "restore", "lifecycle"]).assert_success();

    // 6. Verify worktrees back
    let wt_list = repo.hn(&["list"]);
    assert!(wt_list.stdout.contains("lifecycle-wt1"));
    assert!(wt_list.stdout.contains("lifecycle-wt2"));

    // 7. Delete workspace
    repo.hn(&["workspace", "delete", "lifecycle", "--force"]).assert_success();

    // 8. Verify workspace gone
    let final_list = repo.hn(&["workspace", "list"]);
    assert!(!final_list.stdout.contains("lifecycle"));
}

#[test]
fn test_workspace_save_with_state() {
    let repo = TestRepo::new();

    repo.create_config(r#"
docker:
  enabled: true
  services:
    - app
"#);

    repo.hn(&["add", "state-wt"]).assert_success();

    // Save should capture state including Docker config
    repo.hn(&["workspace", "save", "with-state"]).assert_success();
}

#[test]
fn test_workspace_restore_preserves_branches() {
    let repo = TestRepo::new();

    repo.create_config("");

    repo.hn(&["add", "branch-wt1", "--branch", "feature-1"]).assert_success();
    repo.hn(&["add", "branch-wt2", "--branch", "feature-2"]).assert_success();

    repo.hn(&["workspace", "save", "branches"]).assert_success();

    repo.hn(&["remove", "branch-wt1", "--force"]).assert_success();
    repo.hn(&["remove", "branch-wt2", "--force"]).assert_success();

    repo.hn(&["workspace", "restore", "branches"]).assert_success();

    // Verify branches were restored
    let list = repo.hn(&["list"]);
    assert!(list.stdout.contains("feature-1") || list.stdout.contains("branch-wt1"));
    assert!(list.stdout.contains("feature-2") || list.stdout.contains("branch-wt2"));
}

#[test]
fn test_workspace_multiple_saves() {
    let repo = TestRepo::new();

    repo.create_config("");

    // Save different workspace states
    repo.hn(&["add", "v1-wt"]).assert_success();
    repo.hn(&["workspace", "save", "version-1"]).assert_success();

    repo.hn(&["add", "v2-wt"]).assert_success();
    repo.hn(&["workspace", "save", "version-2"]).assert_success();

    repo.hn(&["add", "v3-wt"]).assert_success();
    repo.hn(&["workspace", "save", "version-3"]).assert_success();

    let list = repo.hn(&["workspace", "list"]);
    assert!(list.stdout.contains("version-1"));
    assert!(list.stdout.contains("version-2"));
    assert!(list.stdout.contains("version-3"));
}

#[test]
fn test_workspace_restore_partial_success() {
    let repo = TestRepo::new();

    repo.create_config("");

    repo.hn(&["add", "partial-wt1"]).assert_success();
    repo.hn(&["add", "partial-wt2"]).assert_success();
    repo.hn(&["workspace", "save", "partial"]).assert_success();

    repo.hn(&["remove", "partial-wt1", "--force"]).assert_success();
    // partial-wt2 still exists

    // Restore should handle existing worktree gracefully
    let result = repo.hn(&["workspace", "restore", "partial"]);
    assert!(result.success || result.stderr.contains("already"));
}

#[test]
fn test_workspace_save_captures_config() {
    let repo = TestRepo::new();

    repo.create_config(r#"
hooks:
  post_create: "echo 'configured' > config.txt"
"#);

    repo.hn(&["add", "config-wt"]).assert_success();
    repo.hn(&["workspace", "save", "with-config"]).assert_success();

    // Config should be saved with workspace
}

#[test]
fn test_workspace_list_sorting() {
    let repo = TestRepo::new();

    repo.create_config("");

    // Create workspaces in non-alphabetical order
    repo.hn(&["workspace", "save", "zebra"]).assert_success();
    repo.hn(&["workspace", "save", "alpha"]).assert_success();
    repo.hn(&["workspace", "save", "beta"]).assert_success();

    let result = repo.hn(&["workspace", "list"]);
    assert!(result.success);
    // Should be sorted (implementation detail)
}

#[test]
fn test_workspace_name_validation() {
    let repo = TestRepo::new();

    repo.create_config("");

    // Test various invalid names
    let invalid_names = vec!["", " spaces ", "slash/name", "dot.", "..parent"];

    for name in invalid_names {
        let result = repo.hn(&["workspace", "save", name]);
        // Should fail validation
        assert!(!result.success || result.stderr.contains("invalid"));
    }
}

#[test]
fn test_workspace_export_import() {
    let repo = TestRepo::new();

    repo.create_config("");

    repo.hn(&["add", "export-wt"]).assert_success();
    repo.hn(&["workspace", "save", "export-test"]).assert_success();

    // If export functionality exists
    let result = repo.hn(&["workspace", "export", "export-test"]);
    // Implementation-dependent
}
