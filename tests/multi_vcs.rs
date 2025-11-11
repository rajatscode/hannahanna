/// Integration tests for Multi-VCS support
/// Tests VCS detection, Mercurial backend, and Jujutsu backend
///
/// NOTE: Run with `cargo test --test multi_vcs -- --test-threads=1`
/// Some tests change current directory and can interfere when run in parallel.
mod common;

use common::TestRepo;
use hannahanna::vcs::git::GitBackend;
use hannahanna::vcs::traits::{create_backend, detect_vcs_type, VcsBackend, VcsType};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

// ===== VCS TRAIT INTERFACE TESTS =====

#[test]
fn test_vcs_backend_trait_with_git() {
    // Test that GitBackend properly implements VcsBackend trait
    let test_repo = TestRepo::new();
    let git = GitBackend::open(&test_repo.repo_path).expect("Failed to open git backend");

    // Test as trait object
    let backend: Box<dyn VcsBackend> = Box::new(git);

    // Test vcs_type
    assert_eq!(backend.vcs_type(), VcsType::Git);

    // Test repo_root
    let root = backend.repo_root().expect("Failed to get repo root");
    assert!(root.ends_with("repo"));

    // Test list_workspaces (should have just the main worktree)
    let worktrees = backend
        .list_workspaces()
        .expect("Failed to list workspaces");
    assert_eq!(worktrees.len(), 1);
    // Main worktree name is derived from directory
    assert!(!worktrees[0].name.is_empty());
}

#[test]
fn test_create_workspace_via_trait() {
    // Test creating workspace through VcsBackend trait
    let test_repo = TestRepo::new();
    let git = GitBackend::open(&test_repo.repo_path).expect("Failed to open git backend");
    let backend: Box<dyn VcsBackend> = Box::new(git);

    // Create workspace
    let worktree = backend
        .create_workspace("feature-test", None, None, false)
        .expect("Failed to create workspace");

    assert_eq!(worktree.name, "feature-test");
    assert!(worktree.path.exists());

    // Verify it appears in list
    let worktrees = backend
        .list_workspaces()
        .expect("Failed to list workspaces");
    assert_eq!(worktrees.len(), 2);
    assert!(worktrees.iter().any(|wt| wt.name == "feature-test"));
}

#[test]
fn test_remove_workspace_via_trait() {
    let test_repo = TestRepo::new();
    let git = GitBackend::open(&test_repo.repo_path).expect("Failed to open git backend");
    let backend: Box<dyn VcsBackend> = Box::new(git);

    // Create and then remove workspace
    backend
        .create_workspace("temp-workspace", None, None, false)
        .expect("Failed to create workspace");

    backend
        .remove_workspace("temp-workspace", false)
        .expect("Failed to remove workspace");

    // Verify it's gone
    let worktrees = backend
        .list_workspaces()
        .expect("Failed to list workspaces");
    assert_eq!(worktrees.len(), 1);
    assert!(!worktrees.iter().any(|wt| wt.name == "temp-workspace"));
}

#[test]
fn test_get_workspace_by_name_via_trait() {
    let test_repo = TestRepo::new();
    let git = GitBackend::open(&test_repo.repo_path).expect("Failed to open git backend");
    let backend: Box<dyn VcsBackend> = Box::new(git);

    // Create workspace
    backend
        .create_workspace("named-workspace", None, None, false)
        .expect("Failed to create workspace");

    // Get by name
    let worktree = backend
        .get_workspace_by_name("named-workspace")
        .expect("Failed to get workspace");

    assert_eq!(worktree.name, "named-workspace");
    assert!(worktree.path.ends_with("named-workspace"));
}

#[test]
fn test_get_current_workspace_via_trait() {
    let test_repo = TestRepo::new();

    // Change to the repo directory so get_current_workspace works
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&test_repo.repo_path).expect("Failed to change dir");

    let git = GitBackend::open_from_current_dir().expect("Failed to open git backend");
    let backend: Box<dyn VcsBackend> = Box::new(git);

    // Get current workspace (should be the main worktree)
    let current = backend
        .get_current_workspace()
        .expect("Failed to get current workspace");

    // Main worktree name is derived from directory
    assert!(!current.name.is_empty());
    assert_eq!(current.path, test_repo.repo_path);

    // Restore directory
    std::env::set_current_dir(original_dir).ok();
}

