use crate::config::HooksConfig;
use crate::errors::{HnError, Result};
use crate::vcs::Worktree;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

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
}

impl HookExecutor {
    pub fn new(config: HooksConfig) -> Self {
        Self { config }
    }

    /// Execute a hook if it's configured
    pub fn run_hook(
        &self,
        hook_type: HookType,
        worktree: &Worktree,
        state_dir: &Path,
    ) -> Result<()> {
        let script = match hook_type {
            HookType::PostCreate => &self.config.post_create,
            HookType::PreRemove => &self.config.pre_remove,
        };

        if let Some(script) = script {
            self.execute_hook(hook_type, script, worktree, state_dir)?;
        }

        Ok(())
    }

    /// Execute the hook script
    fn execute_hook(
        &self,
        hook_type: HookType,
        script: &str,
        worktree: &Worktree,
        state_dir: &Path,
    ) -> Result<()> {
        // Build environment variables
        let env = self.build_env(worktree, state_dir);

        // Execute shell command
        let output = Command::new("sh")
            .arg("-c")
            .arg(script)
            .current_dir(&worktree.path)
            .envs(env)
            .output()?;

        // Check exit code
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            return Err(HnError::HookError(format!(
                "{} hook failed with exit code {}\nStdout: {}\nStderr: {}",
                hook_type.as_str(),
                output.status.code().unwrap_or(-1),
                stdout,
                stderr
            )));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HooksConfig;
    use crate::vcs::Worktree;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_worktree(temp: &TempDir) -> Worktree {
        let wt_path = temp.path().join("test-worktree");
        std::fs::create_dir_all(&wt_path).unwrap();

        Worktree {
            name: "test-worktree".to_string(),
            path: wt_path,
            branch: "main".to_string(),
            commit: "abc123".to_string(),
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
        };

        let executor = HookExecutor::new(config);
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
        };

        let executor = HookExecutor::new(config);
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
        };

        let executor = HookExecutor::new(config);
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
        };

        let executor = HookExecutor::new(config);
        executor
            .run_hook(HookType::PostCreate, &worktree, &state_dir)
            .unwrap();

        // Verify environment variables were passed
        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("WT_NAME=test-worktree"));
        assert!(content.contains("WT_BRANCH=main"));
        assert!(content.contains("WT_COMMIT=abc123"));
    }
}
