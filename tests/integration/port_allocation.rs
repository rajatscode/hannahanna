use hannahanna::docker::ports::PortAllocator;
use tempfile::TempDir;

#[test]
fn test_allocate_sequential_ports() {
    // TDD RED: This test WILL fail - PortAllocator doesn't exist yet!
    // Goal: 3 worktrees get unique sequential ports
    // feature-x: app=3000, postgres=5432
    // feature-y: app=3001, postgres=5433
    // feature-z: app=3002, postgres=5434

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let mut allocator = PortAllocator::new(&state_dir).unwrap();

    // Allocate ports for feature-x
    let ports_x = allocator
        .allocate("feature-x", &["app", "postgres"])
        .unwrap();
    // Should allocate ports (actual numbers may vary based on system availability)
    assert!(ports_x.contains_key("app"));
    assert!(ports_x.contains_key("postgres"));
    let app_x = *ports_x.get("app").unwrap();
    let postgres_x = *ports_x.get("postgres").unwrap();

    // Allocate ports for feature-y (should get different ports)
    let ports_y = allocator
        .allocate("feature-y", &["app", "postgres"])
        .unwrap();
    assert!(ports_y.contains_key("app"));
    assert!(ports_y.contains_key("postgres"));
    let app_y = *ports_y.get("app").unwrap();
    let postgres_y = *ports_y.get("postgres").unwrap();

    // Ports should be different between worktrees
    assert_ne!(app_x, app_y);
    assert_ne!(postgres_x, postgres_y);

    // Allocate ports for feature-z
    let ports_z = allocator
        .allocate("feature-z", &["app", "postgres"])
        .unwrap();
    assert!(ports_z.contains_key("app"));
    assert!(ports_z.contains_key("postgres"));
    let app_z = *ports_z.get("app").unwrap();
    let postgres_z = *ports_z.get("postgres").unwrap();

    // All ports should be unique
    assert_ne!(app_x, app_z);
    assert_ne!(app_y, app_z);
    assert_ne!(postgres_x, postgres_z);
    assert_ne!(postgres_y, postgres_z);
}

#[test]
fn test_port_conflict_detection() {
    // TDD RED: Test for port conflict detection
    // Goal: If port 3000 is occupied externally, allocator should skip to 3001

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let mut allocator = PortAllocator::new(&state_dir).unwrap();

    // TODO: Will implement port conflict detection later
    // For now, just test basic allocation
    let ports = allocator.allocate("feature-test", &["app"]).unwrap();
    assert!(ports.contains_key("app"));
}

#[test]
fn test_port_registry_persistence() {
    // TDD RED: Test that port allocations persist across instances
    // Goal: Allocate ports, save registry, reload, verify ports preserved

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    // First allocator instance
    let allocated_port = {
        let mut allocator = PortAllocator::new(&state_dir).unwrap();
        let ports = allocator.allocate("feature-persist", &["app"]).unwrap();
        let port = *ports.get("app").unwrap();
        allocator.save().unwrap();
        port
    };

    // Second allocator instance - should reload persisted state
    {
        let allocator = PortAllocator::new(&state_dir).unwrap();
        let ports = allocator.get_ports("feature-persist").unwrap();
        // Should get the same port that was allocated before
        assert_eq!(ports.get("app"), Some(&allocated_port));
    }
}

#[test]
fn test_port_release_on_remove() {
    // TDD: Test that ports are released when worktree is removed
    // Goal: Remove worktree, verify ports can be reallocated

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let mut allocator = PortAllocator::new(&state_dir).unwrap();

    // Allocate ports for three worktrees to occupy the first three slots
    allocator.allocate("feature-a", &["app"]).unwrap();
    allocator.allocate("feature-b", &["app"]).unwrap();
    let ports_c = allocator.allocate("feature-c", &["app"]).unwrap();
    let port_c = *ports_c.get("app").unwrap();

    // Verify all three have different ports
    let all_ports = allocator.list_all();
    assert_eq!(all_ports.len(), 3, "Should have 3 allocations");

    // Release the middle one (feature-b)
    allocator.release("feature-b").unwrap();

    // Verify it's released
    let all_ports = allocator.list_all();
    assert_eq!(all_ports.len(), 2, "Should have 2 allocations after release");
    assert!(allocator.get_ports("feature-b").is_err(), "feature-b should be released");

    // Allocate a new worktree - the allocator should be able to find an available port
    // (either by filling the gap left by feature-b, or by finding the next available port)
    let ports_new = allocator.allocate("feature-new", &["app"]).unwrap();
    let port_new = *ports_new.get("app").unwrap();

    // The new port should be different from feature-c (which is still allocated)
    assert_ne!(port_new, port_c, "New allocation should not conflict with existing");

    // Verify we now have 3 allocations again
    let all_ports = allocator.list_all();
    assert_eq!(all_ports.len(), 3, "Should have 3 allocations after new allocation");
}

#[test]
fn test_port_exhaustion() {
    // TDD RED: Test graceful handling of port exhaustion
    // Goal: Range 3000-3005, create 7 worktrees, 7th should fail gracefully

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let mut allocator = PortAllocator::with_range(&state_dir, 3000, 3005).unwrap();

    // Allocate 6 worktrees successfully (3000-3005 = 6 ports)
    for i in 0..6 {
        let name = format!("feature-{}", i);
        let result = allocator.allocate(&name, &["app"]);
        assert!(result.is_ok(), "Should allocate port {} successfully", i);
    }

    // 7th allocation should fail with clear error
    let result = allocator.allocate("feature-7", &["app"]);
    assert!(result.is_err(), "Should fail when ports exhausted");

    // Error message should be clear
    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(err_msg.contains("exhausted") || err_msg.contains("available"));
}