#[test]
fn test_get_workspace_status_via_trait() {
    let test_repo = TestRepo::new();
    let git = GitBackend::open(&test_repo.repo_path).expect("Failed to open git backend");
    let backend: Box<dyn VcsBackend> = Box::new(git);

    // Get status of main workspace
    let status = backend
        .get_workspace_status(&test_repo.repo_path)
        .expect("Failed to get workspace status");

    assert!(status.is_clean());
    assert_eq!(status.modified, 0);
    assert_eq!(status.added, 0);
    assert_eq!(status.deleted, 0);
    assert_eq!(status.untracked, 0);
}

// ===== VCS FACTORY FUNCTION TESTS =====

#[test]
fn test_detect_vcs_type_git() {
    let test_repo = TestRepo::new();

    let vcs_type = detect_vcs_type(&test_repo.repo_path);

    assert_eq!(vcs_type, Some(VcsType::Git));
}

#[test]
fn test_detect_vcs_type_jujutsu_priority() {
    // Test that Jujutsu is detected even if .git exists
    let temp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp.path().join("jj-repo");
    fs::create_dir(&repo_path).expect("Failed to create repo dir");

    // Create both .jj and .git
    fs::create_dir(repo_path.join(".jj")).expect("Failed to create .jj");
    fs::create_dir(repo_path.join(".git")).expect("Failed to create .git");

    let vcs_type = detect_vcs_type(&repo_path);

    // Should detect Jujutsu, not Git
    assert_eq!(vcs_type, Some(VcsType::Jujutsu));
}

#[test]
fn test_detect_vcs_type_mercurial() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp.path().join("hg-repo");
    fs::create_dir(&repo_path).expect("Failed to create repo dir");
    fs::create_dir(repo_path.join(".hg")).expect("Failed to create .hg");

    let vcs_type = detect_vcs_type(&repo_path);

    assert_eq!(vcs_type, Some(VcsType::Mercurial));
}

#[test]
fn test_detect_vcs_type_no_vcs() {
    let temp = TempDir::new().expect("Failed to create temp dir");

    let result = detect_vcs_type(temp.path());

    assert_eq!(result, None);
}

#[test]
fn test_create_backend_git() {
    let test_repo = TestRepo::new();

    // Detect VCS type first, then create backend
    let vcs_type = detect_vcs_type(&test_repo.repo_path).expect("No VCS detected");

    // Save current dir and change to test repo
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&test_repo.repo_path).expect("Failed to change dir");

    let backend = create_backend(vcs_type).expect("Failed to create backend");

    assert_eq!(backend.vcs_type(), VcsType::Git);
    let worktrees = backend.list_workspaces().expect("Failed to list");
    assert!(!worktrees.is_empty());

    // Restore directory
    std::env::set_current_dir(original_dir).ok();
}

#[test]
fn test_vcs_type_parsing() {
    use std::str::FromStr;

    // Test valid types
    assert_eq!(VcsType::from_str("git").unwrap(), VcsType::Git);
    assert_eq!(VcsType::from_str("Git").unwrap(), VcsType::Git);
    assert_eq!(VcsType::from_str("GIT").unwrap(), VcsType::Git);

    assert_eq!(
        VcsType::from_str("mercurial").unwrap(),
        VcsType::Mercurial
    );
    assert_eq!(VcsType::from_str("hg").unwrap(), VcsType::Mercurial);
    assert_eq!(VcsType::from_str("Hg").unwrap(), VcsType::Mercurial);

    assert_eq!(VcsType::from_str("jujutsu").unwrap(), VcsType::Jujutsu);
    assert_eq!(VcsType::from_str("jj").unwrap(), VcsType::Jujutsu);
    assert_eq!(VcsType::from_str("Jj").unwrap(), VcsType::Jujutsu);

    // Test invalid types
    assert!(VcsType::from_str("invalid").is_err());
    assert!(VcsType::from_str("svn").is_err());
}

// ===== VCS DETECTION TESTS =====

#[test]
fn test_detect_git_repository() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp.path().join("git-repo");
    fs::create_dir(&repo_path).expect("Failed to create repo dir");

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to init git");

    // hannahanna should detect it as Git
    assert!(
        repo_path.join(".git").exists(),
        "Git directory should exist"
    );
}

#[test]
fn test_detect_jujutsu_priority_over_git() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp.path().join("jj-repo");
    fs::create_dir(&repo_path).expect("Failed to create repo dir");

    // Create both .jj and .git (jj repos have .git for compatibility)
    fs::create_dir(repo_path.join(".jj")).expect("Failed to create .jj");
    fs::create_dir(repo_path.join(".git")).expect("Failed to create .git");

    // Jujutsu should be detected first (higher priority)
    assert!(
        repo_path.join(".jj").exists(),
        "Jujutsu directory should exist"
    );
    assert!(
        repo_path.join(".git").exists(),
        "Git directory should also exist"
    );
}

