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
        // Validate inputs
        Self::validate_worktree_name(worktree_name)?;

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

    /// Restart containers for a worktree (secure, no command injection)
    pub fn restart(&self, worktree_name: &str, worktree_path: &Path) -> Result<()> {
        let project_name = self.get_project_name(worktree_name);

        // Build docker-compose restart command safely
        let args = vec![
            "-p".to_string(),
            project_name,
            "restart".to_string(),
        ];

        let (program, compose_args) = self.get_compose_command(&args);

        // Execute restart without shell - no injection risk
        let output = Command::new(&program)
            .args(&compose_args)
            .current_dir(worktree_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(HnError::DockerError(format!(
                "Failed to restart containers for '{}': {}",
                worktree_name, stderr
            )));
        }

        Ok(())
    }

    /// Stop containers for a worktree (secure, no command injection)
    pub fn stop(&self, worktree_name: &str, worktree_path: &Path) -> Result<()> {
        // Validate inputs
        Self::validate_worktree_name(worktree_name)?;

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

    /// Get logs command with variant detection (safe from injection)
    /// Returns (program, args) tuple ready for direct execution
    pub fn get_logs_command(
        &self,
        worktree_name: &str,
        service: Option<&str>,
    ) -> Result<(String, Vec<String>)> {
        // Validate inputs
        Self::validate_worktree_name(worktree_name)?;
        if let Some(svc) = service {
            Self::validate_service_name(svc)?;
        }

        let args = self.build_logs_command_args(worktree_name, service)?;
        Ok(self.get_compose_command(&args))
    }

    /// Validate worktree name for security
    pub fn validate_worktree_name(name: &str) -> Result<()> {
        // Check length
        if name.is_empty() || name.len() > 255 {
            return Err(HnError::DockerError(
                "Worktree name must be between 1 and 255 characters".to_string(),
            ));
        }

        // Check for dangerous characters
        let dangerous_chars = [
            '$', '`', '\\', '\n', '\r', ';', '|', '&', '<', '>', '(', ')', '{', '}',
        ];
        if name.chars().any(|c| dangerous_chars.contains(&c)) {
            return Err(HnError::DockerError(
                "Worktree name contains invalid characters".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate service name for security
    pub fn validate_service_name(name: &str) -> Result<()> {
        // Check length
        if name.is_empty() || name.len() > 255 {
            return Err(HnError::DockerError(
                "Service name must be between 1 and 255 characters".to_string(),
            ));
        }

        // Service names should be alphanumeric with hyphens and underscores only
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(HnError::DockerError(
                "Service name must contain only alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            ));
        }

        Ok(())
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
    pub fn parse_timeout(&self, timeout_str: &str) -> Result<u64> {
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

    /// Execute a command safely without shell injection
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

    // ============================================================================
    // Unit Tests for Validation Functions
    // ============================================================================

    #[test]
    fn test_validate_worktree_name_valid() {
        // Test that valid names pass validation
        let max_length_name = "x".repeat(255);
        let valid_names = vec![
            "feature",
            "feature-123",
            "fix-bug",
            "my.branch",
            "test_branch",
            "Feature-Branch",
            "a",                      // minimum length
            max_length_name.as_str(), // maximum length
        ];

        for name in valid_names {
            let result = ContainerManager::validate_worktree_name(name);
            assert!(
                result.is_ok(),
                "Valid name '{}' should pass validation",
                name
            );
        }
    }

    #[test]
    fn test_validate_worktree_name_dangerous_characters() {
        // Test that dangerous shell metacharacters are rejected
        let dangerous_inputs = vec![
            ("test$var", '$', "dollar sign"),
            ("test`cmd`", '`', "backtick"),
            ("test\\escape", '\\', "backslash"),
            ("test\nline", '\n', "newline"),
            ("test\rreturn", '\r', "carriage return"),
            ("test;cmd", ';', "semicolon"),
            ("test|pipe", '|', "pipe"),
            ("test&bg", '&', "ampersand"),
            ("test<input", '<', "less than"),
            ("test>output", '>', "greater than"),
            ("test(sub)", '(', "left paren"),
            ("test)sub", ')', "right paren"),
            ("test{group}", '{', "left brace"),
            ("test}group", '}', "right brace"),
        ];

        for (input, dangerous_char, description) in dangerous_inputs {
            let result = ContainerManager::validate_worktree_name(input);
            assert!(
                result.is_err(),
                "Should reject worktree name with {} ({}): '{}'",
                description,
                dangerous_char,
                input
            );

            let err = result.unwrap_err();
            let err_msg = format!("{}", err);
            assert!(
                err_msg.contains("invalid characters") || err_msg.contains("Invalid"),
                "Error message should mention invalid characters for '{}', got: {}",
                input,
                err_msg
            );
        }
    }

    #[test]
    fn test_validate_worktree_name_empty() {
        let result = ContainerManager::validate_worktree_name("");
        assert!(result.is_err(), "Should reject empty worktree name");

        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("between 1 and 255") || err_msg.contains("empty"),
            "Error should mention length requirement"
        );
    }

    #[test]
    fn test_validate_worktree_name_too_long() {
        let long_name = "a".repeat(256);
        let result = ContainerManager::validate_worktree_name(&long_name);
        assert!(
            result.is_err(),
            "Should reject worktree name longer than 255 characters"
        );

        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("255") || err_msg.contains("length"),
            "Error should mention length limit"
        );
    }

    #[test]
    fn test_validate_worktree_name_command_injection_attempts() {
        // Test specific command injection attack vectors
        let attack_vectors = vec![
            "test$(rm -rf /)",       // Command substitution
            "test`whoami`",          // Backtick execution
            "test; rm -rf /",        // Command chaining
            "test | sh",             // Pipe to shell
            "test & malicious",      // Background execution
            "test > /etc/passwd",    // Output redirection
            "test < /etc/passwd",    // Input redirection
            "(malicious)",           // Subshell
            "{malicious; commands}", // Command grouping
            "test\nrm -rf /",        // Newline injection
            "test\rmalicious",       // Carriage return injection
            "test\\ninjection",      // Escape sequence
        ];

        for attack in attack_vectors {
            let result = ContainerManager::validate_worktree_name(attack);
            assert!(
                result.is_err(),
                "Should reject command injection attempt: '{}'",
                attack
            );
        }
    }

    #[test]
    fn test_validate_service_name_valid() {
        // Test that valid service names pass validation
        let max_length_name = "x".repeat(255);
        let valid_names = vec![
            "app",
            "web",
            "api-server",
            "db_service",
            "cache-1",
            "worker_2",
            "MyService",
            "Service-Name_123",
            "a",                      // minimum length
            max_length_name.as_str(), // maximum length
        ];

        for name in valid_names {
            let result = ContainerManager::validate_service_name(name);
            assert!(
                result.is_ok(),
                "Valid service name '{}' should pass validation",
                name
            );
        }
    }

    #[test]
    fn test_validate_service_name_invalid_characters() {
        // Test that non-alphanumeric characters (except - and _) are rejected
        let invalid_names = vec![
            "app@service",
            "web.server",
            "api#service",
            "db!service",
            "cache service", // space
            "worker$1",
            "service;cmd",
            "app|pipe",
            "test&bg",
            "app>out",
            "app<in",
            "app(sub)",
            "app{group}",
            "app`cmd`",
            "app\\escape",
            "app\nnewline",
            "app\rreturn",
        ];

        for name in invalid_names {
            let result = ContainerManager::validate_service_name(name);
            assert!(
                result.is_err(),
                "Should reject invalid service name: '{}'",
                name
            );

            let err_msg = format!("{}", result.unwrap_err());
            assert!(
                err_msg.contains("alphanumeric") || err_msg.contains("Invalid"),
                "Error should mention alphanumeric requirement for '{}', got: {}",
                name,
                err_msg
            );
        }
    }

    #[test]
    fn test_validate_service_name_empty() {
        let result = ContainerManager::validate_service_name("");
        assert!(result.is_err(), "Should reject empty service name");

        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("between 1 and 255") || err_msg.contains("empty"),
            "Error should mention length requirement"
        );
    }

    #[test]
    fn test_validate_service_name_too_long() {
        let long_name = "a".repeat(256);
        let result = ContainerManager::validate_service_name(&long_name);
        assert!(
            result.is_err(),
            "Should reject service name longer than 255 characters"
        );

        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("255") || err_msg.contains("length"),
            "Error should mention length limit"
        );
    }

    // ============================================================================
    // Unit Tests for Docker Compose Variant Detection
    // ============================================================================

    #[test]
    fn test_detect_compose_variant() {
        // Test that variant detection doesn't crash
        // Actual variant depends on system Docker installation
        let result = ContainerManager::detect_compose_variant();

        // Should return either Subcommand or Hyphenated
        // We can't assert which one without knowing the system state,
        // but we can verify the function executes
        match result {
            DockerComposeVariant::Subcommand => {
                // Modern "docker compose" is available
            }
            DockerComposeVariant::Hyphenated => {
                // Legacy "docker-compose" is available
            }
        }
    }

    #[test]
    fn test_get_compose_command_uses_variant() {
        // Test that get_compose_command uses the detected variant
        let temp_dir = TempDir::new().unwrap();
        let config = DockerConfig::default();
        let manager = ContainerManager::new(&config, temp_dir.path()).unwrap();

        // Test with sample args
        let args = vec!["up".to_string(), "-d".to_string()];
        let (program, cmd_args) = manager.get_compose_command(&args);

        // Should use either modern "docker" or legacy "docker-compose"
        assert!(
            program == "docker" || program == "docker-compose",
            "Program should be 'docker' or 'docker-compose', got: {}",
            program
        );

        // Verify args are included
        assert!(
            cmd_args.iter().any(|a| a == "up"),
            "Command args should contain 'up'"
        );
        assert!(
            cmd_args.iter().any(|a| a == "-d"),
            "Command args should contain '-d'"
        );

        // If using modern variant, args should contain "compose" subcommand
        if program == "docker" {
            assert!(
                cmd_args.iter().any(|a| a == "compose"),
                "Modern variant should have 'compose' subcommand"
            );
        }
    }

    // ============================================================================
    // Unit Tests for Parse Timeout
    // ============================================================================

    #[test]
    fn test_parse_timeout_seconds_unit() {
        let temp_dir = TempDir::new().unwrap();
        let config = DockerConfig::default();
        let manager = ContainerManager::new(&config, temp_dir.path()).unwrap();

        assert_eq!(manager.parse_timeout("30s").unwrap(), 30);
        assert_eq!(manager.parse_timeout("1s").unwrap(), 1);
        assert_eq!(manager.parse_timeout("120s").unwrap(), 120);
    }

    #[test]
    fn test_parse_timeout_minutes_unit() {
        let temp_dir = TempDir::new().unwrap();
        let config = DockerConfig::default();
        let manager = ContainerManager::new(&config, temp_dir.path()).unwrap();

        assert_eq!(manager.parse_timeout("1m").unwrap(), 60);
        assert_eq!(manager.parse_timeout("2m").unwrap(), 120);
        assert_eq!(manager.parse_timeout("5m").unwrap(), 300);
    }

    #[test]
    fn test_parse_timeout_no_unit() {
        let temp_dir = TempDir::new().unwrap();
        let config = DockerConfig::default();
        let manager = ContainerManager::new(&config, temp_dir.path()).unwrap();

        // Plain numbers default to seconds
        assert_eq!(manager.parse_timeout("30").unwrap(), 30);
        assert_eq!(manager.parse_timeout("90").unwrap(), 90);
        assert_eq!(manager.parse_timeout("300").unwrap(), 300);
    }

    #[test]
    fn test_parse_timeout_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let config = DockerConfig::default();
        let manager = ContainerManager::new(&config, temp_dir.path()).unwrap();

        // Invalid formats should error
        assert!(manager.parse_timeout("invalid").is_err());
        assert!(manager.parse_timeout("30x").is_err());
        assert!(manager.parse_timeout("").is_err());
        assert!(manager.parse_timeout("abc").is_err());
        assert!(manager.parse_timeout("-30").is_err());
    }
}
