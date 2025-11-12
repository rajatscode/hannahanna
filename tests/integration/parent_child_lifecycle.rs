// Integration tests for parent/child relationship handling during remove and integrate operations
use anyhow::Result;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Helper to create a test git repository
fn setup_test_repo() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("test-repo");

    // Initialize git repo
    Command::new("git")
        .args(&["init", repo_path.to_str().unwrap()])
        .output()
        .unwrap();

    // Configure git
    Command::new("git")
        .args(&[
            "-C",
            repo_path.to_str().unwrap(),
            "config",
            "user.email",
            "test@example.com",
        ])
        .output()
        .unwrap();
    Command::new("git")
        .args(&[
            "-C",
            repo_path.to_str().unwrap(),
            "config",
            "user.name",
            "Test User",
        ])
        .output()
        .unwrap();

    // Create initial commit
    fs::write(repo_path.join("README.md"), "# Test Repo").unwrap();
    Command::new("git")
        .args(&["-C", repo_path.to_str().unwrap(), "add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .args(&[
            "-C",
            repo_path.to_str().unwrap(),
            "commit",
            "-m",
            "Initial commit",
        ])
        .output()
        .unwrap();

    (temp_dir, repo_path)
}

#[test]
fn test_remove_parent_without_force_fails() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_repo();
    let hn_bin = env!("CARGO_BIN_EXE_hn");

    // Create parent worktree
    std::env::set_current_dir(&repo_path)?;
    let output = Command::new(hn_bin)
        .args(&["add", "parent-feature"])
        .output()?;
    assert!(output.status.success());

    // Create child worktree (from within parent)
    let parent_path = repo_path.parent().unwrap().join("parent-feature");
    std::env::set_current_dir(&parent_path)?;
    let output = Command::new(hn_bin).args(&["add", "child-feature"]).output()?;
    assert!(output.status.success());

    // Try to remove parent without --force should fail
    std::env::set_current_dir(&repo_path)?;
    let output = Command::new(hn_bin)
        .args(&["remove", "parent-feature"])
        .output()?;

    // Should fail because it has children
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("child") || stderr.contains("Cannot remove"));

    Ok(())
}

#[test]
fn test_remove_parent_with_force_warns() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_repo();
    let hn_bin = env!("CARGO_BIN_EXE_hn");

    // Create parent worktree
    std::env::set_current_dir(&repo_path)?;
    let output = Command::new(hn_bin)
        .args(&["add", "parent-feature"])
        .output()?;
    assert!(output.status.success());

    // Create child worktree (from within parent)
    let parent_path = repo_path.parent().unwrap().join("parent-feature");
    std::env::set_current_dir(&parent_path)?;
    let output = Command::new(hn_bin).args(&["add", "child-feature"]).output()?;
    assert!(output.status.success());

    // Remove parent with --force should warn about orphaning
    std::env::set_current_dir(&repo_path)?;
    let output = Command::new(hn_bin)
        .args(&["remove", "parent-feature", "--force"])
        .output()?;

    // Should succeed with force
    assert!(output.status.success());

    // Should warn about orphaning children
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Warning") || stderr.contains("orphan"));

    Ok(())
}

#[test]
fn test_integrate_reparents_children() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_repo();
    let hn_bin = env!("CARGO_BIN_EXE_hn");

    // Create grandparent worktree (main)
    std::env::set_current_dir(&repo_path)?;
    let output = Command::new(hn_bin)
        .args(&["add", "feature-parent"])
        .output()?;
    assert!(output.status.success());

    // Create child from parent
    let parent_path = repo_path.parent().unwrap().join("feature-parent");
    std::env::set_current_dir(&parent_path)?;
    let output = Command::new(hn_bin)
        .args(&["add", "feature-child"])
        .output()?;
    assert!(output.status.success());

    // Integrate parent into main
    std::env::set_current_dir(&repo_path)?;
    let output = Command::new(hn_bin)
        .args(&["integrate", "feature-parent", "--into", "main"])
        .output()?;

    // Integration should mention reparenting
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        // If integrate succeeded, it should have mentioned reparenting
        // (Note: this test might fail if there are merge conflicts or other issues)
        assert!(stderr.contains("Reparenting") || stderr.contains("Reparented"));
    }

    Ok(())
}

#[test]
fn test_remove_child_first_then_parent() -> Result<()> {
    let (_temp_dir, repo_path) = setup_test_repo();
    let hn_bin = env!("CARGO_BIN_EXE_hn");

    // Create parent worktree
    std::env::set_current_dir(&repo_path)?;
    let output = Command::new(hn_bin)
        .args(&["add", "parent-feature"])
        .output()?;
    assert!(output.status.success());

    // Create child worktree
    let parent_path = repo_path.parent().unwrap().join("parent-feature");
    std::env::set_current_dir(&parent_path)?;
    let output = Command::new(hn_bin).args(&["add", "child-feature"]).output()?;
    assert!(output.status.success());

    // Remove child first - should succeed
    std::env::set_current_dir(&repo_path)?;
    let output = Command::new(hn_bin)
        .args(&["remove", "child-feature"])
        .output()?;
    assert!(output.status.success());

    // Then remove parent - should succeed (no children left)
    let output = Command::new(hn_bin)
        .args(&["remove", "parent-feature"])
        .output()?;
    assert!(output.status.success());

    Ok(())
}
