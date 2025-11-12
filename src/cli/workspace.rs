// Workspace management CLI commands (v0.5)

use crate::config::Config;
use crate::errors::{HnError, Result};
use crate::vcs::{self, VcsType};
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone)]
struct WorktreeInfo {
    name: String,
    branch: String,
    path: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct Workspace {
    name: String,
    description: Option<String>,
    created_at: String,
    worktrees: Vec<WorktreeInfo>,
    config_snapshot: Option<String>,
}

/// Get workspace storage directory
fn get_workspaces_dir(repo_root: &Path) -> Result<PathBuf> {
    let dir = repo_root.join(".hn-workspaces");
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// Validate workspace name
fn validate_workspace_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(HnError::ConfigError("Workspace name cannot be empty".into()));
    }
    if name.contains('/') || name.contains('\\') || name.starts_with('.') {
        return Err(HnError::ConfigError(format!(
            "Invalid workspace name '{}'. Workspace names cannot contain path separators or start with '.'",
            name
        )));
    }
    if name.trim() != name {
        return Err(HnError::ConfigError("Workspace name cannot have leading/trailing whitespace".into()));
    }
    Ok(())
}

/// Save current workspace state
pub fn save(name: &str, description: Option<&str>, vcs_type: Option<VcsType>) -> Result<()> {
    validate_workspace_name(name)?;

    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;
    let workspaces_dir = get_workspaces_dir(&repo_root)?;

    // Check if workspace already exists
    let workspace_file = workspaces_dir.join(format!("{}.json", name));
    if workspace_file.exists() {
        return Err(HnError::ConfigError(format!(
            "Workspace '{}' already exists. Delete it first or choose a different name.",
            name
        )));
    }

    // Get current worktrees
    let vcs_backend = if let Some(vcs) = vcs_type {
        vcs::init_backend_with_detection(&repo_root, Some(vcs))?
    } else {
        vcs::init_backend_with_detection(&repo_root, None)?
    };
    let worktrees = vcs_backend.list_workspaces()?;

    // Convert to WorktreeInfo (filter out main repo directory)
    // Main repo has .git as a directory, worktrees have .git as a file
    let worktree_infos: Vec<WorktreeInfo> = worktrees
        .iter()
        .filter(|wt| {
            let git_path = wt.path.join(".git");
            git_path.is_file() // Only include actual worktrees (not main repo)
        })
        .map(|wt| WorktreeInfo {
            name: wt.name.clone(),
            branch: wt.branch.clone(),
            path: wt.path.clone(),
        })
        .collect();

    // Snapshot current config
    let config_path = repo_root.join(".hannahanna.yml");
    let config_snapshot = if config_path.exists() {
        Some(fs::read_to_string(&config_path)?)
    } else {
        None
    };

    // Create workspace
    let created_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    let workspace = Workspace {
        name: name.to_string(),
        description: description.map(|s| s.to_string()),
        created_at,
        worktrees: worktree_infos,
        config_snapshot,
    };

    // Save to file
    let json = serde_json::to_string_pretty(&workspace)?;
    fs::write(&workspace_file, json)?;

    println!();
    println!("{} Workspace '{}' saved successfully!", "✓".green().bold(), name.cyan().bold());
    println!();
    println!("{}: {}", "Worktrees saved".bold(), workspace.worktrees.len());
    if let Some(desc) = description {
        println!("{}: {}", "Description".bold(), desc.dimmed());
    }
    println!();
    println!("Restore with: {} {} {}", "hn workspace restore".bold(), name.cyan(), "--help".dimmed());
    println!();

    Ok(())
}

