// Docker container lifecycle management
// Start, stop, monitor containers for worktrees

use crate::config::DockerConfig;
use crate::errors::{HnError, Result};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Container status information
#[derive(Debug, Clone)]
pub struct ContainerStatus {
    pub running: bool,
    pub container_count: usize,
}

/// Docker Compose command variant
#[derive(Debug, Clone, Copy)]
enum DockerComposeVariant {
    /// Legacy docker-compose (with hyphen)
    Hyphenated,
    /// Modern docker compose (no hyphen, subcommand of docker)
    Subcommand,
}

/// Manages Docker container lifecycle for worktrees
pub struct ContainerManager<'a> {
    config: &'a DockerConfig,
    state_dir: &'a Path,
    compose_variant: DockerComposeVariant,
}

impl<'a> ContainerManager<'a> {
    /// Create a new container manager
    pub fn new(config: &'a DockerConfig, state_dir: &'a Path) -> Result<Self> {
        let compose_variant = Self::detect_compose_variant();
        Ok(Self {
            config,
            state_dir,
            compose_variant,
        })
    }

    /// Detect which docker-compose variant is available
    fn detect_compose_variant() -> DockerComposeVariant {
        // Try modern "docker compose" first
        let modern = Command::new("docker")
            .arg("compose")
            .arg("version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        if modern {
            return DockerComposeVariant::Subcommand;
        }

        // Fall back to legacy "docker-compose"
        DockerComposeVariant::Hyphenated
    }

    /// Get the docker-compose command and args based on detected variant
    fn get_compose_command(&self, args: &[String]) -> (String, Vec<String>) {
        match self.compose_variant {
            DockerComposeVariant::Subcommand => {
                let mut compose_args = vec!["compose".to_string()];
                compose_args.extend_from_slice(args);
                ("docker".to_string(), compose_args)
            }
            DockerComposeVariant::Hyphenated => ("docker-compose".to_string(), args.to_vec()),
        }
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
        // Check if containers are running using docker-compose
        let (running, container_count) = if self.is_docker_available() {
            let count = self.count_running_containers(worktree_name, worktree_path);
            (count > 0, count)
        } else {
            (false, 0)
        };

        Ok(ContainerStatus {
            running,
            container_count,
        })
    }

    /// Start containers for a worktree (secure, no command injection)
    pub fn start(&self, worktree_name: &str, worktree_path: &Path) -> Result<()> {
        if !self.is_docker_available() {
            return Err(HnError::DockerError(
                "Docker is not available. Please install Docker.".to_string(),
            ));
        }

        let args = self.build_start_command_args(worktree_name)?;
        let (program, full_args) = self.get_compose_command(&args);
        self.execute_command_safe(&program, &full_args, worktree_path)?;

        // Wait for health checks if enabled
        if self.config.healthcheck.enabled {
            self.wait_for_healthy(worktree_name)?;
        }

        Ok(())
    }

    /// Stop containers for a worktree (secure, no command injection)
    pub fn stop(&self, worktree_name: &str, worktree_path: &Path) -> Result<()> {
        if !self.is_docker_available() {
            return Ok(()); // Silent success if Docker not available
        }

        let args = self.build_stop_command_args(worktree_name)?;
        let (program, full_args) = self.get_compose_command(&args);
        self.execute_command_safe(&program, &full_args, worktree_path)?;

        Ok(())
    }

