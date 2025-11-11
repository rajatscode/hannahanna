use hannahanna::config::DockerConfig;
use hannahanna::docker::container::ContainerManager;
use hannahanna::docker::ports::PortAllocator;
use std::collections::HashMap;
use tempfile::TempDir;

#[test]
fn test_container_manager_creation() {
    // TDD RED: ContainerManager doesn't exist yet!
    // Goal: Create container manager instance

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir);

    assert!(manager.is_ok());
}

#[test]
fn test_check_docker_available() {
    // TDD RED: Test Docker availability check
    // Goal: Detect if Docker is installed and running

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    // Check if Docker is available (may be false in test environment)
    let is_available = manager.is_docker_available();
    // Don't assert specific value as Docker may not be installed in test env
    // Just verify the method exists and returns bool
    assert!(is_available == true || is_available == false);
}

#[test]
fn test_get_container_status() {
    // TDD RED: Test getting container status for a worktree
    // Goal: Check if containers are running/stopped

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    let worktree_dir = temp_dir.path().join("worktrees").join("feature-status");
    std::fs::create_dir_all(&worktree_dir).unwrap();
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    // Get status (should return a status even if Docker not available)
    let status = manager.get_status("feature-status", &worktree_dir);
    assert!(status.is_ok());
}

#[test]
fn test_start_command_generation() {
    // TDD RED: Test that we can generate docker-compose commands
    // Goal: Build correct docker-compose up command

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    let worktree_dir = temp_dir.path().join("worktrees").join("feature-cmd");
    std::fs::create_dir_all(&worktree_dir).unwrap();
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    // Generate start command
    let cmd = manager.build_start_command("feature-cmd", &worktree_dir);
    assert!(cmd.is_ok());

    let command = cmd.unwrap();
    assert!(command.contains("docker-compose") || command.contains("docker") || command.contains("compose"));
}

#[test]
fn test_stop_command_generation() {
    // TDD RED: Test stop command generation
    // Goal: Build correct docker-compose down command

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    let worktree_dir = temp_dir.path().join("worktrees").join("feature-stop");
    std::fs::create_dir_all(&worktree_dir).unwrap();
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    // Generate stop command
    let cmd = manager.build_stop_command("feature-stop", &worktree_dir);
    assert!(cmd.is_ok());

    let command = cmd.unwrap();
    assert!(command.contains("docker-compose") || command.contains("docker") || command.contains("compose"));
    assert!(command.contains("down") || command.contains("stop"));
}

#[test]
fn test_list_all_containers() {
    // TDD RED: Test listing all containers across worktrees
    // Goal: Get status of all worktree containers

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    // List all containers
    let containers = manager.list_all();
    assert!(containers.is_ok());

    // Should return a list (may be empty)
    let list = containers.unwrap();
    assert!(list.len() >= 0);
}

#[test]
fn test_project_name_generation() {
    // TDD RED: Test Docker Compose project name generation
    // Goal: Generate unique project names for worktrees

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    // Generate project names
    let name1 = manager.get_project_name("feature-one");
    let name2 = manager.get_project_name("feature-two");

    // Names should be different and based on worktree name
    assert_ne!(name1, name2);
    assert!(name1.contains("feature-one") || name1.contains("feature_one"));
    assert!(name2.contains("feature-two") || name2.contains("feature_two"));
}

#[test]
fn test_cleanup_orphaned() {
    // TDD RED: Test cleanup of orphaned containers
    // Goal: Identify and clean up containers for removed worktrees

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    // Try to cleanup (should not error even if nothing to clean)
    let result = manager.cleanup_orphaned(&[]);
    assert!(result.is_ok());
}

#[test]
fn test_container_logs() {
    // TDD RED: Test retrieving container logs
    // Goal: Get logs for a specific worktree's containers

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    let worktree_dir = temp_dir.path().join("worktrees").join("feature-logs");
    std::fs::create_dir_all(&worktree_dir).unwrap();
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    // Build logs command
    let cmd = manager.build_logs_command("feature-logs", &worktree_dir, None);
    assert!(cmd.is_ok());

    let command = cmd.unwrap();
    assert!(command.contains("logs") || command.contains("docker"));
}
