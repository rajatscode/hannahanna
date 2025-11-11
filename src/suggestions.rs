// Error suggestion system for better user experience
use crate::errors::HnError;
use colored::Colorize;

/// Display an error with helpful suggestions
pub fn display_error_with_suggestions(error: &HnError) {
    eprintln!("\n{}: {}", "Error".bright_red().bold(), error);

    match error {
        HnError::WorktreeAlreadyExists(name) => {
            eprintln!("\n{}:", "Suggestions".bright_yellow());
            eprintln!(
                "  • Remove existing: {}",
                format!("hn remove {}", name).bright_cyan()
            );
            eprintln!(
                "  • Use different name: {}",
                format!("hn add {}-v2", name).bright_cyan()
            );
            eprintln!(
                "  • Switch to existing: {}",
                format!("hn switch {}", name).bright_cyan()
            );
        }

        HnError::WorktreeNotFound(name) => {
            eprintln!("\n{}:", "Suggestions".bright_yellow());
            eprintln!("  • List all worktrees: {}", "hn list".bright_cyan());
            eprintln!("  • Check the worktree name for typos");
            eprintln!(
                "  • Create new worktree: {}",
                format!("hn add {}", name).bright_cyan()
            );
        }

        HnError::AmbiguousWorktreeName(query, matches) => {
            eprintln!(
                "\n{} '{}':",
                "Multiple matches found for".bright_yellow(),
                query
            );
            for (i, m) in matches.iter().enumerate() {
                eprintln!("  {}. {}", i + 1, m.bright_cyan());
            }
            eprintln!("\n{}:", "Suggestions".bright_yellow());
            eprintln!("  • Use more specific name");
            eprintln!("  • Use exact worktree name from list above");
        }

        HnError::NoParent(_name) => {
            eprintln!("\n{}:", "Suggestions".bright_yellow());
            eprintln!("  • This worktree was not created from another worktree");
            eprintln!(
                "  • Use {} to switch to a specific worktree",
                "hn switch <name>".bright_cyan()
            );
            eprintln!("  • Use {} to see all worktrees", "hn list".bright_cyan());
        }

        HnError::NotInRepository => {
            eprintln!("\n{}:", "Suggestions".bright_yellow());
            eprintln!("  • Run this command from within a Git repository");
            eprintln!("  • Check if {} directory exists", ".git".bright_cyan());
            eprintln!("  • Initialize a repository: {}", "git init".bright_cyan());
        }

        HnError::InvalidWorktreeName(reason) => {
            eprintln!("\n{}:", "Suggestions".bright_yellow());
            eprintln!("  • Use only alphanumeric characters, hyphens, and underscores");
            eprintln!(
                "  • Example valid names: {}, {}, {}",
                "feature-x".bright_cyan(),
                "fix_bug_123".bright_cyan(),
                "hotfix-2024".bright_cyan()
            );
            if reason.contains("path") {
                eprintln!("  • Avoid characters with special meaning in file paths");
            }
        }

        HnError::Git(git_err) => {
            let err_msg = git_err.to_string();

            // Check for common git errors and provide suggestions
            if err_msg.contains("uncommitted changes") {
                eprintln!("\n{}:", "Suggestions".bright_yellow());
                eprintln!(
                    "  • Commit your changes: {}",
                    "git commit -am \"message\"".bright_cyan()
                );
                eprintln!("  • Stash your changes: {}", "git stash".bright_cyan());
                eprintln!(
                    "  • Force remove (discards changes): {}",
                    "hn remove <name> --force".bright_cyan()
                );
            } else if err_msg.contains("already exists") {
                eprintln!("\n{}:", "Suggestions".bright_yellow());
                eprintln!(
                    "  • Use {} to switch to existing worktree",
                    "hn switch <name>".bright_cyan()
                );
                eprintln!(
                    "  • Use {} to checkout existing branch",
                    "hn add <name> --no-branch".bright_cyan()
                );
                eprintln!("  • Use different worktree name");
            } else if err_msg.contains("merge") && err_msg.contains("conflict") {
                eprintln!("\n{}:", "Suggestions".bright_yellow());
                eprintln!("  • Resolve conflicts manually");
                eprintln!("  • Check status: {}", "git status".bright_cyan());
                eprintln!("  • Abort merge: {}", "git merge --abort".bright_cyan());
            }
        }

        HnError::ConfigError(_) => {
            eprintln!("\n{}:", "Suggestions".bright_yellow());
            eprintln!(
                "  • Check {} for syntax errors",
                ".hannahanna.yml".bright_cyan()
            );
            eprintln!("  • Validate YAML syntax online");
            eprintln!("  • See example config in documentation");
        }

        HnError::HookError(msg) => {
            eprintln!("\n{}:", "Suggestions".bright_yellow());
            eprintln!(
                "  • Check hook script in {}",
                ".hannahanna.yml".bright_cyan()
            );
            eprintln!("  • Run hook command manually to debug");
            if msg.contains("exit code") {
                eprintln!("  • Hook script returned non-zero exit code");
            }
        }

        HnError::PortAllocationError(msg) => {
            eprintln!("\n{}:", "Suggestions".bright_yellow());
            if msg.contains("exhausted") {
                eprintln!("  • Remove unused worktrees: {}", "hn list".bright_cyan());
                eprintln!(
                    "  • Configure wider port range in {}",
                    ".hannahanna.yml".bright_cyan()
                );
                eprintln!(
                    "  • Release ports from removed worktrees: {}",
                    "hn ports list".bright_cyan()
                );
            } else {
                eprintln!(
                    "  • Check port allocations: {}",
                    "hn ports list".bright_cyan()
                );
                eprintln!(
                    "  • Release ports: {}",
                    "hn ports release <name>".bright_cyan()
                );
            }
        }

        HnError::DockerError(msg) => {
            eprintln!("\n{}:", "Suggestions".bright_yellow());
            eprintln!("  • Check if Docker is running");
            if msg.contains("not found") || msg.contains("command not found") {
                eprintln!("  • Install Docker: https://docs.docker.com/get-docker/");
            } else if msg.contains("permission denied") {
                eprintln!(
                    "  • Add user to docker group: {}",
                    "sudo usermod -aG docker $USER".bright_cyan()
                );
                eprintln!("  • Or run with sudo (not recommended)");
            }
            eprintln!(
                "  • View Docker logs: {}",
                "hn docker logs <name>".bright_cyan()
            );
        }

        _ => {
            // No specific suggestions for this error type
        }
    }

    eprintln!(); // Empty line for better readability
}