    /// Get Docker Compose project name for a worktree
    /// Docker project names must be lowercase alphanumeric with hyphens only
    pub fn get_project_name(&self, worktree_name: &str) -> String {
        // Sanitize name for Docker Compose project name requirements:
        // - Lowercase only
        // - Alphanumeric and hyphens
        // - Cannot start/end with hyphen
        let sanitized = worktree_name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>();

        // Remove leading/trailing hyphens and collapse multiple hyphens
        sanitized
            .trim_matches('-')
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Build docker-compose up command arguments (safe from injection)
    fn build_start_command_args(&self, worktree_name: &str) -> Result<Vec<String>> {
        let project_name = self.get_project_name(worktree_name);
        let override_file = self
            .state_dir
            .join(worktree_name)
            .join("docker-compose.override.yml");

        let mut args = vec![
            "-p".to_string(),
            project_name,
            "-f".to_string(),
            self.config.compose_file.clone(),
        ];

        // Add override file if it exists
        if override_file.exists() {
            args.push("-f".to_string());
            args.push(override_file.to_string_lossy().to_string());
        }

        args.push("up".to_string());
        args.push("-d".to_string());

        Ok(args)
    }

    /// Build docker-compose down command arguments (safe from injection)
    fn build_stop_command_args(&self, worktree_name: &str) -> Result<Vec<String>> {
        let project_name = self.get_project_name(worktree_name);

        Ok(vec!["-p".to_string(), project_name, "down".to_string()])
    }

    /// Build docker-compose logs command arguments (safe from injection)
    fn build_logs_command_args(
        &self,
        worktree_name: &str,
        service: Option<&str>,
    ) -> Result<Vec<String>> {
        let project_name = self.get_project_name(worktree_name);

        let mut args = vec![
            "-p".to_string(),
            project_name,
            "logs".to_string(),
            "-f".to_string(),
        ];

        if let Some(svc) = service {
            args.push(svc.to_string());
        }

        Ok(args)
    }

    /// Legacy: Build docker-compose up command string (for backward compatibility)
    #[deprecated(note = "Use build_start_command_args for security")]
    #[allow(dead_code)]
    pub fn build_start_command(
        &self,
        worktree_name: &str,
        _worktree_path: &Path,
    ) -> Result<String> {
        let args = self.build_start_command_args(worktree_name)?;
        Ok(format!("docker-compose {}", args.join(" ")))
    }

    /// Legacy: Build docker-compose down command string (for backward compatibility)
    #[deprecated(note = "Use build_stop_command_args for security")]
    #[allow(dead_code)]
    pub fn build_stop_command(&self, worktree_name: &str, _worktree_path: &Path) -> Result<String> {
        let args = self.build_stop_command_args(worktree_name)?;
        Ok(format!("docker-compose {}", args.join(" ")))
    }

    /// Legacy: Build docker-compose logs command string (for backward compatibility)
    #[deprecated(note = "Use build_logs_command_args for security")]
    pub fn build_logs_command(
        &self,
        worktree_name: &str,
        _worktree_path: &Path,
        service: Option<&str>,
    ) -> Result<String> {
        let args = self.build_logs_command_args(worktree_name, service)?;
        Ok(format!("docker-compose {}", args.join(" ")))
    }

    /// Clean up orphaned containers (for removed worktrees)
    pub fn cleanup_orphaned(&self, active_worktrees: &[String]) -> Result<()> {
        if !self.is_docker_available() {
            return Ok(());
        }

        // Get list of all docker-compose projects by listing containers
        // and extracting their project labels
        let output = Command::new("docker")
            .arg("ps")
            .arg("-a")
            .arg("--filter")
            .arg("label=com.docker.compose.project")
            .arg("--format")
            .arg("{{.Label \"com.docker.compose.project\"}}")
            .output()?;

        if !output.status.success() {
            return Err(HnError::DockerError(
                "Failed to list docker-compose projects".to_string(),
            ));
        }

        let projects_output = String::from_utf8_lossy(&output.stdout);
        let mut projects: std::collections::HashSet<String> = projects_output
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect();

        // Convert active worktrees to project names
        let active_projects: std::collections::HashSet<String> = active_worktrees
            .iter()
            .map(|wt| self.get_project_name(wt))
            .collect();

        // Find orphaned projects (those not in active worktrees)
        projects.retain(|project| !active_projects.contains(project));

        // Stop and remove each orphaned project
        for project in projects {
            eprintln!("Cleaning up orphaned containers for project: {}", project);

            // Build stop command with the appropriate variant
            let args = vec![
                "-p".to_string(),
                project.clone(),
                "down".to_string(),
                "--remove-orphans".to_string(),
            ];
            let (program, full_args) = self.get_compose_command(&args);

            // Stop containers
            let stop_result = Command::new(program).args(full_args).output();

            match stop_result {
                Ok(output) if output.status.success() => {
                    eprintln!("  ✓ Cleaned up {}", project);
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("  ⚠ Warning: Failed to clean up {}: {}", project, stderr);
                }
                Err(e) => {
                    eprintln!("  ⚠ Warning: Failed to clean up {}: {}", project, e);
                }
            }
        }

        Ok(())
    }

    /// Check if containers are running for a worktree
    /// Note: Currently not directly called but kept for potential future use
    #[allow(dead_code)]
    fn check_containers_running(&self, worktree_name: &str, _worktree_path: &Path) -> bool {
        let project_name = self.get_project_name(worktree_name);
        let args = vec![
            "-p".to_string(),
            project_name,
            "ps".to_string(),
            "-q".to_string(),
        ];
        let (program, full_args) = self.get_compose_command(&args);

        Command::new(program)
            .args(full_args)
            .output()
            .map(|output| output.status.success() && !output.stdout.is_empty())
            .unwrap_or(false)
    }

    /// Count the number of running containers for a worktree
    fn count_running_containers(&self, worktree_name: &str, _worktree_path: &Path) -> usize {
        let project_name = self.get_project_name(worktree_name);
        let args = vec![
            "-p".to_string(),
            project_name,
            "ps".to_string(),
            "-q".to_string(),
        ];
        let (program, full_args) = self.get_compose_command(&args);

        Command::new(program)
            .args(full_args)
            .output()
            .map(|output| {
                if output.status.success() {
                    // Count non-empty lines (each line is a container ID)
                    String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .filter(|line| !line.trim().is_empty())
                        .count()
                } else {
                    0
                }
            })
            .unwrap_or(0)
    }

    /// Wait for containers to become healthy
    fn wait_for_healthy(&self, worktree_name: &str) -> Result<()> {
        let timeout = self.parse_timeout(&self.config.healthcheck.timeout)?;
        let start = std::time::Instant::now();
        let project_name = self.get_project_name(worktree_name);

        eprintln!(
            "Waiting for containers to become healthy (timeout: {}s)...",
            timeout
        );

        loop {
            if start.elapsed().as_secs() > timeout {
                return Err(HnError::DockerError(format!(
                    "Health check timeout after {}s",
                    timeout
                )));
            }

            // Check container health status
            let args = vec![
                "-p".to_string(),
                project_name.clone(),
                "ps".to_string(),
                "--format".to_string(),
                "{{.Service}},{{.Status}}".to_string(),
            ];
            let (program, full_args) = self.get_compose_command(&args);

            let output = Command::new(program).args(full_args).output()?;

            if !output.status.success() {
                return Err(HnError::DockerError(
                    "Failed to check container health".to_string(),
                ));
            }

            let status_output = String::from_utf8_lossy(&output.stdout);
            let mut all_healthy = true;
            let mut any_containers = false;

            for line in status_output.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                any_containers = true;
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 2 {
                    let status = parts[1];
                    // Check if container is running (not exited, not unhealthy)
                    if status.contains("(unhealthy)") || status.contains("Exit") {
                        all_healthy = false;
                        break;
                    }
                }
            }

            if !any_containers {
                return Err(HnError::DockerError(
                    "No containers found for project".to_string(),
                ));
            }

            if all_healthy {
                eprintln!("✓ All containers are healthy");
                return Ok(());
            }

            // Wait before next check
            thread::sleep(Duration::from_secs(2));
        }
    }

