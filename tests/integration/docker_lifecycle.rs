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

// ============================================================================
// Health Check and Timeout Tests
// ============================================================================

#[test]
fn test_parse_timeout_seconds() {
    // Test parsing timeout with 's' suffix
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    let result = manager.parse_timeout("30s");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 30);

    let result = manager.parse_timeout("1s");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);

    let result = manager.parse_timeout("120s");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 120);
}

#[test]
fn test_parse_timeout_minutes() {
    // Test parsing timeout with 'm' suffix
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    let result = manager.parse_timeout("1m");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 60);

    let result = manager.parse_timeout("2m");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 120);

    let result = manager.parse_timeout("5m");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 300);
}

#[test]
fn test_parse_timeout_plain_number() {
    // Test parsing plain numbers (assumed to be seconds)
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    let result = manager.parse_timeout("30");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 30);

    let result = manager.parse_timeout("90");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 90);

    let result = manager.parse_timeout("300");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 300);
}

#[test]
fn test_parse_timeout_invalid_format() {
    // Test error handling for invalid formats
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    let invalid_timeouts = vec![
        "invalid",
        "30x",        // Invalid unit
        "abc",        // Not a number
        "",           // Empty string
        "30h",        // Hours not supported
        "1.5m",       // Decimals not supported (if applicable)
        "30 s",       // Space not allowed
        "-30s",       // Negative not allowed
        "s30",        // Unit before number
    ];

    for timeout_str in invalid_timeouts {
        let result = manager.parse_timeout(timeout_str);
        assert!(
            result.is_err(),
            "Should reject invalid timeout format: '{}'",
            timeout_str
        );
    }
}

#[test]
fn test_parse_timeout_edge_cases() {
    // Test boundary conditions
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    // Zero is edge case - may or may not be valid
    let result = manager.parse_timeout("0");
    // If it succeeds, verify it's 0 seconds
    if result.is_ok() {
        assert_eq!(result.unwrap(), 0);
    }

    // Very large timeout
    let result = manager.parse_timeout("86400s"); // 24 hours in seconds
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 86400);
}

#[test]
fn test_health_check_manager_creation() {
    // Test that ContainerManager can be created with default config
    // which includes health check settings
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir);

    assert!(manager.is_ok(), "Manager should be created with default health check config");
}

// ============================================================================
// Docker Compose Variant Detection Tests
// ============================================================================

#[test]
fn test_docker_compose_variant_detection() {
    // Test that Docker Compose variant detection works
    // This is primarily a compilation test as actual detection requires Docker
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir);

    assert!(manager.is_ok());

    // The actual variant detection happens internally
    // We can't easily test it without Docker, but we verify the code compiles
    // and the manager can be created
}

#[test]
fn test_logs_command_uses_detected_variant() {
    // Test that get_logs_command uses the detected Docker Compose variant
    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let config = DockerConfig::default();
    let manager = ContainerManager::new(&config, &state_dir).unwrap();

    let result = manager.get_logs_command("test-worktree", None);

    if let Ok((program, args)) = result {
        // Should use either modern "docker" or legacy "docker-compose"
        assert!(
            program == "docker" || program == "docker-compose",
            "Program should be 'docker' or 'docker-compose', got: {}",
            program
        );

        // If using modern variant, args should contain "compose" subcommand
        if program == "docker" {
            assert!(
                args.iter().any(|a| a == "compose"),
                "Modern variant should have 'compose' subcommand in args"
            );
        }
    }
}