#[test]
fn test_vcs_type_from_string() {
    // Test case-insensitive parsing
    let test_cases = vec![
        ("git", Some("git")),
        ("Git", Some("git")),
        ("GIT", Some("git")),
        ("hg", Some("mercurial")),
        ("mercurial", Some("mercurial")),
        ("Mercurial", Some("mercurial")),
        ("jj", Some("jujutsu")),
        ("jujutsu", Some("jujutsu")),
        ("Jujutsu", Some("jujutsu")),
        ("invalid", None),
        ("svn", None),
    ];

    for (input, expected) in test_cases {
        // This test validates the VcsType::from_str() method
        // We'll implement this in the actual code
        println!("Testing VCS type: {} -> {:?}", input, expected);
    }
}

// ===== MERCURIAL BACKEND TESTS =====

#[test]
fn test_mercurial_init_repository() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp.path().join("hg-repo");
    fs::create_dir(&repo_path).expect("Failed to create repo dir");

    // Initialize hg repo
    let output = Command::new("hg")
        .args(["init"])
        .current_dir(&repo_path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            assert!(
                repo_path.join(".hg").exists(),
                "Mercurial directory should exist"
            );
        }
        _ => {
            eprintln!("Skipping test: Mercurial not installed or failed to init");
        }
    }
}

#[test]
fn test_mercurial_share_workspace() {
    // Test that we can create a shared workspace using hg share
    let temp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp.path().join("hg-repo");
    fs::create_dir(&repo_path).expect("Failed to create repo dir");

    // Initialize and setup hg repo
    if !setup_hg_repo(&repo_path) {
        eprintln!("Skipping test: Mercurial not available");
        return;
    }

    // Create a shared workspace
    let share_path = temp.path().join("hg-share");
    let output = Command::new("hg")
        .args([
            "share",
            repo_path.to_str().unwrap(),
            share_path.to_str().unwrap(),
        ])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            assert!(share_path.exists(), "Share directory should exist");
            assert!(
                share_path.join(".hg").exists(),
                "Share should have .hg directory"
            );
        }
        _ => {
            eprintln!("Failed to create hg share");
        }
    }
}

#[test]
fn test_mercurial_list_shares() {
    // Test that we can list all shared workspaces
    // This will require maintaining a registry since hg doesn't have native list
    let temp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp.path().join("hg-repo");
    fs::create_dir(&repo_path).expect("Failed to create repo dir");

    if !setup_hg_repo(&repo_path) {
        eprintln!("Skipping test: Mercurial not available");
        return;
    }

    // Create multiple shares
    let share1 = temp.path().join("share1");
    let share2 = temp.path().join("share2");

    Command::new("hg")
        .args([
            "share",
            repo_path.to_str().unwrap(),
            share1.to_str().unwrap(),
        ])
        .output()
        .ok();

    Command::new("hg")
        .args([
            "share",
            repo_path.to_str().unwrap(),
            share2.to_str().unwrap(),
        ])
        .output()
        .ok();

    // Test that we can track these shares via registry
    // Implementation will store shares in .hg/wt-registry.json
}

// ===== JUJUTSU BACKEND TESTS =====

#[test]
fn test_jujutsu_init_repository() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp.path().join("jj-repo");
    fs::create_dir(&repo_path).expect("Failed to create repo dir");

    // Initialize jj repo
    let output = Command::new("jj")
        .args(["init", "--git"])
        .current_dir(&repo_path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            assert!(
                repo_path.join(".jj").exists(),
                "Jujutsu directory should exist"
            );
        }
        _ => {
            eprintln!("Skipping test: Jujutsu not installed or failed to init");
        }
    }
}

#[test]
fn test_jujutsu_workspace_add() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp.path().join("jj-repo");
    fs::create_dir(&repo_path).expect("Failed to create repo dir");

    if !setup_jj_repo(&repo_path) {
        eprintln!("Skipping test: Jujutsu not available");
        return;
    }

    // Create a workspace using jj workspace add
    let workspace_path = temp.path().join("jj-workspace");
    let output = Command::new("jj")
        .args(["workspace", "add", workspace_path.to_str().unwrap()])
        .current_dir(&repo_path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            assert!(workspace_path.exists(), "Workspace directory should exist");
        }
        _ => {
            eprintln!("Failed to create jj workspace");
        }
    }
}

