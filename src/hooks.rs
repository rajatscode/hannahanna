use crate::clock::{Clock, SystemClock};
use crate::config::HooksConfig;
use crate::errors::{HnError, Result};
use crate::vcs::Worktree;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub enum HookType {
    PostCreate,
    PreRemove,
}

impl HookType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookType::PostCreate => "post_create",
            HookType::PreRemove => "pre_remove",
        }
    }
}

pub struct HookExecutor {
    config: HooksConfig,
    skip_hooks: bool,
    clock: Arc<dyn Clock>,
}

impl HookExecutor {
    pub fn new(config: HooksConfig, skip_hooks: bool) -> Self {
        Self::new_with_clock(config, skip_hooks, Arc::new(SystemClock))
    }

    #[cfg(test)]
    pub fn new_with_clock(config: HooksConfig, skip_hooks: bool, clock: Arc<dyn Clock>) -> Self {
        Self {
            config,
            skip_hooks,
            clock,
        }
    }

    #[cfg(not(test))]
    pub fn new_with_clock(config: HooksConfig, skip_hooks: bool, clock: Arc<dyn Clock>) -> Self {
        Self {
            config,
            skip_hooks,
            clock,
        }
    }

    /// Execute a hook if it's configured
    pub fn run_hook(
        &self,
        hook_type: HookType,
        worktree: &Worktree,
        state_dir: &Path,
    ) -> Result<()> {
        // Skip hook execution if --no-hooks flag is set
        if self.skip_hooks {
            return Ok(());
        }

        let script = match hook_type {
            HookType::PostCreate => &self.config.post_create,
            HookType::PreRemove => &self.config.pre_remove,
        };

        if let Some(script) = script {
            self.execute_hook(hook_type, script, worktree, state_dir)?;
        }

        Ok(())
    }

