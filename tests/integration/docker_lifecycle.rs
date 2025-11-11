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
    // TDD: Test Docker availability check
    // Goal: Verify method returns bool and doesn't crash

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    // Verify method executes successfully
    // Value depends on test environment (Docker installed or not)
    let _is_available = manager.is_docker_available();
    // No assertion - just verify no crash
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
    // TDD: Test listing all containers across worktrees
    // Goal: Get status of all worktree containers

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    // List all containers (currently stub returns empty list)
    let containers = manager.list_all();
    assert!(containers.is_ok());

    // Verify returns Vec (currently empty - stub implementation)
    let list = containers.unwrap();
    assert_eq!(list.len(), 0, "Stub implementation should return empty list");
}

#[test]
fn test_project_name_generation() {
    // TDD: Test Docker Compose project name generation
    // Goal: Generate valid, unique project names for worktrees

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
    assert_eq!(name1, "feature-one"); // Should preserve simple names
    assert_eq!(name2, "feature-two");

    // Test special character sanitization
    let name_special = manager.get_project_name("Feature_Test/Branch");
    assert_eq!(name_special, "feature-test-branch"); // lowercase, no underscores/slashes
    assert!(name_special.chars().all(|c| c.is_alphanumeric() || c == '-'));
    assert!(!name_special.starts_with('-'));
    assert!(!name_special.ends_with('-'));
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
