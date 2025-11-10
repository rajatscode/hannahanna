use crate::errors::Result;
use crate::vcs::git::GitBackend;

pub fn run(tree: bool) -> Result<()> {
    let git = GitBackend::open_from_current_dir()?;
    let worktrees = git.list_worktrees()?;

    if tree {
        // Tree view - for now just show a simple list
        // TODO: Implement parent/child relationships when git config tracking is added
        println!("Worktrees:");
        for wt in worktrees {
            println!(
                "  {} ({}) [{}]",
                wt.name,
                wt.branch,
                &wt.commit[..7.min(wt.commit.len())]
            );
        }
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
                &wt.commit[..7.min(wt.commit.len())] // Short hash
            );
        }
    }

    Ok(())
}
