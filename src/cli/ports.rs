use crate::config::Config;
use crate::docker::ports::PortAllocator;
use crate::errors::Result;
use std::env;

/// List all port allocations across worktrees
pub fn list() -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let state_dir = repo_root.join(".hn-state");

    let allocator = PortAllocator::new(&state_dir)?;
    let allocations = allocator.list_all();

    if allocations.is_empty() {
        println!("No port allocations found.");
        return Ok(());
    }

    println!("{:<20} {:<15} {:<10}", "WORKTREE", "SERVICE", "PORT");
    println!("{}", "-".repeat(45));

    for (worktree, ports) in allocations {
        for (service, port) in ports {
            println!("{:<20} {:<15} {:<10}", worktree, service, port);
        }
    }

    Ok(())
}

/// Show port allocations for a specific worktree
pub fn show(name: String) -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let state_dir = repo_root.join(".hn-state");

    let allocator = PortAllocator::new(&state_dir)?;
    let ports = allocator.get_ports(&name)?;

    println!("Port allocations for '{}':", name);
    println!("{:<15} {:<10}", "SERVICE", "PORT");
    println!("{}", "-".repeat(25));

    for (service, port) in ports {
        println!("{:<15} {:<10}", service, port);
    }

    Ok(())
}

/// Release port allocations for a worktree
pub fn release(name: String) -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let state_dir = repo_root.join(".hn-state");

    let mut allocator = PortAllocator::new(&state_dir)?;
    allocator.release(&name)?;

    println!("Released port allocations for '{}'", name);

    Ok(())
}

/// Reassign ports for a worktree (release old, allocate new)
pub fn reassign(name: String) -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let state_dir = repo_root.join(".hn-state");

    let mut allocator = PortAllocator::new(&state_dir)?;

    // Get current allocations to know which services were allocated
    let current_allocations = allocator.get_ports(&name)?;

    if current_allocations.is_empty() {
        println!("No port allocations found for '{}'. Use 'hn docker start {}' to allocate ports.", name, name);
        return Ok(());
    }

    // Extract service names from current allocations
    let services: Vec<&str> = current_allocations.keys().map(|s| s.as_str()).collect();

    println!("Reassigning ports for '{}'...", name);
    println!("Current allocations:");
    for (service, port) in &current_allocations {
        println!("  {}: {}", service, port);
    }

    // Release current allocations
    allocator.release(&name)?;

    // Allocate new ports for the same services
    let new_allocations = allocator.allocate(&name, &services)?;

    println!("\nNew allocations:");
    for (service, port) in &new_allocations {
        println!("  {}: {}", service, port);
    }

    println!("\n✓ Successfully reassigned ports for '{}'", name);

    Ok(())
}
