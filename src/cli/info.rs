use crate::config::Config;
use crate::docker::container::ContainerManager;
use crate::docker::ports::PortAllocator;
use crate::errors::Result;
use crate::fuzzy;
use crate::vcs::{init_backend_from_current_dir, VcsType};
use chrono::{DateTime, Local};
use colored::Colorize;
use std::env;
use std::fs;
use std::time::SystemTime;

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

    // Get all worktrees for parent/children relationships
    let all_worktrees = backend.list_workspaces()?;

    // Get status
    let status = backend.get_workspace_status(&worktree.path)?;

    let repo_root = Config::find_repo_root(&env::current_dir()?)?;

    // Print header
    println!("{}", format!("Worktree: {}", worktree.name).bright_cyan().bold());
    println!("{}", "=".repeat(60));

    // Basic info
    println!("{}: {}", "Path".bright_white(), worktree.path.display());
    println!("{}: {}", "Branch".bright_white(), worktree.branch);
    println!(
        "{}: {}",
        "Commit".bright_white(),
        &worktree.commit[..7.min(worktree.commit.len())]
    );
    println!("{}: {}", "VCS".bright_white(), backend.vcs_type().as_str());

    // Status with emoji
    print!("{}: ", "Status".bright_white());
    if status.is_clean() {
        println!("{}", "✓ Clean (no uncommitted changes)".bright_green());
    } else {
        let changes = vec![
            if status.modified > 0 {
                Some(format!("{} modified", status.modified))
            } else {
                None
            },
            if status.added > 0 {
                Some(format!("{} added", status.added))
            } else {
                None
            },
            if status.deleted > 0 {
                Some(format!("{} deleted", status.deleted))
            } else {
                None
            },
            if status.untracked > 0 {
                Some(format!("{} untracked", status.untracked))
            } else {
                None
            },
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
        println!("{}", format!("⚠ {}", changes).yellow());
    }

    // Age (time since creation)
    let state_dir = repo_root.join(".hn-state").join(&worktree.name);
    if state_dir.exists() {
        if let Ok(metadata) = fs::metadata(&state_dir) {
            if let Ok(created) = metadata.created() {
                let age = SystemTime::now()
                    .duration_since(created)
                    .unwrap_or_default();
                let age_str = format_duration(age);
                let created_time: DateTime<Local> = created.into();
                println!(
                    "{}: {} (created {})",
                    "Age".bright_white(),
                    age_str,
                    created_time.format("%Y-%m-%d %H:%M")
                );
            }
        }
    }

    // Disk usage
    if let Ok(size) = calculate_dir_size(&worktree.path) {
        println!("{}: {}", "Disk".bright_white(), format_size(size));
    }

    // Parent/Children relationships
    println!();
    if let Some(ref parent_name) = worktree.parent {
        println!("{}: {}", "Parent".bright_white(), parent_name.bright_cyan());
    } else {
        println!("{}: {}", "Parent".bright_white(), "None (root worktree)".dimmed());
    }

    let children: Vec<_> = all_worktrees
        .iter()
        .filter(|wt| wt.parent.as_ref() == Some(&worktree.name))
        .collect();

    if !children.is_empty() {
        println!("{}:", "Children".bright_white());
        for child in &children {
            // Get age of child
            let child_state_dir = repo_root.join(".hn-state").join(&child.name);
            let age_str = if child_state_dir.exists() {
                if let Ok(metadata) = fs::metadata(&child_state_dir) {
                    if let Ok(created) = metadata.created() {
                        let age = SystemTime::now()
                            .duration_since(created)
                            .unwrap_or_default();
                        format!(" (created {})", format_duration(age))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            println!("  - {}{}", child.name.bright_cyan(), age_str.dimmed());
        }
    }

    // Docker information
    let config = Config::load(&repo_root)?;

    if config.docker.enabled {
        println!();
        println!("{}:", "Docker".bright_white());

        let state_dir_root = repo_root.join(".hn-state");

        // Port allocations
        match PortAllocator::new(&state_dir_root) {
            Ok(allocator) => {
                if let Ok(ports) = allocator.get_ports(&worktree.name) {
                    if !ports.is_empty() {
                        print!("  {}: ", "Ports".bright_white());
                        let port_strs: Vec<String> = ports
                            .iter()
                            .map(|(service, port)| format!("{}:{}", service, port))
                            .collect();
                        println!("{}", port_strs.join(", "));
                    } else {
                        println!("  {}: {}", "Ports".bright_white(), "Not allocated".dimmed());
                    }
                } else {
                    println!("  {}: {}", "Ports".bright_white(), "Not allocated".dimmed());
                }
            }
            Err(_) => {
                println!(
                    "  {}: {}",
                    "Ports".bright_white(),
                    "Error reading allocations".red()
                );
            }
        }

        // Container status with memory/CPU
        match ContainerManager::new(&config.docker, &state_dir_root) {
            Ok(manager) => {
                if let Ok(docker_status) = manager.get_status(&worktree.name, &worktree.path) {
                    if docker_status.running {
                        println!("  {}: {}", "Containers".bright_white(), "Running".bright_green());

                        // Try to get container stats (memory/CPU)
                        if let Ok(stats) = get_container_stats(&worktree.name) {
                            println!("    {}: {}", "Memory".bright_white(), stats.memory);
                            println!("    {}: {}", "CPU".bright_white(), stats.cpu);
                        }
                    } else {
                        println!("  {}: {}", "Containers".bright_white(), "Stopped".dimmed());
                    }
                } else {
                    println!("  {}: {}", "Containers".bright_white(), "Unknown".dimmed());
                }
            }
            Err(_) => {
                println!("  {}: {}", "Containers".bright_white(), "Error".red());
            }
        }
    }

    // Actions section
    println!();
    println!("{}:", "Actions".bright_white());
    println!(
        "  {} {}",
        "→".bright_green(),
        format!("hn switch {}", worktree.name).bright_cyan()
    );
    if let Some(parent) = &worktree.parent {
        println!(
            "  {} {}",
            "→".bright_green(),
            format!("hn integrate {} {}", worktree.name, parent).bright_cyan()
        );
    }
    if config.docker.enabled {
        println!(
            "  {} {}",
            "→".bright_green(),
            format!("hn docker logs {}", worktree.name).bright_cyan()
        );
    }
    println!(
        "  {} {}",
        "→".bright_green(),
        format!("hn remove {}", worktree.name).bright_cyan()
    );

    Ok(())
}

/// Format duration into human-readable string
fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{} seconds ago", secs)
    } else if secs < 3600 {
        format!("{} minutes ago", secs / 60)
    } else if secs < 86400 {
        format!("{} hours ago", secs / 3600)
    } else {
        format!("{} days ago", secs / 86400)
    }
}

/// Calculate directory size recursively
fn calculate_dir_size(path: &std::path::Path) -> Result<u64> {
    let mut size = 0u64;

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                size += calculate_dir_size(&path)?;
            } else if let Ok(metadata) = entry.metadata() {
                size += metadata.len();
            }
        }
    }

    Ok(size)
}

/// Format size in human-readable format
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Container stats
struct ContainerStats {
    memory: String,
    cpu: String,
}

/// Get container stats from docker stats (if docker is available)
fn get_container_stats(_worktree_name: &str) -> Result<ContainerStats> {
    use std::process::Command;

    // Try to get stats from docker stats command
    // TODO: Filter by worktree_name to get specific container stats
    let output = Command::new("docker")
        .args(&[
            "stats",
            "--no-stream",
            "--format",
            "{{.MemUsage}}\t{{.CPUPerc}}",
        ])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse first line (we'll improve this to filter by worktree name)
            if let Some(line) = stdout.lines().next() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    return Ok(ContainerStats {
                        memory: parts[0].to_string(),
                        cpu: parts[1].to_string(),
                    });
                }
            }
        }
    }

    // Fallback: no stats available
    Err(crate::errors::HnError::DockerError(
        "Unable to get container stats".to_string(),
    ))
}
