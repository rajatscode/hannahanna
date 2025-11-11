use crate::config::Config;
use crate::docker::container::ContainerManager;
use crate::docker::ports::PortAllocator;
use crate::errors::Result;
use crate::fuzzy;
use crate::vcs::{init_backend_from_current_dir, VcsType};
use std::env;

/// Show detailed information about a worktree
///
/// If no name is provided, shows info for the current worktree
pub fn run(name: Option<String>, vcs_type: Option<VcsType>) -> Result<()> {
    let backend = if let Some(vcs) = vcs_type {
        crate::vcs::init_backend_with_detection(&env::current_dir()?, Some(vcs))?
    } else {
        init_backend_from_current_dir()?
    };

    // Determine which worktree to show info for
    let worktree = if let Some(name) = name {
        // Get all worktrees for fuzzy matching
        let worktrees = backend.list_workspaces()?;
        let worktree_names: Vec<String> = worktrees.iter().map(|wt| wt.name.clone()).collect();

        // Find the best match using fuzzy matching
        let matched_name = fuzzy::find_best_match(&name, &worktree_names)?;

        // Show info for named worktree
        backend.get_workspace_by_name(&matched_name)?
    } else {
        // Show info for current worktree
        backend.get_current_workspace()?
    };

    // Get status
    let status = backend.get_workspace_status(&worktree.path)?;

    // Print worktree information
    println!("Worktree: {}", worktree.name);
    println!("Path: {}", worktree.path.display());
    println!("Branch: {}", worktree.branch);
    println!(
        "Commit: {}",
        &worktree.commit[..7.min(worktree.commit.len())]
    );
    println!();

    // Print status
    println!("Status:");
    if status.is_clean() {
        println!("  Clean (no changes)");
    } else {
        if status.modified > 0 {
            println!(
                "  Modified: {} file{}",
                status.modified,
                if status.modified == 1 { "" } else { "s" }
            );
        }
        if status.added > 0 {
            println!(
                "  Added: {} file{}",
                status.added,
                if status.added == 1 { "" } else { "s" }
            );
        }
        if status.deleted > 0 {
            println!(
                "  Deleted: {} file{}",
                status.deleted,
                if status.deleted == 1 { "" } else { "s" }
            );
        }
        if status.untracked > 0 {
            println!(
                "  Untracked: {} file{}",
                status.untracked,
                if status.untracked == 1 { "" } else { "s" }
            );
        }
    }

    // Docker information
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let config = Config::load(&repo_root)?;

    if config.docker.enabled {
        println!();
        println!("Docker:");

        let state_dir = repo_root.join(".wt-state");

        // Port allocations
        match PortAllocator::new(&state_dir) {
            Ok(allocator) => {
                if let Ok(ports) = allocator.get_ports(&worktree.name) {
                    println!("  Ports:");
                    for (service, port) in ports {
                        println!("    {}: {}", service, port);
                    }
                } else {
                    println!("  Ports: Not allocated");
                }
            }
            Err(_) => {
                println!("  Ports: Error reading port allocations");
            }
        }

        // Container status
        match ContainerManager::new(&config.docker, &state_dir) {
            Ok(manager) => {
                if let Ok(status) = manager.get_status(&worktree.name, &worktree.path) {
                    let status_str = if status.running { "Running" } else { "Stopped" };
                    println!("  Containers: {}", status_str);
                } else {
                    println!("  Containers: Unknown");
                }
            }
            Err(_) => {
                println!("  Containers: Error");
            }
        }
    }

    Ok(())
}
