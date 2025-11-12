// Resource usage statistics (v0.5)

use crate::config::Config;
use crate::errors::Result;
use crate::vcs::{self, VcsType};
use colored::*;
use std::env;
use std::fs;

fn get_dir_size(path: &std::path::Path) -> Result<u64> {
    let mut size = 0;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                size += get_dir_size(&entry.path())?;
            } else {
                size += metadata.len();
            }
        }
    }
    Ok(size)
}

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
        format!("{} B", bytes)
    }
}

pub fn run(name: Option<String>, show_all: bool, disk_only: bool, vcs_type: Option<VcsType>) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;

    let vcs_backend = if let Some(vcs) = vcs_type {
        vcs::init_backend_with_detection(&repo_root, Some(vcs))?
    } else {
        vcs::init_backend_with_detection(&repo_root, None)?
    };

    let worktrees = vcs_backend.list_workspaces()?;
    let state_dir = repo_root.join(".hn-state");

    println!();
    println!("{}", "Resource Usage Statistics".bold());
    println!("{}", "═".repeat(80));

    // Filter worktrees if name specified
    let filtered_worktrees: Vec<_> = if let Some(ref filter_name) = name {
        worktrees.iter().filter(|wt| wt.name == *filter_name).collect()
    } else {
        worktrees.iter().collect()
    };

    if filtered_worktrees.is_empty() {
        println!("{}", "No worktrees found".yellow());
        println!();
        return Ok(());
    }

    let mut total_size = 0u64;

    for wt in &filtered_worktrees {
        if !show_all && wt.parent.is_none() {
            continue; // Skip main worktree unless --all
        }

        println!();
        println!("{}", wt.name.cyan().bold());
        println!("{}", "-".repeat(40));

        // Disk usage
        if let Ok(size) = get_dir_size(&wt.path) {
            println!("  {:<15} {}", "Disk Usage:".bold(), format_size(size).green());
            total_size += size;
        }

        // State directory size
        let wt_state_dir = state_dir.join(&wt.name);
        if wt_state_dir.exists() {
            if let Ok(state_size) = get_dir_size(&wt_state_dir) {
                println!("  {:<15} {}", "State Dir:".bold(), format_size(state_size).dimmed());
            }
        }

        if !disk_only {
            // Additional stats
            println!("  {:<15} {}", "Branch:".bold(), wt.branch.dimmed());
            println!("  {:<15} {}", "Path:".bold(), wt.path.display().to_string().dimmed());
        }
    }

    println!();
    println!("{}", "═".repeat(80));
    println!("{:<20} {}", "Total Disk Usage:".bold(), format_size(total_size).green().bold());
    println!();

    Ok(())
}