    /// Parse timeout string (e.g., "30s", "1m") into seconds
    fn parse_timeout(&self, timeout_str: &str) -> Result<u64> {
        let timeout_str = timeout_str.trim();

        if let Some(num_str) = timeout_str.strip_suffix('s') {
            num_str.parse::<u64>().map_err(|_| {
                HnError::ConfigError(format!("Invalid timeout value: {}", timeout_str))
            })
        } else if let Some(num_str) = timeout_str.strip_suffix('m') {
            let minutes = num_str.parse::<u64>().map_err(|_| {
                HnError::ConfigError(format!("Invalid timeout value: {}", timeout_str))
            })?;
            Ok(minutes * 60)
        } else {
            // Default to seconds if no unit specified
            timeout_str.parse::<u64>().map_err(|_| {
                HnError::ConfigError(format!("Invalid timeout value: {}", timeout_str))
            })
        }
    }

    /// Execute a command safely without shell injection (recommended)
    fn execute_command_safe(
        &self,
        program: &str,
        args: &[String],
        worktree_path: &Path,
    ) -> Result<()> {
        let output = Command::new(program)
            .args(args)
            .current_dir(worktree_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::DockerError(format!("Command failed: {}", stderr)));
        }

        Ok(())
    }

    /// Legacy: Execute a shell command in the worktree directory
    /// WARNING: This is vulnerable to command injection, use execute_command_safe instead
    #[deprecated(note = "Use execute_command_safe to prevent command injection")]
    #[allow(dead_code)]
    fn execute_command(&self, cmd: &str, worktree_path: &Path) -> Result<()> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(worktree_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::DockerError(format!("Command failed: {}", stderr)));
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
    #[allow(deprecated)] // Testing deprecated methods for backward compatibility
    fn test_build_commands() {
        let temp_dir = TempDir::new().unwrap();
        let worktree_dir = temp_dir.path().join("feature-test");
        std::fs::create_dir_all(&worktree_dir).unwrap();

        let config = DockerConfig::default();
        let manager = ContainerManager::new(&config, temp_dir.path()).unwrap();

        let start_cmd = manager
            .build_start_command("feature-test", &worktree_dir)
            .unwrap();
        assert!(start_cmd.contains("docker-compose"));
        assert!(start_cmd.contains("up -d"));

        let stop_cmd = manager
            .build_stop_command("feature-test", &worktree_dir)
            .unwrap();
        assert!(stop_cmd.contains("docker-compose"));
        assert!(stop_cmd.contains("down"));
    }
}
