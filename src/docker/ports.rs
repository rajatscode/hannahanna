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

        // Open file for writing with exclusive lock
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&registry_path)?;

        // Acquire exclusive lock (blocks until lock is available)
        file.lock_exclusive()
            .map_err(|e| HnError::DockerError(format!("Failed to lock registry file: {}", e)))?;

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
        assert_eq!(ports.get("app"), Some(&3000));
    }
}
