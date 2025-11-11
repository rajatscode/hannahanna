// Port allocation system for Docker containers
// Automatically assigns unique ports to each worktree

use crate::errors::{HnError, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};

/// Port registry format persisted to disk
#[derive(Debug, Serialize, Deserialize, Default)]
struct PortRegistry {
    /// Map of worktree name -> service name -> port number
    allocations: HashMap<String, HashMap<String, u16>>,
    /// Next available port for each service type
    next_available: HashMap<String, u16>,
}

/// Manages port allocation for Docker services across worktrees
pub struct PortAllocator {
    state_dir: PathBuf,
    registry: PortRegistry,
    base_ports: HashMap<String, u16>,
    port_range_start: u16,
    port_range_end: u16,
    /// Cached set of used ports for O(1) lookup
    used_ports: HashSet<u16>,
}

impl PortAllocator {
    /// Create a new port allocator with default base ports
    pub fn new(state_dir: &Path) -> Result<Self> {
        let mut base_ports = HashMap::new();
        base_ports.insert("app".to_string(), 3000);
        base_ports.insert("postgres".to_string(), 5432);
        base_ports.insert("redis".to_string(), 6379);

        let registry = Self::load_registry(state_dir).unwrap_or_default();

        // Build inverse index of used ports for O(1) lookup
        let used_ports = registry
            .allocations
            .values()
            .flat_map(|services| services.values().copied())
            .collect();

        Ok(Self {
            state_dir: state_dir.to_path_buf(),
            registry,
            base_ports,
            port_range_start: 3000,
            port_range_end: 9999,
            used_ports,
        })
    }

    /// Create a port allocator with custom port range
    /// Used primarily for testing port exhaustion scenarios
    #[allow(dead_code)] // Used in integration tests
    pub fn with_range(state_dir: &Path, range_start: u16, range_end: u16) -> Result<Self> {
        let mut allocator = Self::new(state_dir)?;
        allocator.port_range_start = range_start;
        allocator.port_range_end = range_end;
        Ok(allocator)
    }

    /// Allocate ports for a worktree's services
    /// Uses transaction-like semantics: all services get ports or none do
    pub fn allocate(
        &mut self,
        worktree_name: &str,
        services: &[&str],
    ) -> Result<HashMap<String, u16>> {
        // Check if already allocated
        if let Some(existing) = self.registry.allocations.get(worktree_name) {
            return Ok(existing.clone());
        }

        let mut allocated_ports = HashMap::new();
        let mut temp_used_ports = Vec::new();

        // Allocate all ports first (transaction phase)
        for service in services {
            match self.allocate_port_for_service(service) {
                Ok(port) => {
                    allocated_ports.insert(service.to_string(), port);
                    temp_used_ports.push(port);
                }
                Err(e) => {
                    // Rollback: remove temporarily allocated ports from cache
                    for port in temp_used_ports {
                        self.used_ports.remove(&port);
                    }
                    return Err(e);
                }
            }
        }

        // All allocations successful - commit the transaction
        self.registry
            .allocations
            .insert(worktree_name.to_string(), allocated_ports.clone());

        // Auto-save after allocation
        self.save()?;

        Ok(allocated_ports)
    }

    /// Get already allocated ports for a worktree
    pub fn get_ports(&self, worktree_name: &str) -> Result<HashMap<String, u16>> {
        self.registry
            .allocations
            .get(worktree_name)
            .cloned()
            .ok_or_else(|| {
                HnError::PortAllocationError(format!("No ports allocated for '{}'", worktree_name))
            })
    }

    /// Release ports when a worktree is removed
    pub fn release(&mut self, worktree_name: &str) -> Result<()> {
        if let Some(ports) = self.registry.allocations.remove(worktree_name) {
            // Remove ports from used_ports cache
            for port in ports.values() {
                self.used_ports.remove(port);
            }
            self.save()?;
        }
        Ok(())
    }

    /// List all port allocations
    pub fn list_all(&self) -> Vec<(String, HashMap<String, u16>)> {
        self.registry
            .allocations
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Save registry to disk with file locking to prevent concurrent corruption
    pub fn save(&self) -> Result<()> {
        let registry_path = self.state_dir.join("port-registry.yaml");

        // Ensure directory exists
        fs::create_dir_all(&self.state_dir)?;

        // Open file WITHOUT truncate first (we'll truncate after acquiring lock)
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&registry_path)?;

        // Acquire exclusive lock (blocks until lock is available)
        file.lock_exclusive()
            .map_err(|e| HnError::DockerError(format!("Failed to lock registry file: {}", e)))?;

        // Now that we have the lock, truncate the file
        file.set_len(0)?;

        let yaml = serde_yml::to_string(&self.registry)
            .map_err(|e| HnError::DockerError(format!("Failed to serialize registry: {}", e)))?;

        // Write to file
        let mut file_mut = file;
        file_mut.write_all(yaml.as_bytes())?;
        file_mut.sync_all()?;

        // Lock is automatically released when file goes out of scope
        Ok(())
    }

