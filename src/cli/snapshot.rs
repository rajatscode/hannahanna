// Snapshot CLI commands

use crate::config::Config;
use crate::errors::Result;
use crate::snapshot::{self, Snapshot};
use crate::vcs::{self, VcsType};
use colored::*;

/// Create a snapshot of a worktree
pub fn create(
    worktree: &str,
    name: Option<&str>,
    description: Option<&str>,
    vcs_type: Option<VcsType>,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;

    // Get VCS backend
    let backend = if let Some(vcs) = vcs_type {
        vcs::init_backend_with_detection(&repo_root, Some(vcs))?
    } else {
        vcs::init_backend_with_detection(&repo_root, None)?
    };

    // Find the worktree
    let worktrees = backend.list_workspaces()?;
    let wt = worktrees
        .iter()
        .find(|w| w.name == worktree)
        .ok_or_else(|| crate::errors::HnError::WorktreeNotFound(worktree.to_string()))?;

    let state_dir = repo_root.join(".hn-state");

    println!();
    println!("{} snapshot for '{}'...", "Creating".bold(), worktree.cyan());

    let snapshot = snapshot::create_snapshot(&wt.path, &wt.name, name, description, &state_dir)?;

    println!();
    println!("{} Snapshot created successfully!", "✓".green().bold());
    println!();
    println!("{}: {}", "Name".bold(), snapshot.name.cyan());
    println!("{}: {}", "Branch".bold(), snapshot.branch);
    println!("{}: {}", "Commit".bold(), &snapshot.commit[..8.min(snapshot.commit.len())]);

    if snapshot.has_uncommitted {
        println!("{}: {}", "Uncommitted changes".bold().yellow(), "saved".green());
    } else {
        println!("{}: {}", "Uncommitted changes".bold(), "none".dimmed());
    }

    if let Some(desc) = &snapshot.description {
        println!("{}: {}", "Description".bold(), desc.as_str().dimmed());
    }

    println!();
    println!("Restore with: {} {} {} {}",
        "hn snapshot restore".bold(),
        worktree.cyan(),
        snapshot.name.cyan(),
        "--help".dimmed());
    println!();

    Ok(())
}

/// List snapshots
pub fn list(worktree: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;
    let state_dir = repo_root.join(".hn-state");

    let snapshots = snapshot::list_snapshots(&state_dir, worktree)?;

    if snapshots.is_empty() {
        println!();
        if let Some(wt) = worktree {
            println!("{}", format!("No snapshots found for worktree '{}'", wt).yellow());
        } else {
            println!("{}", "No snapshots found".yellow());
        }
        println!();
        println!("Create a snapshot with:");
        println!("  {} <worktree> [name]", "hn snapshot create".bold());
        println!();
        return Ok(());
    }

    println!();
    if let Some(wt) = worktree {
        println!("{} {}", "Snapshots for".bold(), wt.cyan().bold());
    } else {
        println!("{}", "All Snapshots".bold());
    }
    println!("{}", "═".repeat(80));

    // Group by worktree if showing all
    if worktree.is_none() {
        use std::collections::HashMap;
        let mut by_worktree: HashMap<String, Vec<&Snapshot>> = HashMap::new();
        for snap in &snapshots {
            by_worktree
                .entry(snap.worktree.clone())
                .or_default()
                .push(snap);
        }

        for (wt_name, wt_snapshots) in by_worktree {
            println!();
            println!("{}", wt_name.cyan().bold());
            for snap in wt_snapshots {
                print_snapshot(snap);
            }
        }
    } else {
        println!();
        for snap in &snapshots {
            print_snapshot(snap);
        }
    }

    println!("{}", "═".repeat(80));
    println!("{} snapshot{}", snapshots.len().to_string().green().bold(), if snapshots.len() == 1 { "" } else { "s" });
    println!();

    Ok(())
}

fn print_snapshot(snap: &Snapshot) {
    let commit_short = &snap.commit[..8.min(snap.commit.len())];
    print!("  {:<25}", snap.name.cyan());
    print!(" │ {:<15}", snap.branch.dimmed());
    print!(" │ {}", commit_short.dimmed());

    if snap.has_uncommitted {
        print!(" │ {}", "uncommitted".yellow());
    }

    println!();

    if let Some(desc) = &snap.description {
        println!("    {}", desc.as_str().dimmed());
    }
}

/// Restore a snapshot
pub fn restore(
    worktree: &str,
    snapshot: &str,
    vcs_type: Option<VcsType>,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;

    // Get VCS backend
    let backend = if let Some(vcs) = vcs_type {
        vcs::init_backend_with_detection(&repo_root, Some(vcs))?
    } else {
        vcs::init_backend_with_detection(&repo_root, None)?
    };

    // Find the worktree
    let worktrees = backend.list_workspaces()?;
    let wt = worktrees
        .iter()
        .find(|w| w.name == worktree)
        .ok_or_else(|| crate::errors::HnError::WorktreeNotFound(worktree.to_string()))?;

    let state_dir = repo_root.join(".hn-state");

    println!();
    println!("{} snapshot '{}'...", "Restoring".bold(), snapshot.cyan());

    snapshot::restore_snapshot(&wt.path, &wt.name, snapshot, &state_dir)?;

    println!();
    println!("{} Snapshot '{}' restored successfully!", "✓".green().bold(), snapshot.cyan());
    println!();
    println!("{}: {}", "Worktree".bold(), worktree.cyan());
    println!();

    Ok(())
}

/// Delete a snapshot
pub fn delete(
    worktree: &str,
    snapshot: &str,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;
    let state_dir = repo_root.join(".hn-state");

    snapshot::delete_snapshot(worktree, snapshot, &state_dir)?;

    println!();
    println!("{} Snapshot '{}' deleted", "✓".green().bold(), snapshot.cyan());
    println!();

    Ok(())
}
