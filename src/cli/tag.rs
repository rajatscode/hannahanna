// Tag management CLI commands (v0.6)

use crate::config::Config;
use crate::errors::Result;
use crate::tags;
use colored::*;
use std::env;

/// Add tags to a worktree
pub fn add(worktree: &str, new_tags: &[String]) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;
    let state_dir = repo_root.join(".hn-state");

    // Add tags
    tags::add_tags(&state_dir, worktree, new_tags)?;

    println!();
    println!(
        "{} Tagged '{}' with: {}",
        "✓".green().bold(),
        worktree.cyan(),
        new_tags.join(", ").yellow()
    );
    println!();

    Ok(())
}

/// Remove tags from a worktree
#[allow(dead_code)]
pub fn remove(worktree: &str, tags_to_remove: &[String]) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;
    let state_dir = repo_root.join(".hn-state");

    // Remove tags
    tags::remove_tags(&state_dir, worktree, tags_to_remove)?;

    println!();
    println!(
        "{} Removed tags from '{}': {}",
        "✓".green().bold(),
        worktree.cyan(),
        tags_to_remove.join(", ").dimmed()
    );
    println!();

    Ok(())
}

/// List all tags or tags for a specific worktree
pub fn list(worktree: Option<&str>) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;
    let state_dir = repo_root.join(".hn-state");

    if let Some(wt) = worktree {
        // List tags for specific worktree
        let worktree_tags = tags::get_worktree_tags(&state_dir, wt)?;

        println!();
        if worktree_tags.is_empty() {
            println!("No tags for worktree '{}'", wt.cyan());
        } else {
            println!("Tags for '{}': {}", wt.cyan(), worktree_tags.join(", ").yellow());
        }
        println!();
    } else {
        // List all tags
        let all_tags = tags::list_all_tags(&state_dir)?;

        println!();
        if all_tags.is_empty() {
            println!("{}", "No tags found".yellow());
            println!();
            println!("Create tags with: {} <worktree> <tag1> <tag2> ...", "hn tag".bold());
        } else {
            println!("{}", "All Tags".bold());
            println!("{}", "═".repeat(50));

            let mut tags: Vec<_> = all_tags.into_iter().collect();
            tags.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0))); // Sort by count desc, then name

            for (tag, count) in tags {
                println!(
                    "  {:<30} {} {}",
                    tag.yellow(),
                    count.to_string().dimmed(),
                    if count == 1 { "worktree" } else { "worktrees" }.dimmed()
                );
            }

            println!("{}", "═".repeat(50));
        }
        println!();
    }

    Ok(())
}