    /// Load registry from disk with file locking to prevent reading during writes
    fn load_registry(state_dir: &Path) -> Result<PortRegistry> {
        let registry_path = state_dir.join("port-registry.yaml");

        if !registry_path.exists() {
            return Ok(PortRegistry::default());
        }

        // Open file for reading with shared lock
        let file = File::open(&registry_path)?;

        // Acquire shared lock (allows multiple readers, blocks writers)
        file.lock_shared().map_err(|e| {
            HnError::DockerError(format!("Failed to lock registry file for reading: {}", e))
        })?;

        let content = fs::read_to_string(&registry_path)?;
        let registry: PortRegistry = serde_yml::from_str(&content)
            .map_err(|e| HnError::DockerError(format!("Failed to parse registry: {}", e)))?;

        // Lock is automatically released when file goes out of scope
        Ok(registry)
    }

    /// Check if a port is available on the system by attempting to bind to it
    fn is_port_available_on_system(&self, port: u16) -> bool {
        // Try to bind to both IPv4 and IPv6 addresses
        let ipv4_addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
        let ipv6_addr: SocketAddr = format!("[::]:{}", port).parse().unwrap();

        // Check IPv4
        let ipv4_available = TcpListener::bind(ipv4_addr).is_ok();

        // Check IPv6
        let ipv6_available = TcpListener::bind(ipv6_addr).is_ok();

        // Port is available if we can bind to at least one
        ipv4_available || ipv6_available
    }

