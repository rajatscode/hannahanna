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
    PreCreate,
    PostCreate,
    PreRemove,
    PostRemove,
    PostSwitch,
    PreIntegrate,
    PostIntegrate,
}

impl HookType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookType::PreCreate => "pre_create",
            HookType::PostCreate => "post_create",
            HookType::PreRemove => "pre_remove",
            HookType::PostRemove => "post_remove",
            HookType::PostSwitch => "post_switch",
            HookType::PreIntegrate => "pre_integrate",
            HookType::PostIntegrate => "post_integrate",
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

        // First, run the regular (unconditional) hook if configured
        let script = match hook_type {
            HookType::PreCreate => &self.config.pre_create,
            HookType::PostCreate => &self.config.post_create,
            HookType::PreRemove => &self.config.pre_remove,
            HookType::PostRemove => &self.config.post_remove,
            HookType::PostSwitch => &self.config.post_switch,
            HookType::PreIntegrate => &self.config.pre_integrate,
            HookType::PostIntegrate => &self.config.post_integrate,
        };

        if let Some(script) = script {
            self.execute_hook(hook_type, script, worktree, state_dir)?;
        }

        // Then, evaluate and run any conditional hooks that match
        let conditional_hooks = match hook_type {
            HookType::PreCreate => &self.config.pre_create_conditions,
            HookType::PostCreate => &self.config.post_create_conditions,
            HookType::PreRemove => &self.config.pre_remove_conditions,
            HookType::PostRemove => &self.config.post_remove_conditions,
            HookType::PostSwitch => &self.config.post_switch_conditions,
            HookType::PreIntegrate => &self.config.pre_integrate_conditions,
            HookType::PostIntegrate => &self.config.post_integrate_conditions,
        };

        for conditional_hook in conditional_hooks {
            if self.evaluate_condition(&conditional_hook.condition, &worktree.branch)? {
                self.execute_hook(hook_type, &conditional_hook.command, worktree, state_dir)?;
            }
        }

        Ok(())
    }

    /// Evaluate a condition against a branch name
    /// Supports: branch.startsWith('prefix'), branch.endsWith('suffix'), branch.contains('substring')
    fn evaluate_condition(&self, condition: &str, branch: &str) -> Result<bool> {
        let condition = condition.trim();

        // Parse: branch.startsWith('...')
        if let Some(prefix) = Self::parse_starts_with(condition) {
            return Ok(branch.starts_with(&prefix));
        }

        // Parse: branch.endsWith('...')
        if let Some(suffix) = Self::parse_ends_with(condition) {
            return Ok(branch.ends_with(&suffix));
        }

        // Parse: branch.contains('...')
        if let Some(substring) = Self::parse_contains(condition) {
            return Ok(branch.contains(&substring));
        }

        // Unsupported condition format
        Err(HnError::ConfigError(format!(
            "Invalid hook condition: '{}'. Supported formats: branch.startsWith('...'), branch.endsWith('...'), branch.contains('...')",
            condition
        )))
    }

    /// Parse branch.startsWith('prefix') and return the prefix
    fn parse_starts_with(condition: &str) -> Option<String> {
        Self::parse_branch_method(condition, "startsWith")
    }

    /// Parse branch.endsWith('suffix') and return the suffix
    fn parse_ends_with(condition: &str) -> Option<String> {
        Self::parse_branch_method(condition, "endsWith")
    }

    /// Parse branch.contains('substring') and return the substring
    fn parse_contains(condition: &str) -> Option<String> {
        Self::parse_branch_method(condition, "contains")
    }

    /// Generic parser for branch.method('value') patterns
    fn parse_branch_method(condition: &str, method: &str) -> Option<String> {
        let pattern = format!("branch.{}(", method);

        if !condition.starts_with(&pattern) {
            return None;
        }

        // Find the quoted string inside the parentheses
        let start_idx = pattern.len();
        let rest = &condition[start_idx..];

        // Support both single and double quotes
        for quote in &['\'', '"'] {
            if rest.starts_with(*quote) {
                // Find the closing quote
                if let Some(end_idx) = rest[1..].find(*quote) {
                    let value = rest[1..=end_idx].to_string();
                    // Check that it ends with ')' after the quote
                    if rest[end_idx + 2..].trim_start().starts_with(')') {
                        return Some(value);
                    }
                }
            }
        }

        None
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

        // Determine working directory based on hook type
        // Pre-create runs in parent of worktree since worktree doesn't exist yet
        // Post-remove runs in parent since worktree was just deleted
        let working_dir = match hook_type {
            HookType::PreCreate | HookType::PostRemove => {
                worktree.path.parent().unwrap_or(&worktree.path)
            }
            _ => &worktree.path,
        };

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
            .current_dir(working_dir)
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
    /// v0.5: Changed from WT_* to HNHN_* prefix to avoid collision with Hacker News CLI tools
    fn build_env(&self, worktree: &Worktree, state_dir: &Path) -> HashMap<String, String> {
        let mut env = HashMap::new();

        env.insert("HNHN_NAME".to_string(), worktree.name.clone());
        env.insert(
            "HNHN_PATH".to_string(),
            worktree.path.to_string_lossy().to_string(),
        );
        env.insert("HNHN_BRANCH".to_string(), worktree.branch.clone());
        env.insert("HNHN_COMMIT".to_string(), worktree.commit.clone());
        env.insert(
            "HNHN_STATE_DIR".to_string(),
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
            timeout_seconds: 30,
            ..Default::default()
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
            timeout_seconds: 30,
            ..Default::default()
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

        let config = HooksConfig::default();

        let executor = HookExecutor::new(config, false);
        let result = executor.run_hook(HookType::PostCreate, &worktree, &state_dir);

        // Should succeed without doing anything
        assert!(result.is_ok());
    }

    #[test]
    fn test_environment_variables_hnhn_prefix() {
        let temp = TempDir::new().unwrap();
        let worktree = create_test_worktree(&temp);
        let state_dir = temp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Create a hook that writes environment variables to a file
        let output_file = temp.path().join("env_output.txt");
        let hook_script = format!(
            r#"echo "HNHN_NAME=$HNHN_NAME" > {}
echo "HNHN_PATH=$HNHN_PATH" >> {}
echo "HNHN_BRANCH=$HNHN_BRANCH" >> {}
echo "HNHN_COMMIT=$HNHN_COMMIT" >> {}
echo "HNHN_STATE_DIR=$HNHN_STATE_DIR" >> {}"#,
            output_file.display(),
            output_file.display(),
            output_file.display(),
            output_file.display(),
            output_file.display()
        );

        let config = HooksConfig {
            post_create: Some(hook_script),
            timeout_seconds: 30,
            ..Default::default()
        };

        let executor = HookExecutor::new(config, false);
        executor
            .run_hook(HookType::PostCreate, &worktree, &state_dir)
            .unwrap();

        // Verify NEW HNHN_* environment variables are passed
        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("HNHN_NAME=test-worktree"), "HNHN_NAME should be set");
        assert!(content.contains("HNHN_BRANCH=main"), "HNHN_BRANCH should be set");
        assert!(content.contains("HNHN_COMMIT=abc123"), "HNHN_COMMIT should be set");
        assert!(content.contains("HNHN_PATH="), "HNHN_PATH should be set");
        assert!(content.contains("HNHN_STATE_DIR="), "HNHN_STATE_DIR should be set");
    }

    #[test]
    fn test_old_wt_variables_not_set() {
        // v0.5 BREAKING CHANGE: Old WT_* variables are no longer set
        let temp = TempDir::new().unwrap();
        let worktree = create_test_worktree(&temp);
        let state_dir = temp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Create a hook that checks for absence of old variables
        let output_file = temp.path().join("env_output.txt");
        let hook_script = format!(
            r#"echo "WT_NAME=${{WT_NAME:-NOT_SET}}" > {}
echo "WT_BRANCH=${{WT_BRANCH:-NOT_SET}}" >> {}"#,
            output_file.display(),
            output_file.display()
        );

        let config = HooksConfig {
            post_create: Some(hook_script),
            timeout_seconds: 30,
            ..Default::default()
        };

        let executor = HookExecutor::new(config, false);
        executor
            .run_hook(HookType::PostCreate, &worktree, &state_dir)
            .unwrap();

        // Verify OLD WT_* variables are NOT set (breaking change in v0.5)
        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("WT_NAME=NOT_SET"), "Old WT_NAME should not be set");
        assert!(content.contains("WT_BRANCH=NOT_SET"), "Old WT_BRANCH should not be set");
    }

    #[test]
    fn test_all_hook_types_use_hnhn_variables() {
        // Verify all hook types receive HNHN_* variables
        let temp = TempDir::new().unwrap();
        let worktree = create_test_worktree(&temp);
        let state_dir = temp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let hook_types = vec![
            (HookType::PreCreate, "pre_create"),
            (HookType::PostCreate, "post_create"),
            (HookType::PreRemove, "pre_remove"),
            (HookType::PostRemove, "post_remove"),
            (HookType::PostSwitch, "post_switch"),
            (HookType::PreIntegrate, "pre_integrate"),
            (HookType::PostIntegrate, "post_integrate"),
        ];

        for (hook_type, hook_name) in hook_types {
            let output_file = temp.path().join(format!("env_{}.txt", hook_name));
            let hook_script = format!(
                r#"echo "HNHN_NAME=$HNHN_NAME" > {}"#,
                output_file.display()
            );

            let mut config = HooksConfig::default();
            match hook_type {
                HookType::PreCreate => config.pre_create = Some(hook_script),
                HookType::PostCreate => config.post_create = Some(hook_script),
                HookType::PreRemove => config.pre_remove = Some(hook_script),
                HookType::PostRemove => config.post_remove = Some(hook_script),
                HookType::PostSwitch => config.post_switch = Some(hook_script),
                HookType::PreIntegrate => config.pre_integrate = Some(hook_script),
                HookType::PostIntegrate => config.post_integrate = Some(hook_script),
            }

            let executor = HookExecutor::new(config, false);
            executor.run_hook(hook_type, &worktree, &state_dir).unwrap();

            // Verify HNHN_NAME is set for this hook type
            let content = std::fs::read_to_string(&output_file).unwrap();
            assert!(
                content.contains("HNHN_NAME=test-worktree"),
                "Hook {} should receive HNHN_NAME",
                hook_name
            );
        }
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
            timeout_seconds: 30,
            ..Default::default()
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

    #[test]
    fn test_parse_starts_with_single_quotes() {
        let result = HookExecutor::parse_starts_with("branch.startsWith('feature/')");
        assert_eq!(result, Some("feature/".to_string()));
    }

    #[test]
    fn test_parse_starts_with_double_quotes() {
        let result = HookExecutor::parse_starts_with("branch.startsWith(\"hotfix/\")");
        assert_eq!(result, Some("hotfix/".to_string()));
    }

    #[test]
    fn test_parse_ends_with() {
        let result = HookExecutor::parse_ends_with("branch.endsWith('-prod')");
        assert_eq!(result, Some("-prod".to_string()));
    }

    #[test]
    fn test_parse_contains() {
        let result = HookExecutor::parse_contains("branch.contains('bugfix')");
        assert_eq!(result, Some("bugfix".to_string()));
    }

    #[test]
    fn test_parse_invalid_condition() {
        let result = HookExecutor::parse_starts_with("branch.invalid('test')");
        assert_eq!(result, None);
    }

    #[test]
    fn test_evaluate_condition_starts_with_match() {
        let config = HooksConfig::default();
        let executor = HookExecutor::new(config, false);

        let result = executor
            .evaluate_condition("branch.startsWith('feature/')", "feature/new-api")
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_evaluate_condition_starts_with_no_match() {
        let config = HooksConfig::default();
        let executor = HookExecutor::new(config, false);

        let result = executor
            .evaluate_condition("branch.startsWith('feature/')", "hotfix/bug-123")
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_evaluate_condition_ends_with_match() {
        let config = HooksConfig::default();
        let executor = HookExecutor::new(config, false);

        let result = executor
            .evaluate_condition("branch.endsWith('-prod')", "release-prod")
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_evaluate_condition_contains_match() {
        let config = HooksConfig::default();
        let executor = HookExecutor::new(config, false);

        let result = executor
            .evaluate_condition("branch.contains('bugfix')", "feature/bugfix-auth")
            .unwrap();
        assert!(result);
    }

    #[test]
    fn test_evaluate_condition_invalid() {
        let config = HooksConfig::default();
        let executor = HookExecutor::new(config, false);

        let result = executor.evaluate_condition("invalid.condition()", "main");
        assert!(result.is_err());
    }
}