/// Restore a saved workspace
pub fn restore(name: &str, force: bool, vcs_type: Option<VcsType>) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;
    let workspaces_dir = get_workspaces_dir(&repo_root)?;

    // Load workspace
    let workspace_file = workspaces_dir.join(format!("{}.json", name));
    if !workspace_file.exists() {
        return Err(HnError::ConfigError(format!(
            "Workspace '{}' not found. Run 'hn workspace list' to see available workspaces.",
            name
        )));
    }

    let json = fs::read_to_string(&workspace_file)?;
    let workspace: Workspace = serde_json::from_str(&json)?;

    // Get VCS interface
    let vcs_backend = if let Some(vcs) = vcs_type {
        vcs::init_backend_with_detection(&repo_root, Some(vcs))?
    } else {
        vcs::init_backend_with_detection(&repo_root, None)?
    };
    let existing_worktrees = vcs_backend.list_workspaces()?;
    let existing_names: HashMap<String, bool> = existing_worktrees
        .iter()
        .map(|wt| (wt.name.clone(), true))
        .collect();

    println!();
    println!("{} workspace '{}'...", "Restoring".bold(), name.cyan().bold());
    println!();

    // Restore each worktree
    let mut restored = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for wt_info in &workspace.worktrees {
        if existing_names.contains_key(&wt_info.name) {
            if !force {
                println!("  {} Skipping '{}' (already exists)", "⊘".yellow(), wt_info.name.yellow());
                skipped += 1;
                continue;
            }
            println!("  {} Overwriting '{}'...", "⚠".yellow(), wt_info.name.yellow());
        } else {
            println!("  {} Creating '{}'...", "•".cyan(), wt_info.name.cyan());
        }

        // Try to create/restore the worktree
        // Use no_branch=true to checkout existing branch instead of creating new one
        match vcs_backend.create_workspace(&wt_info.name, Some(&wt_info.branch), None, true) {
            Ok(_) => restored += 1,
            Err(e) => {
                println!("    {} Failed: {}", "✗".red(), e);
                failed += 1;
            }
        }
    }

    println!();
    println!("{}", "Summary:".bold());
    println!("  {} restored", restored.to_string().green().bold());
    if skipped > 0 {
        println!("  {} skipped", skipped.to_string().yellow());
    }
    if failed > 0 {
        println!("  {} failed", failed.to_string().red());
    }
    println!();

    if failed > 0 {
        Err(HnError::ConfigError(format!(
            "Failed to restore {} worktree(s)",
            failed
        )))
    } else {
        Ok(())
    }
}

/// List all saved workspaces
pub fn list(json: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;
    let workspaces_dir = get_workspaces_dir(&repo_root)?;

    // Find all workspace files
    let mut workspaces: Vec<Workspace> = Vec::new();

    if workspaces_dir.exists() {
        for entry in fs::read_dir(&workspaces_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(workspace) = serde_json::from_str::<Workspace>(&content) {
                        workspaces.push(workspace);
                    }
                }
            }
        }
    }

    // Sort by name
    workspaces.sort_by(|a, b| a.name.cmp(&b.name));

    if json {
        let json_output = serde_json::to_string_pretty(&workspaces)?;
        println!("{}", json_output);
        return Ok(());
    }

    if workspaces.is_empty() {
        println!();
        println!("{}", "No saved workspaces found".yellow());
        println!();
        println!("Save your current workspace with:");
        println!("  {} <name>", "hn workspace save".bold());
        println!();
        return Ok(());
    }

    println!();
    println!("{}", "Saved Workspaces".bold());
    println!("{}", "═".repeat(80));

    for workspace in &workspaces {
        // Name and worktree count
        print!("{:<20}", workspace.name.cyan().bold());
        print!(" │ {} worktrees", workspace.worktrees.len().to_string().green());

        // Date (timestamp)
        if let Ok(timestamp) = workspace.created_at.parse::<u64>() {
            // Simple date formatting
            print!(" │ {}", format!("created: {}", timestamp).dimmed());
        }

        println!();

        // Description
        if let Some(ref desc) = workspace.description {
            println!("{}  {}", " ".repeat(22), desc.dimmed());
        }
    }

    println!("{}", "═".repeat(80));
    println!("{} workspace{}", workspaces.len().to_string().green().bold(), if workspaces.len() == 1 { "" } else { "s" });
    println!();

    Ok(())
}

/// Delete a saved workspace
pub fn delete(name: &str, force: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;
    let workspaces_dir = get_workspaces_dir(&repo_root)?;

    let workspace_file = workspaces_dir.join(format!("{}.json", name));
    if !workspace_file.exists() {
        return Err(HnError::ConfigError(format!(
            "Workspace '{}' not found",
            name
        )));
    }

    if !force {
        // In a real CLI, we'd prompt for confirmation
        // For now, require --force flag
        return Err(HnError::ConfigError(
            "Use --force to confirm deletion".into(),
        ));
    }

    fs::remove_file(&workspace_file)?;

    println!();
    println!("{} Workspace '{}' deleted", "✓".green().bold(), name.cyan());
    println!();
    println!("{}", "Note: Worktrees themselves were not removed".dimmed());
    println!("Use {} to remove worktrees if needed.", "hn remove <name>".bold());
    println!();

    Ok(())
}

/// Export workspace to a file
pub fn export(name: &str, output: Option<&str>) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;
    let workspaces_dir = get_workspaces_dir(&repo_root)?;

    let workspace_file = workspaces_dir.join(format!("{}.json", name));
    if !workspace_file.exists() {
        return Err(HnError::ConfigError(format!(
            "Workspace '{}' not found",
            name
        )));
    }

    let content = fs::read_to_string(&workspace_file)?;
    let default_output = format!("{}.workspace.json", name);
    let output_path = output.unwrap_or(&default_output);

    fs::write(output_path, content)?;

    println!();
    println!("{} Workspace exported to {}", "✓".green().bold(), output_path.cyan());
    println!();

    Ok(())
}
