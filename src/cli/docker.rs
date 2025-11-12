use crate::config::Config;
use crate::docker::container::ContainerManager;
use crate::errors::Result;
use crate::state::StateManager;
use std::env;

/// Show Docker container status for all worktrees
pub fn ps() -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let config = Config::load(&repo_root)?;
    let state_dir = repo_root.join(".hn-state");

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
    let state_dir = repo_root.join(".hn-state");

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
    let state_dir = repo_root.join(".hn-state");

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

/// Restart Docker containers for a worktree
pub fn restart(name: String) -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let config = Config::load(&repo_root)?;
    let state_dir = repo_root.join(".hn-state");

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

    println!("Restarting containers for '{}'...", name);
    manager.restart(&name, &worktree_path)?;
    println!("✓ Containers restarted for '{}'", name);

    Ok(())
}

/// View logs for a worktree's containers
pub fn logs(name: String, service: Option<String>) -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let config = Config::load(&repo_root)?;
    let state_dir = repo_root.join(".hn-state");

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

    // Get safe command arguments (no shell injection)
    let (program, args) = manager.get_logs_command(&name, service.as_deref())?;

    println!("Following logs for '{}'...", name);
    if let Some(svc) = &service {
        println!("Service: {}", svc);
    }
    println!("(Press Ctrl+C to exit)\n");

    // Execute directly without shell - no injection risk
    std::process::Command::new(&program)
        .args(&args)
        .current_dir(&worktree_path)
        .status()?;

    Ok(())
}

/// Execute a command in a worktree's container
pub fn exec(name: String, service: Option<String>, command: Vec<String>) -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let config = Config::load(&repo_root)?;
    let state_dir = repo_root.join(".hn-state");

    if !config.docker.enabled {
        return Err(crate::errors::HnError::DockerError(
            "Docker support is not enabled in .hannahanna.yml".to_string(),
        ));
    }

    if command.is_empty() {
        return Err(crate::errors::HnError::DockerError(
            "No command specified".to_string(),
        ));
    }

    let _manager = ContainerManager::new(&config.docker, &state_dir)?;
    let worktree_path = repo_root.join("worktrees").join(&name);

    if !worktree_path.exists() {
        return Err(crate::errors::HnError::WorktreeNotFound(name.clone()));
    }

    // Determine which service to exec into
    let service_name = if let Some(svc) = service {
        svc
    } else {
        // Use first service from config if not specified
        if let Some(first_service) = config.docker.ports.base.keys().next() {
            first_service.clone()
        } else {
            return Err(crate::errors::HnError::DockerError(
                "No services configured and none specified".to_string(),
            ));
        }
    };

    println!("Executing command in '{}' (service: {})...", name, service_name);

    // Try modern "docker compose" first, fallback to legacy "docker-compose"
    let compose_cmd = if std::process::Command::new("docker")
        .args(["compose", "version"])
        .output()
        .is_ok()
    {
        vec!["docker", "compose"]
    } else {
        vec!["docker-compose"]
    };

    // Build exec command
    let mut cmd = std::process::Command::new(compose_cmd[0]);
    if compose_cmd.len() > 1 {
        cmd.arg(compose_cmd[1]);
    }
    cmd.arg("exec")
        .arg(&service_name)
        .args(&command)
        .current_dir(&worktree_path);

    // Execute command
    let status = cmd.status()?;

    if !status.success() {
        return Err(crate::errors::HnError::DockerError(format!(
            "Command failed with exit code {}",
            status.code().unwrap_or(-1)
        )));
    }

    Ok(())
}

/// Clean up orphaned Docker containers
pub fn prune() -> Result<()> {
    let repo_root = Config::find_repo_root(&env::current_dir()?)?;
    let config = Config::load(&repo_root)?;
    let state_dir = repo_root.join(".hn-state");

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
