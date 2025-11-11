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
    assert_eq!(ports_x.get("app"), Some(&3000));
    assert_eq!(ports_x.get("postgres"), Some(&5432));

    // Allocate ports for feature-y (should get next sequential ports)
    let ports_y = allocator
        .allocate("feature-y", &["app", "postgres"])
        .unwrap();
    assert_eq!(ports_y.get("app"), Some(&3001));
    assert_eq!(ports_y.get("postgres"), Some(&5433));

    // Allocate ports for feature-z
    let ports_z = allocator
        .allocate("feature-z", &["app", "postgres"])
        .unwrap();
    assert_eq!(ports_z.get("app"), Some(&3002));
    assert_eq!(ports_z.get("postgres"), Some(&5434));
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
    {
        let mut allocator = PortAllocator::new(&state_dir).unwrap();
        let ports = allocator.allocate("feature-persist", &["app"]).unwrap();
        assert_eq!(ports.get("app"), Some(&3000));
        allocator.save().unwrap();
    }

    // Second allocator instance - should reload persisted state
    {
        let allocator = PortAllocator::new(&state_dir).unwrap();
        let ports = allocator.get_ports("feature-persist").unwrap();
        assert_eq!(ports.get("app"), Some(&3000));
    }
}

#[test]
fn test_port_release_on_remove() {
    // TDD RED: Test that ports are released when worktree is removed
    // Goal: Remove worktree, verify ports available for reuse

    let temp_dir = TempDir::new().unwrap();
    let state_dir = temp_dir.path().join(".wt-state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let mut allocator = PortAllocator::new(&state_dir).unwrap();

    // Allocate ports
    let ports_1 = allocator.allocate("feature-temp", &["app"]).unwrap();
    assert_eq!(ports_1.get("app"), Some(&3000));

    // Release ports
    allocator.release("feature-temp").unwrap();

    // Allocate again - should reuse the port
    let ports_2 = allocator.allocate("feature-new", &["app"]).unwrap();
    assert_eq!(ports_2.get("app"), Some(&3000));
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
