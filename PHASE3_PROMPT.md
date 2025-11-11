# Phase 3: Docker Integration - Production Hardening & Missing Features

## Context

Phase 2 Docker Integration is complete and passing all 102 tests. The TDD implementation includes:
- ✅ Port allocation system with O(1) lookups and transaction semantics
- ✅ Docker Compose override generation with template substitution
- ✅ Container lifecycle management (start/stop/status)
- ✅ CLI commands (docker ps/start/stop/logs/prune, ports list/show/release)
- ✅ Integration with core commands (add/remove/info)

**However**, several features are incomplete stubs or missing entirely. Your task is to complete the remaining items using **strict TDD methodology**.

## Critical Items (Must Fix)

### 1. cleanup_orphaned() is a STUB ⚠️
**Location**: `src/docker/container.rs:157-170`
**Current**: Returns `Ok(())` - does nothing!
**Required**: Actually scan and clean up orphaned containers

**TDD Approach**:
```rust
// RED: Write failing test first
#[test]
fn test_cleanup_actually_removes_orphans() {
    // Setup: Create containers for "feature-x", "feature-y"
    // Remove worktree "feature-y" from active list
    // Call cleanup_orphaned(&["feature-x"])
    // Assert: Only feature-x containers remain
}
```

**Implementation Requirements**:
- List all docker-compose projects with label/prefix pattern
- Identify projects not in active_worktrees list
- Call `docker-compose -p <project> down` for each orphan
- Handle errors gracefully (some containers might already be gone)
- Return list of cleaned up projects

### 2. System-Level Port Conflict Detection
**Problem**: We only track our own allocations. If another process is using port 3000, we'll allocate it anyway!

**TDD Approach**:
```rust
#[test]
fn test_port_allocation_skips_in_use_ports() {
    // Start a test server on port 3000
    let _server = bind("127.0.0.1:3000");

    // Try to allocate - should skip to 3001
    let ports = allocator.allocate("test", &["app"]).unwrap();
    assert_eq!(ports["app"], 3001);
}
```

**Implementation**:
- Before marking port as available, check if it's bindable
- Use `std::net::TcpListener::bind("127.0.0.1:{port}")` to test
- Skip to next port if bind fails
- Add to `allocate_port_for_service()`

### 3. File Locking for Concurrent Access
**Problem**: Two `hn add` commands at once could corrupt port-registry.yaml

**TDD Approach**:
```rust
#[test]
fn test_concurrent_port_allocation() {
    // Spawn 10 threads all trying to allocate ports
    // Each should get unique ports
    // Registry should be consistent
}
```

**Implementation**:
- Add file locking using `fs2` crate (add to Cargo.toml)
- Lock before load, unlock after save
- Use `FileExt::try_lock_exclusive()` on registry file
- Handle lock contention with retry/backoff

### 4. Command Injection Security Fix
**Problem**: Uses `sh -c` with string interpolation - vulnerable to injection

**TDD Approach**:
```rust
#[test]
fn test_no_command_injection_in_project_name() {
    let malicious = "test; rm -rf /";
    let sanitized = manager.get_project_name(malicious);

    // Should be safe: test-rm-rf
    assert!(!sanitized.contains(";"));

    // Verify start command is safe
    let cmd = manager.build_start_command(malicious, path)?;
    // Should not contain unescaped shell metacharacters
}
```

**Implementation**:
- Replace `execute_command()` string-based approach
- Use `Command::new("docker-compose")` with separate args
- Build args as Vec<String>, don't interpolate
- Never use `sh -c` for docker commands

### 5. docker-compose Version Detection
**Problem**: Assumes `docker-compose` command exists. Newer Docker uses `docker compose` (no hyphen!)

**TDD Approach**:
```rust
#[test]
fn test_detect_compose_command() {
    let manager = ContainerManager::new(...)?;
    let cmd = manager.get_compose_command(); // "docker compose" or "docker-compose"
    assert!(cmd == "docker compose" || cmd == "docker-compose");
}
```

**Implementation**:
- On first Docker operation, detect which command works
- Cache result in ContainerManager
- Try `docker compose version` first (new)
- Fall back to `docker-compose --version` (old)
- Update all command building to use detected command

## Important Items (Should Implement)

### 6. Health Check Implementation
**Current**: `HealthCheckConfig` exists but unused
**Goal**: Actually check container health

