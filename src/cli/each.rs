// Execute a command in each worktree
use crate::errors::{HnError, Result};
use crate::vcs::git::GitBackend;
use crate::vcs::Worktree;
use colored::Colorize;
use regex::Regex;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn run(
    command: Vec<String>,
    parallel: bool,
    stop_on_error: bool,
    filter: Option<String>,
    docker_running: bool,
) -> Result<()> {
    if command.is_empty() {
        return Err(HnError::ConfigError(
            "No command specified. Usage: hn each <command>".to_string(),
        ));
    }

    let git = GitBackend::open_from_current_dir()?;
    let mut worktrees = git.list_worktrees()?;

    // Apply filter if provided
    if let Some(pattern) = filter {
        let regex = Regex::new(&pattern).map_err(|e| {
            HnError::ConfigError(format!("Invalid filter pattern '{}': {}", pattern, e))
        })?;
        worktrees.retain(|wt| regex.is_match(&wt.name));
    }

    // Filter for Docker running if requested
    if docker_running {
        // Get repo root for Docker status checking
        let repo_root = git.repo_root()?;
        let config = crate::config::Config::load(&repo_root)?;

        if config.docker.enabled {
            let state_dir = repo_root.join(".hn-state");
            let docker_manager =
                crate::docker::container::ContainerManager::new(&config.docker, &state_dir)?;

            let mut running_worktrees = Vec::new();
            for wt in worktrees {
                match docker_manager.get_status(&wt.name, &wt.path) {
                    Ok(status) if status.running => running_worktrees.push(wt),
                    _ => {}
                }
            }
            worktrees = running_worktrees;
        } else {
            eprintln!("Warning: --docker-running specified but Docker is not enabled");
            return Ok(());
        }
    }

    if worktrees.is_empty() {
        eprintln!("No worktrees found matching criteria");
        return Ok(());
    }

    eprintln!(
        "Executing command in {} worktree{}...\n",
        worktrees.len(),
        if worktrees.len() == 1 { "" } else { "s" }
    );

    if parallel {
        run_parallel(&worktrees, &command, stop_on_error)
    } else {
        run_sequential(&worktrees, &command, stop_on_error)
    }
}

fn run_sequential(worktrees: &[Worktree], command: &[String], stop_on_error: bool) -> Result<()> {
    let mut had_errors = false;

    for wt in worktrees {
        print_separator(&wt.name);

        match execute_in_worktree(wt, command) {
            Ok(success) => {
                if !success {
                    had_errors = true;
                    if stop_on_error {
                        return Err(HnError::CommandFailed(format!(
                            "Command failed in worktree '{}'",
                            wt.name
                        )));
                    }
                }
            }
            Err(e) => {
                eprintln!("{}", format!("Error: {}", e).red());
                had_errors = true;
                if stop_on_error {
                    return Err(e);
                }
            }
        }
        println!();
    }

    if had_errors && !stop_on_error {
        eprintln!("{}", "⚠ Some commands failed".yellow());
    }

    Ok(())
}

fn run_parallel(worktrees: &[Worktree], command: &[String], stop_on_error: bool) -> Result<()> {
    use std::thread;

    let had_errors = Arc::new(AtomicBool::new(false));
    let should_stop = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();

    for wt in worktrees {
        let wt = wt.clone();
        let command = command.to_vec();
        let had_errors = Arc::clone(&had_errors);
        let should_stop = Arc::clone(&should_stop);

        let handle = thread::spawn(move || {
            if stop_on_error && should_stop.load(Ordering::Relaxed) {
                return;
            }

            print_separator(&wt.name);

            match execute_in_worktree(&wt, &command) {
                Ok(success) => {
                    if !success {
                        had_errors.store(true, Ordering::Relaxed);
                        if stop_on_error {
                            should_stop.store(true, Ordering::Relaxed);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", format!("Error: {}", e).red());
                    had_errors.store(true, Ordering::Relaxed);
                    if stop_on_error {
                        should_stop.store(true, Ordering::Relaxed);
                    }
                }
            }
            println!();
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    if had_errors.load(Ordering::Relaxed) {
        if stop_on_error {
            return Err(HnError::CommandFailed(
                "Command failed in one or more worktrees".to_string(),
            ));
        } else {
            eprintln!("{}", "⚠ Some commands failed".yellow());
        }
    }

    Ok(())
}

fn print_separator(worktree_name: &str) {
    println!(
        "{}",
        format!("==> {} <==", worktree_name).bright_cyan().bold()
    );
}

fn execute_in_worktree(wt: &Worktree, command: &[String]) -> Result<bool> {
    let program = &command[0];
    let args = &command[1..];

    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(&wt.path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd.status().map_err(|e| {
        HnError::CommandFailed(format!(
            "Failed to execute '{}' in '{}': {}",
            command.join(" "),
            wt.name,
            e
        ))
    })?;

    Ok(status.success())
}