    /// Execute the hook script with timeout
    fn execute_hook(
        &self,
        hook_type: HookType,
        script: &str,
        worktree: &Worktree,
        state_dir: &Path,
    ) -> Result<()> {
        use std::fs::File;
        use std::io::Read;

        // Build environment variables
        let env = self.build_env(worktree, state_dir);

        // Create temporary files for stdout/stderr to avoid pipe buffer deadlock
        // If hooks produce >64KB output, pipes will fill and cause deadlock
        let stdout_file = tempfile::NamedTempFile::new().map_err(|e| {
            HnError::HookError(format!("Failed to create temp file for stdout: {}", e))
        })?;
        let stderr_file = tempfile::NamedTempFile::new().map_err(|e| {
            HnError::HookError(format!("Failed to create temp file for stderr: {}", e))
        })?;

        // Spawn the command with output redirected to files
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(script)
            .current_dir(&worktree.path)
            .envs(env)
            .stdout(File::create(stdout_file.path())?)
            .stderr(File::create(stderr_file.path())?)
            .spawn()?;

        // Wait with timeout
        let timeout = Duration::from_secs(self.config.timeout_seconds);

        // Use platform-specific wait_timeout if available (Unix/Windows)
        #[cfg(unix)]
        {
            let wait_result = wait_with_timeout(&mut child, timeout, self.clock.clone())?;

            match wait_result {
                Some(status) => {
                    // Process completed - read output from temp files
                    let mut stdout = String::new();
                    let mut stderr = String::new();
                    File::open(stdout_file.path())?.read_to_string(&mut stdout)?;
                    File::open(stderr_file.path())?.read_to_string(&mut stderr)?;

                    if !status.success() {
                        return Err(HnError::HookError(format!(
                            "{} hook failed with exit code {}\nStdout: {}\nStderr: {}",
                            hook_type.as_str(),
                            status.code().unwrap_or(-1),
                            stdout,
                            stderr
                        )));
                    }
                }
                None => {
                    // Timeout occurred - kill process and read partial output
                    let kill_result = child.kill();
                    let wait_result = child.wait();

                    // Read whatever output was produced before timeout
                    let mut stdout = String::new();
                    let mut stderr = String::new();
                    let _ = File::open(stdout_file.path())?.read_to_string(&mut stdout);
                    let _ = File::open(stderr_file.path())?.read_to_string(&mut stderr);

                    // Check if process was already dead (race condition)
                    if let Err(e) = kill_result {
                        if e.kind() == std::io::ErrorKind::InvalidInput {
                            // Process already exited - check if it succeeded
                            if let Ok(status) = wait_result {
                                if status.success() {
                                    // Process completed successfully just before timeout
                                    return Ok(());
                                } else {
                                    return Err(HnError::HookError(format!(
                                        "{} hook failed with exit code {}\nStdout: {}\nStderr: {}",
                                        hook_type.as_str(),
                                        status.code().unwrap_or(-1),
                                        stdout,
                                        stderr
                                    )));
                                }
                            }
                        }
                    }

                    // Clean up zombie if kill succeeded
                    let _ = wait_result;

                    return Err(HnError::HookError(format!(
                        "{} hook timed out after {} seconds\nPartial stdout: {}\nPartial stderr: {}",
                        hook_type.as_str(),
                        self.config.timeout_seconds,
                        if stdout.len() > 500 { &stdout[..500] } else { &stdout },
                        if stderr.len() > 500 { &stderr[..500] } else { &stderr }
                    )));
                }
            }
        }

        #[cfg(not(unix))]
        {
            // For non-Unix systems, use a simple wait (no timeout for now)
            // TODO: Implement timeout for Windows using WaitForSingleObject
            let status = child.wait()?;

            // Read output from temp files
            let mut stdout = String::new();
            let mut stderr = String::new();
            File::open(stdout_file.path())?.read_to_string(&mut stdout)?;
            File::open(stderr_file.path())?.read_to_string(&mut stderr)?;

            if !status.success() {
                return Err(HnError::HookError(format!(
                    "{} hook failed with exit code {}\nStdout: {}\nStderr: {}",
                    hook_type.as_str(),
                    status.code().unwrap_or(-1),
                    stdout,
                    stderr
                )));
            }
        }

        Ok(())
    }

    /// Build environment variables for hook execution
    fn build_env(&self, worktree: &Worktree, state_dir: &Path) -> HashMap<String, String> {
        let mut env = HashMap::new();

        env.insert("WT_NAME".to_string(), worktree.name.clone());
        env.insert(
            "WT_PATH".to_string(),
            worktree.path.to_string_lossy().to_string(),
        );
        env.insert("WT_BRANCH".to_string(), worktree.branch.clone());
        env.insert("WT_COMMIT".to_string(), worktree.commit.clone());
        env.insert(
            "WT_STATE_DIR".to_string(),
            state_dir.to_string_lossy().to_string(),
        );

        env
    }
}

