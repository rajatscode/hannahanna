use crate::config::Config;
use crate::docker::container::ContainerManager;
use crate::errors::Result;
use crate::state::StateManager;
use std::env;

/// Show Docker container status for all worktrees
pub fn ps() -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let config = Config::load(&repo_root)?;
    let state_dir = repo_root.join(".wt-state");

    if !config.docker.enabled {
        println!("Docker support is not enabled in .hannahanna.yml");
        return Ok(());
    }

    let manager = ContainerManager::new(&config.docker, &state_dir)?;
    let state_mgr = StateManager::new(&state_dir)?;

    println!("{:<20} {:<15} {:<10}", "WORKTREE", "STATUS", "CONTAINERS");
    println!("{}", "-".repeat(45));

    // Get all worktrees from state
    let worktrees = state_mgr.list_worktrees()?;

    for worktree in worktrees {
        let worktree_path = repo_root.join("worktrees").join(&worktree);
        match manager.get_status(&worktree, &worktree_path) {
            Ok(status) => {
                let status_str = if status.running { "Running" } else { "Stopped" };
                println!(
                    "{:<20} {:<15} {:<10}",
                    worktree, status_str, status.container_count
                );
            }
            Err(_) => {
                println!("{:<20} {:<15} {:<10}", worktree, "Unknown", 0);
            }
        }
    }

    Ok(())
}

/// Start Docker containers for a worktree
pub fn start(name: String) -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let config = Config::load(&repo_root)?;
    let state_dir = repo_root.join(".wt-state");

    if !config.docker.enabled {
        return Err(crate::errors::HnError::DockerError(
            "Docker support is not enabled in .hannahanna.yml".to_string(),
        ));
    }

    let manager = ContainerManager::new(&config.docker, &state_dir)?;
    let worktree_path = repo_root.join("worktrees").join(&name);

    if !worktree_path.exists() {
        return Err(crate::errors::HnError::WorktreeNotFound(name));
    }

    println!("Starting containers for '{}'...", name);
    manager.start(&name, &worktree_path)?;
    println!("✓ Containers started for '{}'", name);

    Ok(())
}

/// Stop Docker containers for a worktree
pub fn stop(name: String) -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let config = Config::load(&repo_root)?;
    let state_dir = repo_root.join(".wt-state");

    if !config.docker.enabled {
        return Err(crate::errors::HnError::DockerError(
            "Docker support is not enabled in .hannahanna.yml".to_string(),
        ));
    }

    let manager = ContainerManager::new(&config.docker, &state_dir)?;
    let worktree_path = repo_root.join("worktrees").join(&name);

    if !worktree_path.exists() {
        return Err(crate::errors::HnError::WorktreeNotFound(name));
    }

    println!("Stopping containers for '{}'...", name);
    manager.stop(&name, &worktree_path)?;
    println!("✓ Containers stopped for '{}'", name);

    Ok(())
}

/// View logs for a worktree's containers
pub fn logs(name: String, service: Option<String>) -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let config = Config::load(&repo_root)?;
    let state_dir = repo_root.join(".wt-state");

    if !config.docker.enabled {
        return Err(crate::errors::HnError::DockerError(
            "Docker support is not enabled in .hannahanna.yml".to_string(),
        ));
    }

    let manager = ContainerManager::new(&config.docker, &state_dir)?;
    let worktree_path = repo_root.join("worktrees").join(&name);

    if !worktree_path.exists() {
        return Err(crate::errors::HnError::WorktreeNotFound(name));
    }

    // Allow deprecated - this is CLI display code, actual container operations use safe methods
    #[allow(deprecated)]
    let cmd = manager.build_logs_command(&name, &worktree_path, service.as_deref())?;
    println!("Running: {}", cmd);
    println!("(Press Ctrl+C to exit)\n");

    // Execute the logs command
    std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .current_dir(&worktree_path)
        .status()?;

    Ok(())
}

/// Clean up orphaned Docker containers
pub fn prune() -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let config = Config::load(&repo_root)?;
    let state_dir = repo_root.join(".wt-state");

    if !config.docker.enabled {
        return Err(crate::errors::HnError::DockerError(
            "Docker support is not enabled in .hannahanna.yml".to_string(),
        ));
    }

    let manager = ContainerManager::new(&config.docker, &state_dir)?;
    let state_mgr = StateManager::new(&state_dir)?;

    // Get active worktrees
    let active_worktrees = state_mgr.list_worktrees()?;

    println!("Cleaning up orphaned Docker containers...");
    manager.cleanup_orphaned(&active_worktrees)?;
    println!("✓ Cleanup complete");

    Ok(())
}