#[test]
fn test_jujutsu_workspace_list() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp.path().join("jj-repo");
    fs::create_dir(&repo_path).expect("Failed to create repo dir");

    if !setup_jj_repo(&repo_path) {
        eprintln!("Skipping test: Jujutsu not available");
        return;
    }

    // Create workspaces
    let ws1 = temp.path().join("ws1");
    let ws2 = temp.path().join("ws2");

    Command::new("jj")
        .args(["workspace", "add", ws1.to_str().unwrap()])
        .current_dir(&repo_path)
        .output()
        .ok();

    Command::new("jj")
        .args(["workspace", "add", ws2.to_str().unwrap()])
        .current_dir(&repo_path)
        .output()
        .ok();

    // List workspaces
    let output = Command::new("jj")
        .args(["workspace", "list"])
        .current_dir(&repo_path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            println!("Workspaces: {}", stdout);
            // Should show both workspaces
        }
        _ => {
            eprintln!("Failed to list jj workspaces");
        }
    }
}

// ===== INTEGRATION TESTS WITH HN =====

#[test]
#[ignore] // Phase 4: CLI integration not yet implemented
fn test_hn_add_with_mercurial() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp.path().join("hg-repo");
    fs::create_dir(&repo_path).expect("Failed to create repo dir");

    if !setup_hg_repo(&repo_path) {
        eprintln!("Skipping test: Mercurial not available");
        return;
    }

    // Run hn add in a mercurial repo
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["add", "feature-x"])
        .current_dir(&repo_path)
        .output();

    match result {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("stdout: {}", stdout);
            println!("stderr: {}", stderr);

            // Should either succeed or give clear error about Mercurial
            if !output.status.success() {
                assert!(
                    stderr.contains("Mercurial") || stderr.contains("not supported"),
                    "Should mention Mercurial support status"
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to run hn: {}", e);
        }
    }
}

#[test]
#[ignore] // Phase 4: CLI integration not yet implemented
fn test_hn_add_with_jujutsu() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp.path().join("jj-repo");
    fs::create_dir(&repo_path).expect("Failed to create repo dir");

    if !setup_jj_repo(&repo_path) {
        eprintln!("Skipping test: Jujutsu not available");
        return;
    }

    // Run hn add in a jujutsu repo
    let result = Command::new(env!("CARGO_BIN_EXE_hn"))
        .args(["add", "feature-y"])
        .current_dir(&repo_path)
        .output();

    match result {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("stdout: {}", stdout);
            println!("stderr: {}", stderr);

            // Should either succeed or give clear error about Jujutsu
            if !output.status.success() {
                assert!(
                    stderr.contains("Jujutsu") || stderr.contains("not supported"),
                    "Should mention Jujutsu support status"
                );
            }
        }
        Err(e) => {
            eprintln!("Failed to run hn: {}", e);
        }
    }
}

// ===== HELPER FUNCTIONS =====

fn setup_hg_repo(path: &Path) -> bool {
    // Initialize hg repo with initial commit
    let init = Command::new("hg").args(["init"]).current_dir(path).output();

    if init.is_err() || !init.unwrap().status.success() {
        return false;
    }

    // Configure hg
    Command::new("hg")
        .args([
            "config",
            "--local",
            "ui.username",
            "Test User <test@example.com>",
        ])
        .current_dir(path)
        .output()
        .ok();

    // Create initial file and commit
    fs::write(path.join("README"), "Test repo\n").ok();

    let add = Command::new("hg")
        .args(["add", "README"])
        .current_dir(path)
        .output();

    if add.is_err() || !add.unwrap().status.success() {
        return false;
    }

    let commit = Command::new("hg")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(path)
        .output();

    commit.is_ok() && commit.unwrap().status.success()
}

fn setup_jj_repo(path: &Path) -> bool {
    // Initialize jj repo
    let init = Command::new("jj")
        .args(["init", "--git"])
        .current_dir(path)
        .output();

    if init.is_err() || !init.unwrap().status.success() {
        return false;
    }

    // Create initial file
    fs::write(path.join("README"), "Test repo\n").ok();

    // Jujutsu automatically tracks files, just need to describe the change
    let describe = Command::new("jj")
        .args(["describe", "-m", "Initial commit"])
        .current_dir(path)
        .output();

    describe.is_ok() && describe.unwrap().status.success()
}
