// Sync command: Sync current worktree with another branch (typically main)
use crate::errors::{HnError, Result};
use crate::vcs::git::GitBackend;
use std::env;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyncStrategy {
    Merge,
    Rebase,
}

impl SyncStrategy {
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "merge" => Ok(Self::Merge),
            "rebase" => Ok(Self::Rebase),
            _ => Err(HnError::ConfigError(format!(
                "Invalid sync strategy '{}'. Use 'merge' or 'rebase'.",
                s
            ))),
        }
    }
}

pub fn run(
    source_branch: Option<String>,
    strategy: Option<String>,
    autostash: bool,
    no_commit: bool,
) -> Result<()> {
    let git = GitBackend::open_from_current_dir()?;

    // Get current worktree
    let current_worktree = git.get_current_worktree()?;

    // Default source branch to "main"
    let source = source_branch.unwrap_or_else(|| "main".to_string());

    // Parse strategy (default to merge)
    let sync_strategy = if let Some(strat) = strategy {
        SyncStrategy::from_str(&strat)?
    } else {
        SyncStrategy::Merge
    };

    eprintln!("→ Current worktree: {}", current_worktree.name);
    eprintln!("→ Current branch: {}", current_worktree.branch);
    eprintln!("→ Syncing with: {}", source);
    eprintln!("→ Strategy: {:?}", sync_strategy);

    // Check if current worktree has uncommitted changes
    let status = git.get_worktree_status(&current_worktree.path)?;
    if !status.is_clean() && !autostash {
        return Err(HnError::Git(git2::Error::from_str(&format!(
            "Worktree '{}' has uncommitted changes. Use --autostash or commit/stash them first.",
            current_worktree.name
        ))));
    }

    // Change to current worktree directory
    env::set_current_dir(&current_worktree.path)?;

    // Stash if needed and autostash is enabled
    let mut stashed = false;
    if !status.is_clean() && autostash {
        eprintln!("\n→ Stashing uncommitted changes...");
        let stash_output = Command::new("git")
            .arg("stash")
            .arg("push")
            .arg("-m")
            .arg(format!("hn sync autostash - {}", chrono::Utc::now().to_rfc3339()))
            .output()?;

        if !stash_output.status.success() {
            let stderr = String::from_utf8_lossy(&stash_output.stderr);
            return Err(HnError::Git(git2::Error::from_str(&format!(
                "Failed to stash changes: {}",
                stderr
            ))));
        }
        stashed = true;
        eprintln!("✓ Changes stashed");
    }

    // Fetch the latest from the source branch
    eprintln!("\n→ Fetching latest changes from {}...", source);
    let fetch_output = Command::new("git")
        .arg("fetch")
        .arg("origin")
        .arg(&source)
        .output()?;

    if !fetch_output.status.success() {
        let stderr = String::from_utf8_lossy(&fetch_output.stderr);
        eprintln!("⚠ Warning: Failed to fetch from origin: {}", stderr);
        eprintln!("  Continuing with local branch...");
    } else {
        eprintln!("✓ Fetch complete");
    }

    // Perform sync based on strategy
    let sync_result = match sync_strategy {
        SyncStrategy::Merge => sync_merge(&source, no_commit),
        SyncStrategy::Rebase => sync_rebase(&source),
    };

    // Handle the result
    match sync_result {
        Ok(_) => {
            eprintln!("✓ Sync successful");

            // Pop stash if we stashed
            if stashed {
                eprintln!("\n→ Restoring stashed changes...");
                let pop_output = Command::new("git").arg("stash").arg("pop").output()?;

                if !pop_output.status.success() {
                    let stderr = String::from_utf8_lossy(&pop_output.stderr);
                    eprintln!("⚠ Warning: Failed to restore stashed changes: {}", stderr);
                    eprintln!("  Your changes are still in the stash. Run 'git stash pop' manually.");
                    return Err(HnError::Git(git2::Error::from_str(
                        "Failed to restore stashed changes",
                    )));
                }
                eprintln!("✓ Changes restored");
            }

            Ok(())
        }
        Err(e) => {
            // If sync failed and we stashed, inform user
            if stashed {
                eprintln!("\n⚠ Note: Your changes are stashed. Run 'git stash pop' to restore them after resolving conflicts.");
            }
            Err(e)
        }
    }
}

fn sync_merge(source: &str, no_commit: bool) -> Result<()> {
    eprintln!("\n→ Merging {} into current branch...", source);

    let mut cmd = Command::new("git");
    cmd.arg("merge");

    if no_commit {
        cmd.arg("--no-commit");
    }

    cmd.arg(source);

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Check if it's a conflict
        if stderr.contains("CONFLICT") || stdout.contains("CONFLICT") {
            eprintln!("\n⚠ Merge conflicts detected:");
            eprintln!("{}", stdout);
            eprintln!("{}", stderr);
            eprintln!("\nResolve conflicts manually, then run: git commit");
            return Err(HnError::Git(git2::Error::from_str(
                "Merge conflicts need manual resolution",
            )));
        }

        return Err(HnError::Git(git2::Error::from_str(&format!(
            "Failed to merge: {}{}",
            stdout, stderr
        ))));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        eprintln!("{}", stdout);
    }

    Ok(())
}

fn sync_rebase(source: &str) -> Result<()> {
    eprintln!("\n→ Rebasing current branch onto {}...", source);

    let mut cmd = Command::new("git");
    cmd.arg("rebase").arg(source);

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Check if it's a conflict
        if stderr.contains("CONFLICT") || stdout.contains("CONFLICT") || stderr.contains("could not apply") {
            eprintln!("\n⚠ Rebase conflicts detected:");
            eprintln!("{}", stdout);
            eprintln!("{}", stderr);
            eprintln!("\nResolve conflicts manually, then run:");
            eprintln!("  git add <resolved-files>");
            eprintln!("  git rebase --continue");
            eprintln!("\nOr abort the rebase:");
            eprintln!("  git rebase --abort");
            return Err(HnError::Git(git2::Error::from_str(
                "Rebase conflicts need manual resolution",
            )));
        }

        return Err(HnError::Git(git2::Error::from_str(&format!(
            "Failed to rebase: {}{}",
            stdout, stderr
        ))));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        eprintln!("{}", stdout);
    }

    Ok(())
}
