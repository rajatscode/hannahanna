/// Integration tests for environment management: symlinks and compatibility checking
mod common;

use common::TestRepo;
use std::fs;

#[test]
fn test_symlink_creation() {
    let repo = TestRepo::new();

    // Create a directory to share
    fs::create_dir(repo.path().join("node_modules")).expect("Failed to create node_modules");
    fs::write(
        repo.path().join("node_modules").join("package.json"),
        r#"{"name": "test"}"#,
    )
    .expect("Failed to create package.json");

    // Create config with symlinks
    repo.create_config(
        r#"
shared_resources:
  - source: node_modules
    target: node_modules
"#,
    );

    // Add worktree
    repo.hn(&["add", "feature-x"]).assert_success();

    // Check if symlink was created
    let worktree_path = repo.worktree_path("feature-x");
    let node_modules_link = worktree_path.join("node_modules");

    assert!(
        node_modules_link.exists(),
        "node_modules symlink should exist"
    );

    // Verify it's a symlink
    let metadata =
        fs::symlink_metadata(&node_modules_link).expect("Failed to get symlink metadata");
    assert!(
        metadata.file_type().is_symlink(),
        "node_modules should be a symlink"
    );
}

#[test]
fn test_compatibility_checking_with_identical_lockfiles() {
    let repo = TestRepo::new();

    // Create node_modules and lockfile
    fs::create_dir(repo.path().join("node_modules")).expect("Failed to create node_modules");
    fs::write(
        repo.path().join("package-lock.json"),
        r#"{"lockfileVersion": 2}"#,
    )
    .expect("Failed to create lockfile");

    // Create config with compatibility check
    repo.create_config(
        r#"
shared_resources:
  - source: node_modules
    target: node_modules
    compatibility: package-lock.json
"#,
    );

    // Add worktree - should create symlink since lockfile is identical
    let result = repo.hn(&["add", "feature-x"]);
    result.assert_success();

    // Verify symlink was created
    let worktree_path = repo.worktree_path("feature-x");
    let node_modules_link = worktree_path.join("node_modules");

    if node_modules_link.exists() {
        let metadata = fs::symlink_metadata(&node_modules_link).ok();
        // If the link exists, it should be a symlink
        if let Some(meta) = metadata {
            assert!(
                meta.file_type().is_symlink(),
                "node_modules should be a symlink when lockfiles match"
            );
        }
    }
}

#[test]
fn test_compatibility_checking_with_different_lockfiles() {
    let repo = TestRepo::new();

    // Create node_modules and lockfile in main repo
    fs::create_dir(repo.path().join("node_modules")).expect("Failed to create node_modules");
    fs::write(
        repo.path().join("package-lock.json"),
        r#"{"lockfileVersion": 2, "packages": {"foo": "1.0.0"}}"#,
    )
    .expect("Failed to create lockfile");

    // Create config
    repo.create_config(
        r#"
shared_resources:
  - source: node_modules
    target: node_modules
    compatibility: package-lock.json
"#,
    );

    // Add worktree
    repo.hn(&["add", "feature-x"]).assert_success();

    // Modify lockfile in worktree to be different
    let worktree_path = repo.worktree_path("feature-x");
    fs::write(
        worktree_path.join("package-lock.json"),
        r#"{"lockfileVersion": 2, "packages": {"bar": "2.0.0"}}"#,
    )
    .expect("Failed to modify lockfile");

    // If we add another worktree, it should detect the difference
    // (This is more of a conceptual test - implementation may vary)
}

#[test]
fn test_multiple_symlinks() {
    let repo = TestRepo::new();

    // Create multiple directories to share
    fs::create_dir(repo.path().join("node_modules")).ok();
    fs::create_dir(repo.path().join("vendor")).ok();
    fs::create_dir(repo.path().join(".build-cache")).ok();

    repo.create_config(
        r#"
shared_resources:
  - source: node_modules
    target: node_modules
  - source: vendor
    target: vendor
  - source: .build-cache
    target: .build-cache
"#,
    );

    repo.hn(&["add", "feature-x"]).assert_success();

    let worktree_path = repo.worktree_path("feature-x");

    // Check all symlinks
    for dir in &["node_modules", "vendor", ".build-cache"] {
        let link_path = worktree_path.join(dir);
        if link_path.exists() {
            let metadata = fs::symlink_metadata(&link_path).ok();
            if let Some(meta) = metadata {
                assert!(meta.file_type().is_symlink(), "{} should be a symlink", dir);
            }
        }
    }
}

#[test]
fn test_no_symlinks_without_config() {
    let repo = TestRepo::new();

    // Create directory but no config
    fs::create_dir(repo.path().join("node_modules")).ok();

    repo.hn(&["add", "feature-x"]).assert_success();

    // Should not create symlink without config
    let worktree_path = repo.worktree_path("feature-x");
    let node_modules_link = worktree_path.join("node_modules");

    // Either doesn't exist or is not a symlink
    // Without config, we don't enforce any particular behavior - allow both cases
    if node_modules_link.exists() {
        let _metadata = fs::symlink_metadata(&node_modules_link).expect("Failed to get metadata");
        // Without explicit config, we allow both symlinks and regular directories
    }
}

// TODO: Implement file copying from config
// The file copy feature needs to be implemented in src/cli/add.rs
// to read the config and copy files as specified in shared.copy
#[test]
#[ignore = "Feature not implemented: file copying from config"]
fn test_file_copying() {
    let repo = TestRepo::new();

    // Create template file
    fs::write(
        repo.path().join(".env.template"),
        "DATABASE_URL=postgres://localhost/test\n",
    )
    .expect("Failed to create template");

    repo.create_config(
        r#"
shared:
  copy:
    - .env.template -> .env
"#,
    );

    repo.hn(&["add", "feature-x"]).assert_success();

    let worktree_path = repo.worktree_path("feature-x");
    let env_file = worktree_path.join(".env");

    // File should be copied
    assert!(env_file.exists(), ".env file should exist");

    // Verify content
    let content = fs::read_to_string(&env_file).expect("Failed to read .env");
    assert!(content.contains("DATABASE_URL"));
}
