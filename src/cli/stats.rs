// Resource usage statistics (v0.6)

use crate::config::Config;
use crate::errors::Result;
use crate::monitoring::{self, get_metrics_path, MetricsHistory, MetricsSnapshot};
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

pub fn run(
    name: Option<String>,
    show_all: bool,
    disk_only: bool,
    show_history: bool,
    history_days: Option<u64>,
    vcs_type: Option<VcsType>,
) -> Result<()> {
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
        worktrees
            .iter()
            .filter(|wt| wt.name == *filter_name)
            .collect()
    } else {
        worktrees.iter().collect()
    };

    if filtered_worktrees.is_empty() {
        println!("{}", "No worktrees found".yellow());
        println!();
        return Ok(());
    }

    let mut total_size = 0u64;
    let now = monitoring::now();

    for wt in &filtered_worktrees {
        if !show_all && wt.parent.is_none() {
            continue; // Skip main worktree unless --all
        }

        println!();
        println!("{}", wt.name.cyan().bold());
        println!("{}", "-".repeat(40));

        // Current disk usage
        let current_size = if let Ok(size) = get_dir_size(&wt.path) {
            println!(
                "  {:<15} {}",
                "Disk Usage:".bold(),
                format_size(size).green()
            );
            total_size += size;
            Some(size)
        } else {
            None
        };

        // State directory size
        let wt_state_dir = state_dir.join(&wt.name);
        let state_size = if wt_state_dir.exists() {
            if let Ok(state_size) = get_dir_size(&wt_state_dir) {
                println!(
                    "  {:<15} {}",
                    "State Dir:".bold(),
                    format_size(state_size).dimmed()
                );
                Some(state_size)
            } else {
                None
            }
        } else {
            None
        };

        if !disk_only {
            // Additional stats
            println!("  {:<15} {}", "Branch:".bold(), wt.branch.dimmed());
            println!(
                "  {:<15} {}",
                "Path:".bold(),
                wt.path.display().to_string().dimmed()
            );
        }

        // Record current metrics for historical tracking
        if let (Some(disk), Some(state)) = (current_size, state_size) {
            let snapshot = MetricsSnapshot {
                timestamp: now,
                disk_usage: disk,
                state_dir_size: state,
                docker_running: false, // TODO: check docker status
                docker_memory_mb: None,
                docker_cpu_percent: None,
            };
            let _ = monitoring::record_metrics(&state_dir, &wt.name, snapshot);
        }

        // Show history if requested
        if show_history {
            let metrics_path = get_metrics_path(&state_dir, &wt.name);
            if metrics_path.exists() {
                if let Ok(history) = MetricsHistory::load(&metrics_path) {
                    print_history(&history, history_days);
                }
            } else {
                println!();
                println!("  {}", "No historical data available".dimmed());
            }
        }
    }

    println!();
    println!("{}", "═".repeat(80));
    println!(
        "{:<20} {}",
        "Total Disk Usage:".bold(),
        format_size(total_size).green().bold()
    );
    println!();

    Ok(())
}

fn print_history(history: &MetricsHistory, days: Option<u64>) {
    let days = days.unwrap_or(7);
    let now = monitoring::now();
    let start_time = now.saturating_sub(days * 24 * 3600);

    let snapshots = history.range(start_time, now);

    if snapshots.is_empty() {
        println!();
        println!("  {}", "No historical data in the specified range".dimmed());
        return;
    }

    println!();
    println!(
        "  {} (last {} days)",
        "Historical Data".bold().underline(),
        days
    );
    println!();

    // Show recent snapshots (last 5)
    let recent = if snapshots.len() > 5 {
        &snapshots[snapshots.len() - 5..]
    } else {
        &snapshots[..]
    };

    for snap in recent {
        let timestamp = snap.timestamp;
        let date = format_timestamp(timestamp);
        println!(
            "  {} │ Disk: {} │ State: {}",
            date.dimmed(),
            format_size(snap.disk_usage).green(),
            format_size(snap.state_dir_size).dimmed()
        );
    }

    // Show trend
    if snapshots.len() >= 2 {
        let first = snapshots[0];
        let last = snapshots[snapshots.len() - 1];
        let change = last.disk_usage as i64 - first.disk_usage as i64;

        println!();
        if change > 0 {
            println!(
                "  {} {} ({})",
                "Trend:".bold(),
                format!("↑ {}", format_size(change.unsigned_abs())).red(),
                "increased".red()
            );
        } else if change < 0 {
            println!(
                "  {} {} ({})",
                "Trend:".bold(),
                format!("↓ {}", format_size(change.unsigned_abs())).green(),
                "decreased".green()
            );
        } else {
            println!("  {} {}", "Trend:".bold(), "→ stable".dimmed());
        }
    }
}

fn format_timestamp(timestamp: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let dt = UNIX_EPOCH + Duration::from_secs(timestamp);
    let now = std::time::SystemTime::now();

    if let Ok(duration) = now.duration_since(dt) {
        let secs = duration.as_secs();
        if secs < 60 {
            format!("{}s ago", secs)
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86400 {
            format!("{}h ago", secs / 3600)
        } else {
            format!("{}d ago", secs / 86400)
        }
    } else {
        "future".to_string()
    }
}
