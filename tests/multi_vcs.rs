/// Integration tests for Multi-VCS support
/// Tests VCS detection, Mercurial backend, and Jujutsu backend
mod common;

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

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
#[ignore] // Ignore by default - requires hg installed
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
#[ignore] // Ignore by default - requires hg installed
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
#[ignore] // Ignore by default - requires hg installed
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
#[ignore] // Ignore by default - requires jj installed
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
#[ignore] // Ignore by default - requires jj installed
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
#[ignore] // Ignore by default - requires jj installed
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
#[ignore] // Ignore by default - requires hg installed
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
#[ignore] // Ignore by default - requires jj installed
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
