// Port allocation system for Docker containers
// Automatically assigns unique ports to each worktree

use crate::errors::{HnError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
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
}

impl PortAllocator {
    /// Create a new port allocator with default base ports
    pub fn new(state_dir: &Path) -> Result<Self> {
        let mut base_ports = HashMap::new();
        base_ports.insert("app".to_string(), 3000);
        base_ports.insert("postgres".to_string(), 5432);
        base_ports.insert("redis".to_string(), 6379);

        let registry = Self::load_registry(state_dir).unwrap_or_default();

        Ok(Self {
            state_dir: state_dir.to_path_buf(),
            registry,
            base_ports,
            port_range_start: 3000,
            port_range_end: 9999,
        })
    }

    /// Create a port allocator with custom port range
    pub fn with_range(state_dir: &Path, range_start: u16, range_end: u16) -> Result<Self> {
        let mut allocator = Self::new(state_dir)?;
        allocator.port_range_start = range_start;
        allocator.port_range_end = range_end;
        Ok(allocator)
    }

    /// Allocate ports for a worktree's services
    pub fn allocate(&mut self, worktree_name: &str, services: &[&str]) -> Result<HashMap<String, u16>> {
        // Check if already allocated
        if let Some(existing) = self.registry.allocations.get(worktree_name) {
            return Ok(existing.clone());
        }

        let mut allocated_ports = HashMap::new();

        for service in services {
            let port = self.allocate_port_for_service(service)?;
            allocated_ports.insert(service.to_string(), port);
        }

        // Store allocation
        self.registry.allocations.insert(worktree_name.to_string(), allocated_ports.clone());

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
            .ok_or_else(|| HnError::PortAllocationError(format!("No ports allocated for '{}'", worktree_name)))
    }

    /// Release ports when a worktree is removed
    pub fn release(&mut self, worktree_name: &str) -> Result<()> {
        if self.registry.allocations.remove(worktree_name).is_some() {
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

    /// Save registry to disk
    pub fn save(&self) -> Result<()> {
        let registry_path = self.state_dir.join("port-registry.yaml");

        // Ensure directory exists
        fs::create_dir_all(&self.state_dir)?;

        let yaml = serde_yml::to_string(&self.registry)
            .map_err(|e| HnError::DockerError(format!("Failed to serialize registry: {}", e)))?;

        fs::write(&registry_path, yaml)?;
        Ok(())
    }

    /// Load registry from disk
    fn load_registry(state_dir: &Path) -> Result<PortRegistry> {
        let registry_path = state_dir.join("port-registry.yaml");

        if !registry_path.exists() {
            return Ok(PortRegistry::default());
        }

        let content = fs::read_to_string(&registry_path)?;
        let registry: PortRegistry = serde_yml::from_str(&content)
            .map_err(|e| HnError::DockerError(format!("Failed to parse registry: {}", e)))?;

        Ok(registry)
    }

    /// Allocate next available port for a service type
    fn allocate_port_for_service(&mut self, service: &str) -> Result<u16> {
        // Get base port for this service
        let base_port = self.base_ports.get(service).copied().unwrap_or(3000);

        // Start from base port to find any gaps from released ports
        let mut port = base_port;

        loop {
            if port > self.port_range_end {
                return Err(HnError::PortAllocationError(format!(
                    "Port exhausted for service '{}': no available ports in range {}-{}",
                    service, self.port_range_start, self.port_range_end
                )));
            }

            // Check if this port is already allocated
            let port_used = self.registry.allocations.values().any(|services| {
                services.values().any(|&p| p == port)
            });

            if !port_used {
                // Found an available port
                self.registry.next_available.insert(service.to_string(), port + 1);
                return Ok(port);
            }

            port += 1;
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
