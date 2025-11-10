/// Common test utilities for hannahanna integration tests
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// A test repository with temporary directory management
#[allow(dead_code)]
pub struct TestRepo {
    pub temp_dir: TempDir,
    pub repo_path: PathBuf,
}

impl TestRepo {
    /// Create a new test repository with git initialized
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        // Create a subdirectory for the actual repo, so worktrees can be siblings
        let repo_path = temp_dir.path().join("repo");
        std::fs::create_dir(&repo_path).expect("Failed to create repo directory");

        // Initialize git repository
        Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to init git repo");

        // Configure git
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to configure git");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to configure git");

        // Disable GPG signing for tests
        Command::new("git")
            .args(["config", "commit.gpgsign", "false"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to configure git");

        // Create initial commit
        std::fs::write(repo_path.join("README.md"), "# Test Repo\n")
            .expect("Failed to write README");

        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to add files");

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to create initial commit");

        // Ensure we're on main branch (git init might create master or main depending on config)
        Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to rename branch to main");

        TestRepo {
            temp_dir,
            repo_path,
        }
    }

    /// Get the repository path
    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.repo_path
    }

    /// Run hn command in this repository
    pub fn hn(&self, args: &[&str]) -> CommandResult {
        let output = Command::new(env!("CARGO_BIN_EXE_hn"))
            .args(args)
            .current_dir(&self.repo_path)
            .output()
            .expect("Failed to execute hn command");

        CommandResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
            exit_code: output.status.code(),
        }
    }

    /// Check if a worktree exists
    #[allow(dead_code)]
    pub fn worktree_exists(&self, name: &str) -> bool {
        self.repo_path.parent().unwrap().join(name).exists()
    }

    /// Get worktree path
    #[allow(dead_code)]
    pub fn worktree_path(&self, name: &str) -> PathBuf {
        self.repo_path.parent().unwrap().join(name)
    }

    /// Check if state directory exists
    #[allow(dead_code)]
    pub fn state_exists(&self, name: &str) -> bool {
        self.repo_path.join(".wt-state").join(name).exists()
    }

    /// Create a config file
    #[allow(dead_code)]
    pub fn create_config(&self, content: &str) {
        std::fs::write(self.repo_path.join(".hannahanna.yml"), content)
            .expect("Failed to write config file");
    }

    /// Create a file and commit it
    #[allow(dead_code)]
    pub fn create_and_commit(&self, filename: &str, content: &str, message: &str) {
        std::fs::write(self.repo_path.join(filename), content).expect("Failed to write file");

        Command::new("git")
            .args(["add", filename])
            .current_dir(&self.repo_path)
            .output()
            .expect("Failed to add file");

        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.repo_path)
            .output()
            .expect("Failed to commit");
    }
}

/// Result of running a command
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
    pub exit_code: Option<i32>,
}

impl CommandResult {
    /// Assert the command succeeded
    pub fn assert_success(&self) {
        if !self.success {
            panic!(
                "Command failed:\nstdout: {}\nstderr: {}\nexit code: {:?}",
                self.stdout, self.stderr, self.exit_code
            );
        }
    }

    /// Assert the command failed
    #[allow(dead_code)]
    pub fn assert_failure(&self) {
        if self.success {
            panic!(
                "Command succeeded when it should have failed:\nstdout: {}\nstderr: {}",
                self.stdout, self.stderr
            );
        }
    }

    /// Assert stdout contains text
    #[allow(dead_code)]
    pub fn assert_stdout_contains(&self, text: &str) {
        assert!(
            self.stdout.contains(text),
            "stdout does not contain '{}'\nstdout: {}",
            text,
            self.stdout
        );
    }

    /// Assert stderr contains text
    #[allow(dead_code)]
    pub fn assert_stderr_contains(&self, text: &str) {
        assert!(
            self.stderr.contains(text),
            "stderr does not contain '{}'\nstderr: {}",
            text,
            self.stderr
        );
    }
}
