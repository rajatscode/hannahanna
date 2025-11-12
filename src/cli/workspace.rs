// Workspace management CLI commands (v0.5)

use crate::config::Config;
use crate::docker::ports::PortAllocator;
use crate::errors::{HnError, Result};
use crate::vcs::{self, VcsType};
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize, Deserialize, Clone)]
struct WorktreeInfo {
    name: String,
    branch: String,
    path: PathBuf,
    commit: Option<String>,
    git_status: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct DockerState {
    enabled: bool,
    ports: HashMap<String, u16>,
    services: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct Workspace {
    name: String,
    description: Option<String>,
    created_at: String,
    worktrees: Vec<WorktreeInfo>,
    config_snapshot: Option<String>,
    docker_state: Option<DockerState>,
    hannahanna_version: String,
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
        .map(|wt| {
            // Capture git status
            let git_status = Command::new("git")
                .arg("-C")
                .arg(&wt.path)
                .arg("status")
                .arg("--short")
                .output()
                .ok()
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            WorktreeInfo {
                name: wt.name.clone(),
                branch: wt.branch.clone(),
                path: wt.path.clone(),
                commit: Some(wt.commit.clone()),
                git_status,
            }
        })
        .collect();

    // Snapshot current config
    let config_path = repo_root.join(".hannahanna.yml");
    let config_snapshot = if config_path.exists() {
        Some(fs::read_to_string(&config_path)?)
    } else {
        None
    };

    // Capture Docker state if enabled
    let docker_state = if let Ok(config) = Config::load(&repo_root) {
        if config.docker.enabled {
            let state_dir = repo_root.join(".hn-state");
            if let Ok(port_allocator) = PortAllocator::new(&state_dir) {
                let mut all_ports = HashMap::new();
                let mut all_services = Vec::new();

                // Collect ports from all worktrees
                for wt_info in &worktree_infos {
                    if let Ok(ports) = port_allocator.get_ports(&wt_info.name) {
                        for (service, port) in ports {
                            all_ports.insert(format!("{}:{}", wt_info.name, service), port);
                            if !all_services.contains(&service) {
                                all_services.push(service);
                            }
                        }
                    }
                }

                Some(DockerState {
                    enabled: true,
                    ports: all_ports,
                    services: all_services,
                })
            } else {
                Some(DockerState {
                    enabled: true,
                    ports: HashMap::new(),
                    services: Vec::new(),
                })
            }
        } else {
            None
        }
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
        docker_state: docker_state.clone(),
        hannahanna_version: env!("CARGO_PKG_VERSION").to_string(),
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

    // Show Docker info if enabled
    if let Some(ref docker) = docker_state {
        if docker.enabled {
            println!("{}: {} ({} unique services)",
                "Docker state".bold(),
                "captured".green(),
                docker.services.len());
        }
    }

    // Show worktrees with uncommitted changes
    let uncommitted: Vec<&WorktreeInfo> = workspace.worktrees.iter()
        .filter(|wt| wt.git_status.is_some())
        .collect();
    if !uncommitted.is_empty() {
        println!("{}: {}", "Worktrees with changes".bold().yellow(), uncommitted.len());
        for wt in uncommitted {
            println!("  - {}", wt.name.yellow());
        }
    }

    println!();
    println!("Restore with: {} {}", "hn workspace restore".bold(), name.cyan());
    println!("Export with: {} {} {}", "hn workspace export".bold(), name.cyan(), "[path]".dimmed());
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

/// Export workspace to a comprehensive file
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
    let workspace: Workspace = serde_json::from_str(&content)?;

    let default_output = format!("{}.workspace.json", name);
    let output_path = output.unwrap_or(&default_output);

    // Write enhanced export with full metadata
    let json = serde_json::to_string_pretty(&workspace)?;
    fs::write(output_path, json)?;

    println!();
    println!("{} Workspace exported successfully!", "✓".green().bold());
    println!();
    println!("{}: {}", "File".bold(), output_path.cyan());
    println!("{}: {}", "Worktrees".bold(), workspace.worktrees.len());

    if let Some(ref docker) = workspace.docker_state {
        if docker.enabled {
            println!("{}: {} ports, {} services",
                "Docker state".bold(),
                docker.ports.len(),
                docker.services.len());
        }
    }

    let uncommitted_count = workspace.worktrees.iter()
        .filter(|wt| wt.git_status.is_some())
        .count();
    if uncommitted_count > 0 {
        println!("{}: {} worktrees",
            "Uncommitted changes".bold().yellow(),
            uncommitted_count);
    }

    println!();
    println!("Import with: {} {}", "hn workspace import".bold(), output_path.cyan());
    println!();

    Ok(())
}

/// Import workspace from a file
pub fn import(path: &str, create_worktrees: bool, vcs_type: Option<VcsType>) -> Result<()> {
    // Read and parse workspace file
    let content = fs::read_to_string(path)
        .map_err(|e| HnError::ConfigError(format!("Failed to read workspace file: {}", e)))?;

    let workspace: Workspace = serde_json::from_str(&content)
        .map_err(|e| HnError::ConfigError(format!("Invalid workspace file: {}", e)))?;

    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;
    let workspaces_dir = get_workspaces_dir(&repo_root)?;

    // Check if workspace with this name already exists
    let workspace_file = workspaces_dir.join(format!("{}.json", workspace.name));
    if workspace_file.exists() {
        return Err(HnError::ConfigError(format!(
            "Workspace '{}' already exists. Delete it first or choose a different name.",
            workspace.name
        )));
    }

    println!();
    println!("{} workspace '{}'...", "Importing".bold(), workspace.name.cyan().bold());
    println!();

    // Save workspace metadata
    let json = serde_json::to_string_pretty(&workspace)?;
    fs::write(&workspace_file, json)?;
    println!("{} Workspace metadata saved", "✓".green());

    // Optionally create worktrees
    if create_worktrees {
        println!();
        println!("{} worktrees...", "Creating".bold());

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

        let mut created = 0;
        let mut skipped = 0;
        let mut failed = 0;

        for wt_info in &workspace.worktrees {
            if existing_names.contains_key(&wt_info.name) {
                println!("  {} Skipping '{}' (already exists)", "⊘".yellow(), wt_info.name.yellow());
                skipped += 1;
                continue;
            }

            println!("  {} Creating '{}'...", "•".cyan(), wt_info.name.cyan());

            // Create worktree on the same branch
            match vcs_backend.create_workspace(&wt_info.name, Some(&wt_info.branch), None, true) {
                Ok(_) => {
                    created += 1;

                    // Show git status if there were uncommitted changes
                    if let Some(ref status) = wt_info.git_status {
                        println!("    {} Had uncommitted changes (not restored):", "ℹ".dimmed());
                        for line in status.lines().take(3) {
                            println!("      {}", line.dimmed());
                        }
                    }
                }
                Err(e) => {
                    println!("    {} Failed: {}", "✗".red(), e);
                    failed += 1;
                }
            }
        }

        println!();
        println!("{}", "Summary:".bold());
        println!("  {} created", created.to_string().green().bold());
        if skipped > 0 {
            println!("  {} skipped", skipped.to_string().yellow());
        }
        if failed > 0 {
            println!("  {} failed", failed.to_string().red());
        }

        if failed > 0 {
            println!();
            return Err(HnError::ConfigError(format!(
                "Failed to create {} worktree(s)",
                failed
            )));
        }
    } else {
        println!();
        println!("{}", "Note:".bold());
        println!("  Worktrees were not created. Use --create-worktrees to create them.");
        println!("  Or restore later with: {} {}", "hn workspace restore".bold(), workspace.name.cyan());
    }

    println!();
    println!("{} Workspace '{}' imported successfully!", "✓".green().bold(), workspace.name.cyan().bold());
    println!();

    Ok(())
}

/// Compare two workspaces and show differences
pub fn diff(name1: &str, name2: &str) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;
    let workspaces_dir = get_workspaces_dir(&repo_root)?;

    // Load both workspaces
    let ws1_file = workspaces_dir.join(format!("{}.json", name1));
    let ws2_file = workspaces_dir.join(format!("{}.json", name2));

    if !ws1_file.exists() {
        return Err(HnError::ConfigError(format!("Workspace '{}' not found", name1)));
    }
    if !ws2_file.exists() {
        return Err(HnError::ConfigError(format!("Workspace '{}' not found", name2)));
    }

    let ws1: Workspace = serde_json::from_str(&fs::read_to_string(&ws1_file)?)?;
    let ws2: Workspace = serde_json::from_str(&fs::read_to_string(&ws2_file)?)?;

    println!();
    println!("{}", "═".repeat(80).bright_blue());
    println!("  {} '{}' {} '{}'", "Comparing".bold(), name1.cyan(), "vs".dimmed(), name2.cyan());
    println!("{}", "═".repeat(80).bright_blue());
    println!();

    // Compare worktrees
    println!("{}", "Worktrees".bold().underline());
    println!();

    let wt1_names: HashMap<String, &WorktreeInfo> = ws1.worktrees.iter()
        .map(|wt| (wt.name.clone(), wt))
        .collect();
    let wt2_names: HashMap<String, &WorktreeInfo> = ws2.worktrees.iter()
        .map(|wt| (wt.name.clone(), wt))
        .collect();

    // Worktrees only in ws1
    let only_in_1: Vec<&WorktreeInfo> = ws1.worktrees.iter()
        .filter(|wt| !wt2_names.contains_key(&wt.name))
        .collect();
    if !only_in_1.is_empty() {
        println!("{} (only in {}):", "Added".green().bold(), name1.cyan());
        for wt in only_in_1 {
            println!("  + {} ({})", wt.name.green(), wt.branch.dimmed());
        }
        println!();
    }

    // Worktrees only in ws2
    let only_in_2: Vec<&WorktreeInfo> = ws2.worktrees.iter()
        .filter(|wt| !wt1_names.contains_key(&wt.name))
        .collect();
    if !only_in_2.is_empty() {
        println!("{} (only in {}):", "Removed".red().bold(), name2.cyan());
        for wt in only_in_2 {
            println!("  - {} ({})", wt.name.red(), wt.branch.dimmed());
        }
        println!();
    }

    // Worktrees in both but with differences
    let mut changed = Vec::new();
    for wt1 in &ws1.worktrees {
        if let Some(wt2) = wt2_names.get(&wt1.name) {
            if wt1.branch != wt2.branch || wt1.commit != wt2.commit {
                changed.push((wt1, wt2));
            }
        }
    }

    if !changed.is_empty() {
        println!("{}", "Modified".yellow().bold());
        for (wt1, wt2) in changed {
            println!("  ~ {}", wt1.name.yellow());
            if wt1.branch != wt2.branch {
                println!("    Branch: {} → {}", wt1.branch.dimmed(), wt2.branch.cyan());
            }
            if wt1.commit != wt2.commit {
                if let (Some(c1), Some(c2)) = (&wt1.commit, &wt2.commit) {
                    let c1_short = &c1[..c1.len().min(8)];
                    let c2_short = &c2[..c2.len().min(8)];
                    println!("    Commit: {} → {}", c1_short.dimmed(), c2_short.cyan());
                }
            }
        }
        println!();
    }

    // Compare Docker state
    if ws1.docker_state.is_some() || ws2.docker_state.is_some() {
        println!("{}", "Docker State".bold().underline());
        println!();

        match (&ws1.docker_state, &ws2.docker_state) {
            (Some(d1), Some(d2)) => {
                if d1.services != d2.services {
                    println!("{}: {} → {}",
                        "Services".bold(),
                        d1.services.len(),
                        d2.services.len());

                    let s1: std::collections::HashSet<_> = d1.services.iter().collect();
                    let s2: std::collections::HashSet<_> = d2.services.iter().collect();

                    for s in s1.difference(&s2) {
                        println!("  - {}", s.red());
                    }
                    for s in s2.difference(&s1) {
                        println!("  + {}", s.green());
                    }
                }

                if d1.ports.len() != d2.ports.len() {
                    println!("{}: {} → {}",
                        "Port allocations".bold(),
                        d1.ports.len(),
                        d2.ports.len());
                }
            }
            (Some(_), None) => {
                println!("{}: {} → {}", "Docker".bold(), "enabled".green(), "disabled".red());
            }
            (None, Some(_)) => {
                println!("{}: {} → {}", "Docker".bold(), "disabled".dimmed(), "enabled".green());
            }
            _ => {}
        }
        println!();
    }

    // Summary
    println!("{}", "═".repeat(80).bright_blue());
    println!("{}: {} │ {}: {}",
        name1.cyan().bold(),
        ws1.worktrees.len(),
        name2.cyan().bold(),
        ws2.worktrees.len());
    println!("{}", "═".repeat(80).bright_blue());
    println!();

    Ok(())
}
