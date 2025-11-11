use hannahanna::config::DockerConfig;
use hannahanna::docker::container::ContainerManager;
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
    assert!(name_special
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-'));
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
fn test_get_logs_command() {
    // Test safe logs command generation
    // Goal: Get secure command arguments for viewing logs

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    // Get logs command (safe from injection)
    let result = manager.get_logs_command("feature-logs", None);
    assert!(result.is_ok());

    let (program, args) = result.unwrap();
    // Should be either "docker" or "docker-compose"
    assert!(program == "docker" || program == "docker-compose");
    // Args should contain logs and project name
    assert!(args.iter().any(|a| a == "logs"));
    assert!(args.iter().any(|a| a.contains("feature-logs")));
}
