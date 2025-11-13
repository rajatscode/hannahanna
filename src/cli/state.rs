// State command: Manage hannahanna state directories
use crate::config::Config;
use crate::errors::Result;
use crate::state::StateManager;
use crate::vcs::{init_backend_from_current_dir, RegistryCache};
use colored::Colorize;
use std::fs;

/// List all state directories
pub fn list() -> Result<()> {
    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;
    let _state_manager = StateManager::new(&repo_root)?;
    let state_root = repo_root.join(".hn-state");

    if !state_root.exists() {
        println!("{}", "No state directory found.".bright_yellow());
        println!("\nState directories are created automatically when you:");
        println!("  • Create a worktree: {}", "hn add <name>".bright_cyan());
        return Ok(());
    }

    // Get all state directories
    let entries = fs::read_dir(&state_root)?;
    let mut state_dirs: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    if state_dirs.is_empty() {
        println!("{}", "No state directories found.".bright_yellow());
        return Ok(());
    }

    state_dirs.sort();

    // Get active worktrees
    let backend = init_backend_from_current_dir()?;
    let worktrees = backend.list_workspaces()?;
    let active_names: Vec<String> = worktrees.iter().map(|wt| wt.name.clone()).collect();

    println!("{}", "State Directories".bright_cyan().bold());
    println!("{}", "=".repeat(70));

    for name in &state_dirs {
        let state_dir = state_root.join(name);
        let size = calculate_dir_size(&state_dir)?;
        let size_str = format_size(size);

        let status = if active_names.contains(name) {
            "active".bright_green()
        } else {
            "orphaned".bright_red()
        };

        println!("  {} {} ({})", name.bright_cyan(), status, size_str);
    }

    println!("{}", "=".repeat(70));
    println!(
        "\nTotal: {} state directories ({} active, {} orphaned)",
        state_dirs.len(),
        state_dirs
            .iter()
            .filter(|n| active_names.contains(n))
            .count(),
        state_dirs
            .iter()
            .filter(|n| !active_names.contains(n))
            .count()
    );

    let orphaned_count = state_dirs
        .iter()
        .filter(|n| !active_names.contains(n))
        .count();
    if orphaned_count > 0 {
        println!(
            "\n{}: Clean orphaned state with: {}",
            "Tip".bright_yellow(),
            "hn state clean".bright_cyan()
        );
    }

    Ok(())
}

/// Clean orphaned state directories
pub fn clean() -> Result<()> {
    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;
    let state_manager = StateManager::new(&repo_root)?;

    // Get active worktrees
    let backend = init_backend_from_current_dir()?;
    let worktrees = backend.list_workspaces()?;
    let active_names: Vec<String> = worktrees.iter().map(|wt| wt.name.clone()).collect();

    // Find orphaned state directories
    let orphaned = state_manager.list_orphaned(&active_names)?;

    if orphaned.is_empty() {
        println!(
            "{}",
            "✓ No orphaned state directories found.".bright_green()
        );
        return Ok(());
    }

    println!(
        "Found {} orphaned state director{}:",
        orphaned.len(),
        if orphaned.len() == 1 { "y" } else { "ies" }
    );
    for name in &orphaned {
        println!("  {} {}", "•".bright_red(), name);
    }

    // Clean orphaned directories
    println!("\nCleaning...");
    let cleaned = state_manager.clean_orphaned(&active_names)?;

    println!(
        "\n{} Cleaned {} orphaned state director{}.",
        "✓".bright_green(),
        cleaned.len(),
        if cleaned.len() == 1 { "y" } else { "ies" }
    );

    Ok(())
}

/// Show size of state directories
pub fn size(name: Option<String>) -> Result<()> {
    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;
    let state_root = repo_root.join(".hn-state");

    if !state_root.exists() {
        println!("{}", "No state directory found.".bright_yellow());
        return Ok(());
    }

    if let Some(worktree_name) = name {
        // Show size for specific worktree
        let state_dir = state_root.join(&worktree_name);

        if !state_dir.exists() {
            println!(
                "{}: State directory for '{}' not found",
                "Error".bright_red(),
                worktree_name
            );
            return Ok(());
        }

        let size = calculate_dir_size(&state_dir)?;
        println!(
            "State directory for '{}': {}",
            worktree_name.bright_cyan(),
            format_size(size).bright_green()
        );
    } else {
        // Show total size
        let total_size = calculate_dir_size(&state_root)?;

        println!("{}", "State Directory Sizes".bright_cyan().bold());
        println!("{}", "=".repeat(70));

        // List individual sizes
        let entries = fs::read_dir(&state_root)?;
        let mut sizes: Vec<(String, u64)> = Vec::new();

        for entry in entries.filter_map(|e| e.ok()) {
            if entry.path().is_dir() {
                if let Ok(name) = entry.file_name().into_string() {
                    let size = calculate_dir_size(&entry.path())?;
                    sizes.push((name, size));
                }
            }
        }

        sizes.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by size descending

        for (name, size) in &sizes {
            println!("  {}: {}", name.bright_cyan(), format_size(*size));
        }

        println!("{}", "=".repeat(70));
        println!(
            "Total state size: {}",
            format_size(total_size).bright_green().bold()
        );
    }

    Ok(())
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
            } else {
                size += entry.metadata()?.len();
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

/// Show cache statistics
pub fn cache_stats() -> Result<()> {
    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;
    let state_dir = repo_root.join(".hn-state");

    let cache = RegistryCache::new(&state_dir, None)?;

    match cache.stats()? {
        Some(stats) => {
            println!("{}", "Worktree Registry Cache".bright_cyan().bold());
            println!("{}", "=".repeat(50));
            println!(
                "Status: {}",
                if stats.valid {
                    "Valid".bright_green()
                } else {
                    "Expired".bright_red()
                }
            );
            println!("Age: {:.1}s", stats.age.as_secs_f64());
            println!("Worktrees: {}", stats.worktree_count);
            println!("Size: {}", format_size(stats.size_bytes));
            println!("{}", "=".repeat(50));

            if !stats.valid {
                println!(
                    "\n{}: Cache is expired. Run {} to refresh.",
                    "Note".bright_yellow(),
                    "hn list".bright_cyan()
                );
            }
        }
        None => {
            println!("{}", "No cache found.".bright_yellow());
            println!(
                "\nThe cache will be created automatically when you run: {}",
                "hn list".bright_cyan()
            );
        }
    }

    Ok(())
}

/// Clear the cache
pub fn cache_clear() -> Result<()> {
    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;
    let state_dir = repo_root.join(".hn-state");

    let cache = RegistryCache::new(&state_dir, None)?;
    cache.invalidate()?;

    println!("{}", "✓ Cache cleared successfully.".bright_green());
    println!(
        "\nThe cache will be rebuilt automatically on next: {}",
        "hn list".bright_cyan()
    );

    Ok(())
}