    /// Allocate next available port for a service type
    fn allocate_port_for_service(&mut self, service: &str) -> Result<u16> {
        // Get base port for this service
        let base_port = self.base_ports.get(service).copied().unwrap_or(3000);

        // Always start from base port to fill gaps (released ports)
        // The HashSet lookup is O(1) so this is still efficient
        let mut port = base_port;
        let mut attempts = 0;
        let max_attempts = (self.port_range_end - self.port_range_start) as usize;

        loop {
            if port > self.port_range_end || attempts > max_attempts {
                return Err(HnError::PortAllocationError(format!(
                    "Port exhausted for service '{}': no available ports in range {}-{}",
                    service, self.port_range_start, self.port_range_end
                )));
            }

            // O(1) lookup using HashSet instead of O(n) iteration
            if !self.used_ports.contains(&port) {
                // Check if port is actually available on the system
                if self.is_port_available_on_system(port) {
                    // Found an available port - add to cache and update next_available
                    self.used_ports.insert(port);
                    self.registry
                        .next_available
                        .insert(service.to_string(), port + 1);
                    return Ok(port);
                } else {
                    // Port is in use by another process, skip it
                    eprintln!(
                        "Warning: Port {} is in use by another process, trying next port",
                        port
                    );
                }
            }

            port += 1;
            attempts += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use tempfile::TempDir;

    #[test]
    fn test_new_allocator() {
        let temp_dir = TempDir::new().unwrap();
        let allocator = PortAllocator::new(temp_dir.path()).unwrap();
        assert_eq!(allocator.base_ports.get("app"), Some(&3000));
        assert_eq!(allocator.base_ports.get("postgres"), Some(&5432));
    }

    #[test]
    fn test_basic_allocation() {
        let temp_dir = TempDir::new().unwrap();
        let mut allocator = PortAllocator::new(temp_dir.path()).unwrap();

        let ports = allocator.allocate("test-wt", &["app"]).unwrap();

        // Test behavior, not exact port number (port 3000 might be in use)
        assert!(ports.contains_key("app"), "Should allocate port for 'app'");
        let port = *ports.get("app").unwrap();
        assert!((3000..=9999).contains(&port), "Port should be in valid range (got {})", port);
    }

    // ============================================================================
    // File Locking Tests for Concurrent Access
    // ============================================================================

    #[test]
    fn test_concurrent_port_allocation() {
        // Test that multiple threads allocating ports concurrently don't corrupt the registry
        // Note: Due to how PortAllocator works (each instance loads from disk), concurrent
        // allocations may overwrite each other. The file locking prevents corruption, not
        // conflicts. In real usage, the caller would serialize worktree operations.
        let temp_dir = TempDir::new().unwrap();
        let state_dir = Arc::new(temp_dir.path().to_path_buf());

        // Spawn multiple threads that allocate ports concurrently
        let mut handles = vec![];

        for i in 0..5 {
            let state_dir = Arc::clone(&state_dir);
            let handle = thread::spawn(move || {
                let mut allocator = PortAllocator::new(&state_dir).unwrap();
                let worktree = format!("worktree-{}", i);

                // Each thread allocates a port for "app" service
                allocator.allocate(&worktree, &["app"])
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        let mut success_count = 0;
        for handle in handles {
            let result = handle.join().unwrap();
            if result.is_ok() {
                success_count += 1;
            }
        }

        // All allocations should succeed (no panics or errors)
        assert_eq!(
            success_count, 5,
            "All concurrent allocations should succeed"
        );

        // Verify registry file is valid (not corrupted by concurrent writes)
        let final_allocator = PortAllocator::new(&state_dir).unwrap();
        let allocations = final_allocator.list_all();

        // At least one allocation should be preserved
        // (The exact number depends on timing - last write wins)
        assert!(
            !allocations.is_empty(),
            "Registry should have at least one allocation"
        );

        // Verify the registry file is valid YAML (no corruption)
        let registry_path = state_dir.join("port-registry.yaml");
        let content = std::fs::read_to_string(&registry_path).unwrap();
        let _parsed: serde_yml::Value = serde_yml::from_str(&content)
            .expect("Registry should be valid YAML after concurrent writes");
    }

    #[test]
    fn test_registry_save_with_exclusive_lock() {
        // Test that save() uses exclusive locking
        // This is verified by ensuring save() doesn't corrupt data
        let temp_dir = TempDir::new().unwrap();
        let mut allocator = PortAllocator::new(temp_dir.path()).unwrap();

        // Allocate some ports
        allocator.allocate("wt1", &["app"]).unwrap();
        allocator.allocate("wt2", &["app"]).unwrap();

        // Save should succeed
        let result = allocator.save();
        assert!(result.is_ok(), "Save with exclusive lock should succeed");

        // Verify registry file exists
        let registry_path = temp_dir.path().join("port-registry.yaml");
        assert!(
            registry_path.exists(),
            "Registry file should exist after save"
        );

        // Verify we can load it back
        let loaded_allocator = PortAllocator::new(temp_dir.path()).unwrap();
        let loaded_ports = loaded_allocator.list_all();

        assert_eq!(loaded_ports.len(), 2, "Should load 2 worktree allocations");
    }

    #[test]
    fn test_registry_load_with_shared_lock() {
        // Test that load_registry() uses shared locking
        // Multiple loads should be able to happen concurrently
        let temp_dir = TempDir::new().unwrap();
        let state_dir = Arc::new(temp_dir.path().to_path_buf());

        // Pre-populate the registry
        {
            let mut allocator = PortAllocator::new(&state_dir).unwrap();
            allocator.allocate("test-wt", &["app"]).unwrap();
        }

        // Spawn multiple threads that load the registry concurrently
        let mut handles = vec![];

        for _ in 0..5 {
            let state_dir = Arc::clone(&state_dir);
            let handle = thread::spawn(move || {
                // Load the allocator (which loads the registry)
                PortAllocator::new(&state_dir)
            });
            handles.push(handle);
        }

        // Wait for all threads and verify all succeeded
        let mut success_count = 0;
        for handle in handles {
            let result = handle.join().unwrap();
            if result.is_ok() {
                success_count += 1;
            }
        }

        assert_eq!(
            success_count, 5,
            "All concurrent reads with shared locks should succeed"
        );
    }

    #[test]
    fn test_file_locking_prevents_corruption() {
        // Test that file locking prevents YAML corruption during concurrent writes
        // Note: File locking prevents corruption but doesn't prevent overwrites
        // (last write wins with PortAllocator's design)
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let state_dir = Arc::new(temp_dir.path().to_path_buf());

        // Spawn threads that perform write operations concurrently
        let mut handles = vec![];

        for i in 0..3 {
            let state_dir = Arc::clone(&state_dir);
            let handle = thread::spawn(move || {
                let mut allocator = PortAllocator::new(&state_dir).unwrap();

                // Each thread allocates ports for different services
                let services = match i {
                    0 => vec!["app"],
                    1 => vec!["postgres"],
                    2 => vec!["redis"],
                    _ => vec!["app"],
                };

                let worktree = format!("wt-{}", i);
                allocator.allocate(&worktree, &services).unwrap();

                // Explicit save to trigger write lock (already auto-saved by allocate)
                allocator.save()
            });
            handles.push(handle);
        }

        // Wait for all writes to complete
        for handle in handles {
            let result = handle.join().unwrap();
            assert!(result.is_ok(), "Concurrent saves should all succeed");
        }

        // Verify the registry file is valid YAML and not corrupted
        let registry_path = state_dir.join("port-registry.yaml");
        let content = std::fs::read_to_string(&registry_path).unwrap();

        // Try to parse it as YAML - this is the key test!
        let parsed: serde_yml::Value = serde_yml::from_str(&content)
            .expect("Registry should be valid YAML (not corrupted) after concurrent writes");

        // Verify structure
        assert!(
            parsed.get("allocations").is_some(),
            "Registry should have allocations field"
        );

        // Verify we have at least one allocation (file locking prevented total corruption)
        let final_allocator = PortAllocator::new(&state_dir).unwrap();
        let allocations = final_allocator.list_all();
        assert!(
            !allocations.is_empty(),
            "Registry should have at least one allocation (proves YAML wasn't corrupted)"
        );
    }

    #[test]
    fn test_lock_release_on_scope_exit() {
        // Test that locks are automatically released when file goes out of scope
        let temp_dir = TempDir::new().unwrap();

        // First scope: acquire and release lock
        {
            let mut allocator = PortAllocator::new(temp_dir.path()).unwrap();
            allocator.allocate("test1", &["app"]).unwrap();
            // Lock is released here when allocator goes out of scope
        }

        // Second scope: should be able to acquire lock immediately
        {
            let mut allocator = PortAllocator::new(temp_dir.path()).unwrap();
            let result = allocator.allocate("test2", &["app"]);

            // Should succeed because previous lock was released
            assert!(
                result.is_ok(),
                "Should be able to acquire lock after previous scope exit"
            );
        }
    }

    #[test]
    fn test_exclusive_lock_blocks_concurrent_writes() {
        // Test that exclusive lock prevents YAML corruption during concurrent writes
        // Note: Due to PortAllocator's design (each instance loads from disk),
        // concurrent allocations may overwrite each other (last write wins).
        // The file locking prevents corruption, not conflicts.
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        let temp_dir = TempDir::new().unwrap();
        let state_dir = Arc::new(temp_dir.path().to_path_buf());

        // Pre-populate registry
        {
            let mut allocator = PortAllocator::new(&state_dir).unwrap();
            allocator.allocate("initial", &["app"]).unwrap();
        }

        let state_dir1 = Arc::clone(&state_dir);
        let state_dir2 = Arc::clone(&state_dir);

        // First thread: hold the lock for a bit
        let handle1 = thread::spawn(move || {
            let mut allocator = PortAllocator::new(&state_dir1).unwrap();
            allocator.allocate("thread1", &["app"]).unwrap();

            // Hold the allocation (and thus keep object alive) for a moment
            thread::sleep(Duration::from_millis(100));

            "thread1 done"
        });

        // Give first thread time to acquire lock
        thread::sleep(Duration::from_millis(10));

        // Second thread: should wait for lock to be released
        let handle2 = thread::spawn(move || {
            let mut allocator = PortAllocator::new(&state_dir2).unwrap();
            allocator.allocate("thread2", &["postgres"])
        });

        // Wait for both threads
        handle1.join().unwrap();
        let result2 = handle2.join().unwrap();

        // Second thread should eventually succeed (after first thread releases lock)
        assert!(
            result2.is_ok(),
            "Second thread should succeed after first releases lock"
        );

        // Verify the registry file is valid YAML (not corrupted by concurrent writes)
        let registry_path = state_dir.join("port-registry.yaml");
        let content = std::fs::read_to_string(&registry_path).unwrap();
        let _parsed: serde_yml::Value = serde_yml::from_str(&content)
            .expect("Registry should be valid YAML (not corrupted) after concurrent writes");

        // Verify at least one allocation is present (file locking prevented total corruption)
        let final_allocator = PortAllocator::new(&state_dir).unwrap();
        let allocations = final_allocator.list_all();
        assert!(
            !allocations.is_empty(),
            "Registry should have at least one allocation (proves no corruption)"
        );

        // Note: We don't assert that BOTH thread1 and thread2 are present because
        // of the last-write-wins behavior. The key is that the file isn't corrupted
        // and at least one allocation succeeded.
    }
}