**TDD Test**:
```rust
#[test]
fn test_health_check_integration() {
    let config = DockerConfig {
        health_check: Some(HealthCheckConfig {
            enabled: true,
            endpoint: "/health".to_string(),
            interval_seconds: 5,
            timeout_seconds: 2,
        }),
        ..Default::default()
    };

    let manager = ContainerManager::new(&config, ...)?;
    manager.start("test-wt", path)?;

    // Wait for container to be healthy
    std::thread::sleep(Duration::from_secs(10));

    let health = manager.check_health("test-wt", path)?;
    assert!(health.is_healthy);
}
```

### 7. Actual container_count
**Problem**: Hardcoded to 0 or 1
**Goal**: Parse `docker-compose ps` output and count services

### 8. Registry Versioning
**Add** to `PortRegistry`:
```rust
struct PortRegistry {
    version: u32,  // Start at 1
    allocations: HashMap<...>,
    next_available: HashMap<...>,
}
```

Implement migration logic in `load_registry()`.

### 9. Corrupted Registry Recovery
**Test**:
```rust
#[test]
fn test_recover_from_corrupted_registry() {
    // Write garbage to port-registry.yaml
    fs::write(registry_path, "}{invalid yaml!@#")?;

    // Should recover gracefully
    let allocator = PortAllocator::new(state_dir)?;
    assert!(allocator.list_all().is_empty());
}
```

### 10. Volume Management Commands
Add to `src/cli/docker.rs`:
- `hn docker volumes` - list all volumes
- `hn docker volume prune` - remove unused volumes

### 11. Validate docker-compose.yml Exists
Before starting containers, check if compose file exists in worktree.

## Nice-to-Have Items

### 12. docker exec Support
```rust
pub fn exec(&self, worktree: &str, service: &str, command: &[&str]) -> Result<String>
```

### 13. Streaming Logs
Integrate `build_logs_command()` to actually stream output.

### 14. Integration Tests with Real Docker
Add `tests/docker_real.rs` with `#[ignore]` by default:
```rust
#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_full_docker_lifecycle_real() {
    // Actually run docker-compose up
    // Check container is running
    // Stop container
    // Verify cleanup
}
```

### 15. Port Utilization Stats
```rust
pub fn get_stats(&self) -> PortStats {
    PortStats {
        total_allocated: usize,
        ports_by_service: HashMap<String, usize>,
        most_used_port: u16,
        ...
    }
}
```

## Specification Gaps Not Yet Addressed

From `spec/plan.md`, these items were listed but not fully implemented:

1. **Isolated networks** - Config only supports shared networks
2. **docker-compose.override.yml validation** - We generate but never validate
3. **Worktree path validation in build commands** - `_worktree_path` is unused
4. **Integration with existing docker-compose.yml** - Doesn't parse services from base file
5. **Error recovery tests** - No tests for Docker daemon down scenarios

## Instructions for Next Claude

1. **Follow TDD Strictly**:
   - RED: Write failing test FIRST
   - GREEN: Implement minimum code to pass
   - REFACTOR: Clean up while keeping tests green

2. **Prioritize Critical Items**:
   - Start with cleanup_orphaned() - it's called but does nothing!
   - Then port conflict detection
   - Then file locking

3. **Security First**:
   - Fix command injection vulnerability early
   - Never use `sh -c` with user-controlled input

4. **Maintain Test Coverage**:
   - All new code must have tests
   - Aim for >90% coverage on new code

5. **Commit Frequently**:
   - Commit after each feature is green
   - Clear commit messages explaining what was implemented

## Current Test Status

```
✅ 102 tests passing
✅ Clippy clean
✅ No compiler warnings
✅ All critical fixes from Sanjay's review applied
```

## Files to Focus On

- `src/docker/container.rs` - cleanup_orphaned(), command injection, version detection
- `src/docker/ports.rs` - system port conflict detection, file locking
- `src/docker/compose.rs` - validation
- `tests/integration/docker_lifecycle.rs` - add real Docker tests

## Don't Forget

- Run `cargo fmt` and `cargo clippy` before committing
- All tests must pass: `cargo test --all`
- Push to branch: `claude/docker-integration-tdd-phase3-<session-id>`

Good luck! Remember: **TDD is non-negotiable**. Write the failing test FIRST, every time.
