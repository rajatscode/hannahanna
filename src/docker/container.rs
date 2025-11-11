// Docker container lifecycle management
// Start, stop, monitor containers for worktrees

use crate::config::DockerConfig;
use crate::errors::{HnError, Result};
use std::path::Path;
use std::process::Command;

/// Container status information
#[derive(Debug, Clone)]
pub struct ContainerStatus {
    pub worktree_name: String,
    pub running: bool,
    pub container_count: usize,
}

/// Manages Docker container lifecycle for worktrees
pub struct ContainerManager<'a> {
    config: &'a DockerConfig,
    state_dir: &'a Path,
}

impl<'a> ContainerManager<'a> {
    /// Create a new container manager
    pub fn new(config: &'a DockerConfig, state_dir: &'a Path) -> Result<Self> {
        Ok(Self { config, state_dir })
    }

    /// Check if Docker is available on the system
    pub fn is_docker_available(&self) -> bool {
        Command::new("docker")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Get container status for a worktree
    pub fn get_status(&self, worktree_name: &str, worktree_path: &Path) -> Result<ContainerStatus> {
        let project_name = self.get_project_name(worktree_name);

        // Check if containers are running using docker-compose
        let running = if self.is_docker_available() {
            self.check_containers_running(worktree_name, worktree_path)
        } else {
            false
        };

        Ok(ContainerStatus {
            worktree_name: worktree_name.to_string(),
            running,
            container_count: if running { 1 } else { 0 },
        })
    }

    /// Start containers for a worktree
    pub fn start(&self, worktree_name: &str, worktree_path: &Path) -> Result<()> {
        if !self.is_docker_available() {
            return Err(HnError::DockerError(
                "Docker is not available. Please install Docker.".to_string(),
            ));
        }

        let cmd = self.build_start_command(worktree_name, worktree_path)?;
        self.execute_command(&cmd, worktree_path)?;

        Ok(())
    }

    /// Stop containers for a worktree
    pub fn stop(&self, worktree_name: &str, worktree_path: &Path) -> Result<()> {
        if !self.is_docker_available() {
            return Ok(()); // Silent success if Docker not available
        }

        let cmd = self.build_stop_command(worktree_name, worktree_path)?;
        self.execute_command(&cmd, worktree_path)?;

        Ok(())
    }

    /// List all container statuses
    pub fn list_all(&self) -> Result<Vec<ContainerStatus>> {
        // Return empty list for now - would need worktree list to implement fully
        Ok(Vec::new())
    }

    /// Get Docker Compose project name for a worktree
    pub fn get_project_name(&self, worktree_name: &str) -> String {
        // Sanitize name for Docker (replace invalid characters)
        worktree_name.replace('/', "-").replace('_', "-")
    }

    /// Build docker-compose up command
    pub fn build_start_command(&self, worktree_name: &str, worktree_path: &Path) -> Result<String> {
        let project_name = self.get_project_name(worktree_name);
        let override_file = self
            .state_dir
            .join(worktree_name)
            .join("docker-compose.override.yml");

        let mut cmd = format!("docker-compose -p {} ", project_name);

        // Add compose file
        cmd.push_str(&format!("-f {} ", self.config.compose_file));

        // Add override file if it exists
        if override_file.exists() {
            cmd.push_str(&format!(
                "-f {} ",
                override_file.to_string_lossy()
            ));
        }

        cmd.push_str("up -d");

        Ok(cmd)
    }

    /// Build docker-compose down command
    pub fn build_stop_command(&self, worktree_name: &str, worktree_path: &Path) -> Result<String> {
        let project_name = self.get_project_name(worktree_name);

        Ok(format!("docker-compose -p {} down", project_name))
    }

    /// Build docker-compose logs command
    pub fn build_logs_command(
        &self,
        worktree_name: &str,
        worktree_path: &Path,
        service: Option<&str>,
    ) -> Result<String> {
        let project_name = self.get_project_name(worktree_name);

        let mut cmd = format!("docker-compose -p {} logs -f", project_name);

        if let Some(svc) = service {
            cmd.push_str(&format!(" {}", svc));
        }

        Ok(cmd)
    }

    /// Clean up orphaned containers (for removed worktrees)
    pub fn cleanup_orphaned(&self, active_worktrees: &[String]) -> Result<()> {
        if !self.is_docker_available() {
            return Ok(());
        }

        // List all docker-compose projects
        // For each project not in active_worktrees, stop and remove

        // This is a simplified implementation
        // Full implementation would scan for hn-managed projects and clean them up

        Ok(())
    }

    /// Check if containers are running for a worktree
    fn check_containers_running(&self, worktree_name: &str, worktree_path: &Path) -> bool {
        let project_name = self.get_project_name(worktree_name);

        Command::new("docker-compose")
            .arg("-p")
            .arg(&project_name)
            .arg("ps")
            .arg("-q")
            .output()
            .map(|output| {
                output.status.success() && !output.stdout.is_empty()
            })
            .unwrap_or(false)
    }

    /// Execute a shell command in the worktree directory
    fn execute_command(&self, cmd: &str, worktree_path: &Path) -> Result<()> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(worktree_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::DockerError(format!(
                "Command failed: {}",
                stderr
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_project_name() {
        let temp_dir = TempDir::new().unwrap();
        let config = DockerConfig::default();
        let manager = ContainerManager::new(&config, temp_dir.path()).unwrap();

        assert_eq!(manager.get_project_name("feature-x"), "feature-x");
        assert_eq!(manager.get_project_name("feature/test"), "feature-test");
        assert_eq!(manager.get_project_name("my_feature"), "my-feature");
    }

    #[test]
    fn test_build_commands() {
        let temp_dir = TempDir::new().unwrap();
        let worktree_dir = temp_dir.path().join("feature-test");
        std::fs::create_dir_all(&worktree_dir).unwrap();

        let config = DockerConfig::default();
        let manager = ContainerManager::new(&config, temp_dir.path()).unwrap();

        let start_cmd = manager.build_start_command("feature-test", &worktree_dir).unwrap();
        assert!(start_cmd.contains("docker-compose"));
        assert!(start_cmd.contains("up -d"));

        let stop_cmd = manager.build_stop_command("feature-test", &worktree_dir).unwrap();
        assert!(stop_cmd.contains("docker-compose"));
        assert!(stop_cmd.contains("down"));
    }
}