/// Helper function to wait for a child process with timeout
/// Uses simple polling with try_wait()
#[cfg(unix)]
fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
    clock: Arc<dyn Clock>,
) -> Result<Option<std::process::ExitStatus>> {
    let start = clock.now();
    let poll_interval = Duration::from_millis(100);

    loop {
        // Check if process has completed
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process completed
                return Ok(Some(status));
            }
            Ok(None) => {
                // Process still running, check timeout
                if clock.now().duration_since(start) >= timeout {
                    // Timeout exceeded
                    return Ok(None);
                }

                // Sleep before next poll
                clock.sleep(poll_interval);
            }
            Err(e) => {
                // Error in try_wait() - try to clean up anyway to prevent orphan
                let _ = child.kill();
                let _ = child.wait();
                return Err(HnError::HookError(format!(
                    "Failed to monitor child process: {}",
                    e
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HooksConfig;
    use crate::vcs::Worktree;
    use tempfile::TempDir;

    fn create_test_worktree(temp: &TempDir) -> Worktree {
        let wt_path = temp.path().join("test-worktree");
        std::fs::create_dir_all(&wt_path).unwrap();

        Worktree {
            name: "test-worktree".to_string(),
            path: wt_path,
            branch: "main".to_string(),
            commit: "abc123".to_string(),
            parent: None,
        }
    }

    #[test]
    fn test_execute_successful_hook() {
        let temp = TempDir::new().unwrap();
        let worktree = create_test_worktree(&temp);
        let state_dir = temp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let config = HooksConfig {
            post_create: Some("echo 'Hello from hook'".to_string()),
            pre_remove: None,
            timeout_seconds: 30,
        };

        let executor = HookExecutor::new(config, false);
        let result = executor.run_hook(HookType::PostCreate, &worktree, &state_dir);

        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_failing_hook() {
        let temp = TempDir::new().unwrap();
        let worktree = create_test_worktree(&temp);
        let state_dir = temp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let config = HooksConfig {
            post_create: Some("exit 1".to_string()),
            pre_remove: None,
            timeout_seconds: 30,
        };

        let executor = HookExecutor::new(config, false);
        let result = executor.run_hook(HookType::PostCreate, &worktree, &state_dir);

        assert!(result.is_err());
        if let Err(HnError::HookError(msg)) = result {
            assert!(msg.contains("failed"));
        } else {
            panic!("Expected HookError");
        }
    }

    #[test]
    fn test_missing_hook_is_noop() {
        let temp = TempDir::new().unwrap();
        let worktree = create_test_worktree(&temp);
        let state_dir = temp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let config = HooksConfig {
            post_create: None,
            pre_remove: None,
            timeout_seconds: 30,
        };

        let executor = HookExecutor::new(config, false);
        let result = executor.run_hook(HookType::PostCreate, &worktree, &state_dir);

        // Should succeed without doing anything
        assert!(result.is_ok());
    }

    #[test]
    fn test_environment_variables() {
        let temp = TempDir::new().unwrap();
        let worktree = create_test_worktree(&temp);
        let state_dir = temp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Create a hook that writes environment variables to a file
        let output_file = temp.path().join("env_output.txt");
        let hook_script = format!(
            r#"echo "WT_NAME=$WT_NAME" > {}
echo "WT_BRANCH=$WT_BRANCH" >> {}
echo "WT_COMMIT=$WT_COMMIT" >> {}"#,
            output_file.display(),
            output_file.display(),
            output_file.display()
        );

        let config = HooksConfig {
            post_create: Some(hook_script),
            pre_remove: None,
            timeout_seconds: 30,
        };

        let executor = HookExecutor::new(config, false);
        executor
            .run_hook(HookType::PostCreate, &worktree, &state_dir)
            .unwrap();

        // Verify environment variables were passed
        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("WT_NAME=test-worktree"));
        assert!(content.contains("WT_BRANCH=main"));
        assert!(content.contains("WT_COMMIT=abc123"));
    }

    #[test]
    fn test_skip_hooks_flag() {
        let temp = TempDir::new().unwrap();
        let worktree = create_test_worktree(&temp);
        let state_dir = temp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Create a hook that would fail
        let config = HooksConfig {
            post_create: Some("exit 1".to_string()),
            pre_remove: None,
            timeout_seconds: 30,
        };

        // With skip_hooks=true, should succeed even though hook would fail
        let executor = HookExecutor::new(config, true);
        let result = executor.run_hook(HookType::PostCreate, &worktree, &state_dir);

        assert!(result.is_ok(), "Hook should be skipped and not fail");
    }

    // Note: Timeout behavior is manually tested but not included in automated tests.
    // While we have a Clock abstraction for time operations, testing actual process
    // timeouts requires spawning real processes that take real time to complete.
    // To properly test timeouts without real-time delays would require mocking
    // process execution itself, which adds significant complexity.
    // The timeout implementation has been verified through manual testing.
}
