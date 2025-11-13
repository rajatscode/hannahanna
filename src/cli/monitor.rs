// Real-time monitoring dashboard for worktrees (v0.6)
//
// This is a basic implementation that provides snapshot monitoring.
// Live dashboard features will be enhanced in future versions.

use crate::config::Config;
use crate::errors::Result;
use crate::state::StateManager;
use crate::vcs::{init_backend_from_current_dir, VcsType};
use colored::*;

/// Run the monitoring dashboard
pub fn run(live: bool, _refresh_secs: Option<u64>, vcs_type: Option<VcsType>) -> Result<()> {
    let backend = if let Some(vcs) = vcs_type {
        crate::vcs::init_backend_with_detection(&std::env::current_dir()?, Some(vcs))?
    } else {
        init_backend_from_current_dir()?
    };

    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;
    let state_manager = StateManager::new(&repo_root)?;

    if live {
        eprintln!("{}", "Live monitoring mode coming soon!".yellow());
        eprintln!("{}", "For now, showing snapshot view...".dimmed());
        println!();
    }

    // Get worktrees
    let worktrees = backend.list_workspaces()?;
    let config = Config::load(&repo_root)?;

    println!("\n{}", "Hannahanna Monitoring Dashboard".bold().cyan());
    println!("{}", "═".repeat(80).cyan());
    println!();

    // System-wide stats
    println!("{}", "System Overview".bold());
    println!(
        "  Active Worktrees: {}",
        worktrees.len().to_string().green()
    );
    println!(
        "  Docker Enabled: {}",
        if config.docker.enabled {
            "Yes".green()
        } else {
            "No".red()
        }
    );
    println!();

    // Worktree details
    println!("{}", "Worktrees".bold());
    println!("{}", "─".repeat(80));
    println!(
        "{:<25} {:<20} {:<15} {:<10}",
        "NAME", "BRANCH", "COMMIT", "PATH"
    );
    println!("{}", "─".repeat(80));

    for wt in &worktrees {
        let commit_short = if wt.commit.len() > 7 {
            &wt.commit[..7]
        } else {
            &wt.commit
        };

        let path_str = wt.path.to_string_lossy();
        let path_display = if path_str.len() > 30 {
            format!("...{}", &path_str[path_str.len() - 27..])
        } else {
            path_str.to_string()
        };

        println!(
            "{:<25} {:<20} {:<15} {}",
            wt.name.cyan(),
            truncate(&wt.branch, 20),
            commit_short.yellow(),
            path_display.dimmed()
        );
    }

    println!("{}", "─".repeat(80));
    println!();

    // Check for state directories
    let state_worktrees = state_manager.list_worktrees()?;
    if state_worktrees.len() != worktrees.len() {
        println!(
            "{} {} orphaned state directories detected. Run `hn state clean` to remove them.",
            "⚠".yellow(),
            state_worktrees.len().saturating_sub(worktrees.len())
        );
        println!();
    }

    Ok(())
}

/// Truncate string to max length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
