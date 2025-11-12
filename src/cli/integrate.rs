// Integrate command: Merge a source worktree/branch into a target worktree/branch
use crate::config::Config;
use crate::errors::{HnError, Result};
use crate::fuzzy;
use crate::hooks::{HookExecutor, HookType};
use crate::state::StateManager;
use crate::vcs::{init_backend_from_current_dir, VcsType};
use std::env;
use std::process::Command;

pub fn run(
    source: String,
    into: Option<String>,
    no_ff: bool,
    squash: bool,
    strategy: Option<String>,
    vcs_type: Option<VcsType>,
) -> Result<()> {
    // Validate flag combinations
    if squash && no_ff {
        return Err(HnError::ConfigError(
            "Cannot use both --squash and --no-ff".to_string(),
        ));
    }

    let backend = if let Some(vcs) = vcs_type {
        crate::vcs::init_backend_with_detection(&env::current_dir()?, Some(vcs))?
    } else {
        init_backend_from_current_dir()?
    };
    let worktrees = backend.list_workspaces()?;

    // Determine target worktree (defaults to current)
    let target_worktree = if let Some(target_name) = into {
        // Find target worktree by name (with fuzzy matching)
        let names: Vec<String> = worktrees.iter().map(|wt| wt.name.clone()).collect();
        let matched_name = fuzzy::find_best_match(&target_name, &names)?;
        worktrees
            .iter()
            .find(|wt| wt.name == matched_name)
            .ok_or_else(|| HnError::WorktreeNotFound(matched_name.clone()))?
            .clone()
    } else {
        // Use current worktree as target
        backend.get_current_workspace()?
    };

    // Determine source branch/worktree
    // Try to match as a worktree name first, otherwise treat as branch name
    let names: Vec<String> = worktrees.iter().map(|wt| wt.name.clone()).collect();
    let source_branch = match fuzzy::find_best_match(&source, &names) {
        Ok(matched_name) => {
            // Found a worktree with this name
            let wt = worktrees
                .iter()
                .find(|wt| wt.name == matched_name)
                .ok_or_else(|| HnError::WorktreeNotFound(matched_name.clone()))?;
            eprintln!("→ Using worktree '{}' (branch: {})", wt.name, wt.branch);
            wt.branch.clone()
        }
        Err(_) => {
            // Not a worktree name, treat as branch name
            eprintln!("→ Using branch '{}'", source);
            source.clone()
        }
    };

    eprintln!("→ Target worktree: {}", target_worktree.name);
    eprintln!("→ Target branch: {}", target_worktree.branch);
    eprintln!("→ Source branch: {}", source_branch);

    // Check if target has uncommitted changes
    let status = backend.get_workspace_status(&target_worktree.path)?;
    if !status.is_clean() {
        return Err(HnError::Git(git2::Error::from_str(&format!(
            "Target worktree '{}' has uncommitted changes. Commit or stash them first.",
            target_worktree.name
        ))));
    }

    // Load config and run pre_integrate hook
    let repo_root = Config::find_repo_root(&target_worktree.path)?;
    let config = Config::load(&repo_root)?;

    let has_pre_integrate_hooks = config.hooks.pre_integrate.is_some()
        || !config.hooks.pre_integrate_conditions.is_empty();

    if has_pre_integrate_hooks {
        let state_manager = StateManager::new(&repo_root)?;
        let state_dir = state_manager.get_state_dir(&target_worktree.name);

        eprintln!("Running pre_integrate hook...");
        let hook_executor = HookExecutor::new(config.hooks.clone(), false);
        hook_executor.run_hook(HookType::PreIntegrate, &target_worktree, &state_dir)?;
        eprintln!("✓ Pre-integrate hook completed successfully");
    }

    // Change to target worktree directory
    env::set_current_dir(&target_worktree.path)?;

    // Build the git merge command
    let mut cmd = Command::new("git");
    cmd.arg("merge");

    // Add merge strategy if specified
    if let Some(strat) = strategy {
        cmd.arg("--strategy").arg(strat);
    }

    // Add no-ff flag if specified
    if no_ff {
        cmd.arg("--no-ff");
    }

    // Add squash flag if specified
    if squash {
        cmd.arg("--squash");
    }

    // Add source branch
    cmd.arg(&source_branch);

    eprintln!(
        "\n→ Merging '{}' into '{}'...",
        source_branch, target_worktree.branch
    );

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Check if it's a conflict
        if stderr.contains("CONFLICT") || stdout.contains("CONFLICT") {
            eprintln!("\n⚠ Merge conflicts detected:");
            eprintln!("{}", stdout);
            eprintln!("{}", stderr);
            eprintln!(
                "\nResolve conflicts manually in: {}",
                target_worktree.path.display()
            );
            eprintln!("Then run: git commit");
            return Err(HnError::Git(git2::Error::from_str(
                "Merge conflicts need manual resolution",
            )));
        }

        return Err(HnError::Git(git2::Error::from_str(&format!(
            "Failed to merge '{}' into '{}': {}{}",
            source_branch, target_worktree.branch, stdout, stderr
        ))));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        eprintln!("{}", stdout);
    }

    if squash {
        eprintln!("✓ Squash merge successful (changes staged but not committed)");
        eprintln!(
            "  Run 'git commit' in {} to complete the merge",
            target_worktree.path.display()
        );
    } else {
        eprintln!("✓ Merge successful");
    }

    // Run post_integrate hook
    let has_post_integrate_hooks = config.hooks.post_integrate.is_some()
        || !config.hooks.post_integrate_conditions.is_empty();

    if has_post_integrate_hooks {
        let state_manager = StateManager::new(&repo_root)?;
        let state_dir = state_manager.get_state_dir(&target_worktree.name);

        eprintln!("Running post_integrate hook...");
        let hook_executor = HookExecutor::new(config.hooks.clone(), false);
        hook_executor.run_hook(HookType::PostIntegrate, &target_worktree, &state_dir)?;
        eprintln!("✓ Post-integrate hook completed successfully");
    }

    Ok(())
}
