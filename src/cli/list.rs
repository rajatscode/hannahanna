use crate::errors::Result;
use crate::vcs::{init_backend_from_current_dir, short_commit, VcsType, Worktree};
use std::collections::HashMap;

pub fn run(tree: bool, vcs_type: Option<VcsType>) -> Result<()> {
    let backend = if let Some(vcs) = vcs_type {
        crate::vcs::init_backend_with_detection(&std::env::current_dir()?, Some(vcs))?
    } else {
        init_backend_from_current_dir()?
    };
    let worktrees = backend.list_workspaces()?;

    if tree {
        // Tree view with parent/child relationships
        display_tree_view(&worktrees);
    } else {
        // Standard table view
        println!("{:<20} {:<25} {:<10}", "NAME", "BRANCH", "COMMIT");
        println!("{}", "-".repeat(60));

        // Print each worktree
        for wt in worktrees {
            println!(
                "{:<20} {:<25} {:<10}",
                wt.name,
                wt.branch,
                short_commit(&wt.commit)
            );
        }
    }

    Ok(())
}

/// Display worktrees in a tree structure based on parent/child relationships
fn display_tree_view(worktrees: &[Worktree]) {
    // Build parent-to-children map
    let mut children_map: HashMap<Option<String>, Vec<&Worktree>> = HashMap::new();

    for wt in worktrees {
        let parent_key = wt.parent.clone();
        children_map.entry(parent_key).or_default().push(wt);
    }

    println!("Worktrees:");

    // Display root worktrees (those without parents)
    if let Some(roots) = children_map.get(&None) {
        for root in roots {
            // Display root without tree characters
            println!(
                "{} ({}) [{}]",
                root.name,
                root.branch,
                short_commit(&root.commit)
            );

            // Display children of this root
            if let Some(children) = children_map.get(&Some(root.name.clone())) {
                let child_count = children.len();
                for (i, child) in children.iter().enumerate() {
                    let is_last_child = i == child_count - 1;
                    display_worktree_node(child, "", is_last_child, &children_map);
                }
            }
        }
    }
}

/// Recursively display a worktree node and its children
fn display_worktree_node(
    wt: &Worktree,
    prefix: &str,
    is_last: bool,
    children_map: &HashMap<Option<String>, Vec<&Worktree>>,
) {
    // Choose the appropriate tree characters
    let branch_char = if is_last { "└──" } else { "├──" };
    let continuation = if is_last { "    " } else { "│   " };

    // Display current worktree
    println!(
        "{}{} {} ({}) [{}]",
        prefix,
        branch_char,
        wt.name,
        wt.branch,
        short_commit(&wt.commit)
    );

    // Display children
    if let Some(children) = children_map.get(&Some(wt.name.clone())) {
        let child_count = children.len();
        for (i, child) in children.iter().enumerate() {
            let is_last_child = i == child_count - 1;
            let child_prefix = format!("{}{}", prefix, continuation);
            display_worktree_node(child, &child_prefix, is_last_child, children_map);
        }
    }
}
