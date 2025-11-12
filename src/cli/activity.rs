// Activity log viewing for worktrees (v0.6)
//
// Shows historical activity for worktrees with filtering options

use crate::config::Config;
use crate::errors::Result;
use crate::fuzzy::find_best_match;
use crate::monitoring::ActivityEvent;
use crate::state::StateManager;
use crate::vcs::{init_backend_from_current_dir, VcsType};
use colored::*;

/// Show activity log for a worktree
pub fn run(
    name: Option<String>,
    _since: Option<String>,
    _limit: Option<usize>,
    vcs_type: Option<VcsType>,
) -> Result<()> {
    let backend = if let Some(vcs) = vcs_type {
        crate::vcs::init_backend_with_detection(&std::env::current_dir()?, Some(vcs))?
    } else {
        init_backend_from_current_dir()?
    };

    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;
    let state_manager = StateManager::new(&repo_root)?;

    if let Some(wt_name) = name {
        // Show activity for specific worktree
        let worktrees = backend.list_workspaces()?;
        let worktree_names: Vec<String> = worktrees.iter().map(|w| w.name.clone()).collect();
        let matched_name = find_best_match(&wt_name, &worktree_names)?;

        show_worktree_activity(&matched_name, &state_manager)?;
    } else {
        // Show activity for all worktrees
        let worktrees = backend.list_workspaces()?;
        show_all_activity(&worktrees, &state_manager)?;
    }

    Ok(())
}

/// Show activity for a specific worktree
fn show_worktree_activity(
    name: &str,
    state_manager: &StateManager,
) -> Result<()> {
    let state_dir = state_manager.get_state_dir(name);
    let activity_log = state_dir.join("activity.json");

    println!();
    println!("{}", format!("Activity Log: {}", name).bold().cyan());
    println!("{}", "═".repeat(80).cyan());
    println!();

    if !activity_log.exists() {
        println!("  {}", "No activity log found".yellow());
        println!();
        println!("  Activity logging tracks:");
        println!("    • Worktree creation/removal");
        println!("    • Docker operations");
        println!("    • Hook executions");
        println!("    • Integration/sync operations");
        println!();
        return Ok(());
    }

    let content = std::fs::read_to_string(&activity_log)?;
    if content.is_empty() {
        println!("  {}", "Activity log is empty".yellow());
        println!();
        return Ok(());
    }

    let events: Vec<ActivityEvent> = serde_json::from_str(&content)?;

    if events.is_empty() {
        println!("  {}", "No events found".yellow());
        println!();
        return Ok(());
    }

    // Display events
    for event in &events {
        display_event(event);
    }

    println!();
    println!(
        "{} │ {}",
        "Total events".dimmed(),
        events.len().to_string().green()
    );
    println!();

    Ok(())
}

/// Show activity for all worktrees
fn show_all_activity(
    worktrees: &[crate::vcs::Worktree],
    state_manager: &StateManager,
) -> Result<()> {
    println!();
    println!("{}", "Activity Log: All Worktrees".bold().cyan());
    println!("{}", "═".repeat(80).cyan());
    println!();

    let mut found_any = false;

    for wt in worktrees {
        let state_dir = state_manager.get_state_dir(&wt.name);
        let activity_log = state_dir.join("activity.json");

        if !activity_log.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&activity_log)?;
        if content.is_empty() {
            continue;
        }

        let events: Vec<ActivityEvent> = serde_json::from_str(&content)?;
        if events.is_empty() {
            continue;
        }

        found_any = true;
        println!("{}", format!("► {}", wt.name).bold().green());
        println!("{}", "─".repeat(80));

        for event in &events {
            display_event(event);
        }

        println!();
    }

    if !found_any {
        println!("  {}", "No activity logs found".yellow());
        println!();
        println!("  Activity logging tracks:");
        println!("    • Worktree creation/removal");
        println!("    • Docker operations");
        println!("    • Hook executions");
        println!("    • Integration/sync operations");
        println!();
    }

    Ok(())
}

/// Display a single activity event
fn display_event(event: &ActivityEvent) {
    let (timestamp, icon, description) = match event {
        ActivityEvent::WorktreeCreated { timestamp, branch, template } => {
            let desc = if let Some(tmpl) = template {
                format!("Created from {} with template '{}'", branch.yellow(), tmpl.cyan())
            } else {
                format!("Created from {}", branch.yellow())
            };
            (*timestamp, "✨", desc)
        }
        ActivityEvent::WorktreeRemoved { timestamp } => {
            (*timestamp, "🗑️", "Worktree removed".to_string())
        }
        ActivityEvent::WorktreeSwitched { timestamp, from } => {
            let desc = if let Some(f) = from {
                format!("Switched from {}", f.yellow())
            } else {
                "Switched to this worktree".to_string()
            };
            (*timestamp, "🔄", desc)
        }
        ActivityEvent::DockerStarted { timestamp, services } => {
            let desc = format!("Docker started ({})", services.join(", ").cyan());
            (*timestamp, "🐳", desc)
        }
        ActivityEvent::DockerStopped { timestamp } => {
            (*timestamp, "🐳", "Docker stopped".to_string())
        }
        ActivityEvent::HookExecuted { timestamp, hook, duration_ms, success } => {
            let status_icon = if *success { "✓".green() } else { "✗".red() };
            let desc = format!(
                "Hook {} executed {} ({} ms)",
                hook.cyan(),
                status_icon,
                duration_ms
            );
            (*timestamp, "🪝", desc)
        }
        ActivityEvent::IntegrationPerformed { timestamp, source, target } => {
            let desc = format!("Integrated {} → {}", source.yellow(), target.yellow());
            (*timestamp, "🔗", desc)
        }
        ActivityEvent::SnapshotCreated { timestamp, snapshot_name } => {
            let desc = format!("Snapshot '{}' created", snapshot_name.cyan());
            (*timestamp, "📸", desc)
        }
        ActivityEvent::SnapshotRestored { timestamp, snapshot_name } => {
            let desc = format!("Snapshot '{}' restored", snapshot_name.cyan());
            (*timestamp, "📸", desc)
        }
    };

    let time_str = format_timestamp(timestamp);
    println!("  {} │ {} │ {}", time_str.dimmed(), icon, description);
}

/// Format timestamp as human-readable date/time
fn format_timestamp(timestamp: u64) -> String {
    use chrono::{Local, TimeZone};

    let dt = Local.timestamp_opt(timestamp as i64, 0).unwrap();
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}
