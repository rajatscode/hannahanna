use crate::config::Config;
use crate::docker::compose::ComposeGenerator;
use crate::docker::container::ContainerManager;
use crate::docker::ports::PortAllocator;
use crate::env::copy::{CopyAction, CopyManager};
use crate::env::symlinks::{SymlinkAction, SymlinkManager};
use crate::env::validation;
use crate::errors::Result;
use crate::hooks::{HookExecutor, HookType};
use crate::monitoring::{self, ActivityEvent};
use crate::state::StateManager;
use crate::vcs::{init_backend_from_current_dir, RegistryCache, VcsType};
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

#[allow(clippy::too_many_arguments)]
pub fn run(
    name: Option<String>,
    branch: Option<String>,
    from: Option<String>,
    no_branch: bool,
    sparse_paths: Option<Vec<String>>,
    template: Option<String>,
    template_params: Option<Vec<String>>,
    profile: Option<String>,
    no_hooks: bool,
    vcs_type: Option<VcsType>,
) -> Result<()> {
    // Interactive mode if name is not provided
    let (name, branch, from, no_branch, sparse_paths, template) = if name.is_none() {
        interactive_prompts(branch, from, no_branch, sparse_paths, template)?
    } else {
        (
            name.unwrap(),
            branch,
            from,
            no_branch,
            sparse_paths,
            template,
        )
    };

    // Validate worktree name
    validation::validate_worktree_name(&name)?;

    // Initialize VCS backend (auto-detect or use explicit type)
    let backend = if let Some(vcs) = vcs_type {
        crate::vcs::init_backend_with_detection(&std::env::current_dir()?, Some(vcs))?
    } else {
        init_backend_from_current_dir()?
    };

    // Find repository root
    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;

    // Load configuration
    let mut config = Config::load(&repo_root)?;

    // Apply profile if specified (v0.6)
    if let Some(ref profile_name) = profile {
        eprintln!("Applying profile '{}'...", profile_name);
        config.apply_profile(profile_name)?;
        eprintln!("✓ Profile '{}' applied", profile_name);
    }

    // Run pre_create hook if configured
    let has_pre_create_hooks = config.hooks.pre_create.is_some()
        || !config.hooks.pre_create_conditions.is_empty();

    if has_pre_create_hooks && !no_hooks {
        eprintln!("Running pre_create hook...");

        // Create a temporary worktree struct for the hook
        // We don't have all the info yet, but we have enough for the hook to use
        let current_workspace = backend.get_current_workspace().ok();
        let current_branch = current_workspace
            .as_ref()
            .map(|w| w.branch.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let effective_branch = branch.clone()
            .or_else(|| from.clone())
            .unwrap_or_else(|| current_branch.clone());

        let temp_worktree = crate::vcs::Worktree {
            name: name.clone(),
            path: repo_root.join(&name), // Estimated path
            branch: effective_branch,
            commit: String::new(), // Not known yet
            parent: None, // Will be set later
        };

        let state_dir = repo_root.join(".hn-state").join(&name);
        let hook_executor = HookExecutor::new(config.hooks.clone(), no_hooks);
        hook_executor.run_hook(HookType::PreCreate, &temp_worktree, &state_dir)?;
        eprintln!("✓ Pre-create hook completed successfully");
    } else if has_pre_create_hooks && no_hooks {
        eprintln!("⚠ Skipping pre_create hook (--no-hooks)");
    }

    // Create the worktree
    eprintln!("Creating worktree '{}'...", name);
    let worktree =
        backend.create_workspace(&name, branch.as_deref(), from.as_deref(), no_branch)?;
    eprintln!("✓ Worktree created at {}", worktree.path.display());

    // Log worktree creation activity
    let state_dir_path = repo_root.join(".hn-state");
    let _ = monitoring::log_activity(
        &state_dir_path,
        &name,
        ActivityEvent::WorktreeCreated {
            timestamp: monitoring::now(),
            branch: worktree.branch.clone(),
            template: template.clone(),
        },
    );

    // Invalidate cache after creating worktree
    if let Ok(cache) = RegistryCache::new(&state_dir_path, None) {
        let _ = cache.invalidate(); // Ignore cache invalidation errors
    }

    // Setup sparse checkout if requested
    // Priority: CLI flag > config default
    let effective_sparse_paths: &[String] = if let Some(ref cli_paths) = sparse_paths {
        // CLI override
        cli_paths
    } else if config.sparse.enabled && !config.sparse.paths.is_empty() {
        // Use config default
        &config.sparse.paths
    } else {
        &[]
    };

    if !effective_sparse_paths.is_empty() {
        eprintln!("Setting up sparse checkout...");
        match backend.setup_sparse_checkout(&worktree.path, effective_sparse_paths) {
            Ok(_) => {
                eprintln!("✓ Sparse checkout configured:");
                for path in effective_sparse_paths {
                    eprintln!("  - {}", path);
                }
            }
            Err(e) => {
                eprintln!("⚠ Sparse checkout failed: {}", e);
                eprintln!("  Continuing with full checkout...");
            }
        }
    }

    // Create state directory
    let state_manager = StateManager::new(&repo_root)?;
    let state_dir = state_manager.create_state_dir(&name)?;

    // Setup symlinks for shared resources
    if !config.shared_resources.is_empty() {
        let actions = SymlinkManager::setup(&config.shared_resources, &repo_root, &worktree.path)?;

        for action in actions {
            match action {
                SymlinkAction::Created { source, target: _ } => {
                    eprintln!(
                        "✓ Shared {} (symlinked)",
                        source.file_name().unwrap().to_string_lossy()
                    );
                }
                SymlinkAction::Skipped { resource, reason } => {
                    eprintln!("⚠ Skipped {} ({})", resource, reason);
                }
            }
        }
    }

    // Setup file copies from shared.copy configuration
    if let Some(ref shared) = config.shared {
        if !shared.copy.is_empty() {
            let actions = CopyManager::setup(&shared.copy, &repo_root, &worktree.path)?;

            for action in actions {
                match action {
                    CopyAction::Copied { source, target: _ } => {
                        eprintln!(
                            "✓ Copied {} to worktree",
                            source.file_name().unwrap().to_string_lossy()
                        );
                    }
                    CopyAction::Skipped { resource, reason } => {
                        eprintln!("⚠ Skipped copying {} ({})", resource, reason);
                    }
                }
            }
        }
    }

    // Run post_create hook if configured (regular or conditional)
    let has_post_create_hooks = config.hooks.post_create.is_some()
        || !config.hooks.post_create_conditions.is_empty();

    if has_post_create_hooks && !no_hooks {
        eprintln!("Running post_create hook...");
        let hook_executor = HookExecutor::new(config.hooks.clone(), no_hooks);
        hook_executor.run_hook(HookType::PostCreate, &worktree, &state_dir)?;
        eprintln!("✓ Hook completed successfully");
    } else if has_post_create_hooks && no_hooks {
        eprintln!("⚠ Skipping post_create hook (--no-hooks)");
    }

    // Apply template if specified
    if let Some(template_name) = template {
        eprintln!("\nApplying template '{}'...", template_name);

        // First apply hannahanna config from template (v0.5)
        crate::templates::apply_template(&repo_root, &worktree.path, &template_name)?;

        // Then copy template files with parameterization (v0.6)
        let params = template_params.unwrap_or_default();
        crate::templates::apply_template_with_parameters(
            &repo_root,
            &worktree.path,
            &template_name,
            &name,
            &params,
        )?;
    }

    // Docker integration
    if config.docker.enabled {
        eprintln!("\nSetting up Docker...");

        // Allocate ports
        let state_dir_path = repo_root.join(".hn-state");
        let mut port_allocator = PortAllocator::new(&state_dir_path)?;

        // Get services from config or use defaults
        let services: Vec<&str> = config
            .docker
            .ports
            .base
            .keys()
            .map(|s| s.as_str())
            .collect();

        let ports = port_allocator.allocate(&name, &services)?;

        // Display allocated ports
        for (service, port) in &ports {
            eprintln!("  {} port: {}", service, port);
        }

        // Generate docker-compose.override.yml
        let compose_gen = ComposeGenerator::new(&config.docker, &state_dir_path);
        compose_gen.save(&name, &worktree.path, &ports)?;
        eprintln!("✓ Generated docker-compose.override.yml");

        // Auto-start containers if configured
        if config.docker.auto_start {
            eprintln!("Starting Docker containers...");
            let container_mgr = ContainerManager::new(&config.docker, &state_dir_path)?;

            match container_mgr.start(&name, &worktree.path) {
                Ok(_) => eprintln!("✓ Containers started"),
                Err(e) => eprintln!("⚠ Failed to start containers: {}", e),
            }
        }
    }

    eprintln!("\nDone! Switch to the worktree with:");
    eprintln!("  hn switch {}", name);

    Ok(())
}

/// Interactive prompts for creating a worktree
fn interactive_prompts(
    branch: Option<String>,
    from: Option<String>,
    no_branch: bool,
    sparse_paths: Option<Vec<String>>,
    template: Option<String>,
) -> Result<(
    String,
    Option<String>,
    Option<String>,
    bool,
    Option<Vec<String>>,
    Option<String>,
)> {
    let theme = ColorfulTheme::default();

    // Header
    eprintln!("{}", "═".repeat(60).bright_blue());
    eprintln!("{}", "  Interactive Worktree Creation".bold().bright_green());
    eprintln!("{}", "═".repeat(60).bright_blue());
    eprintln!();

    // Section 1: Basic Information
    eprintln!("{}", "📋 Basic Information".bold().cyan());
    eprintln!("{}", "─".repeat(60).bright_black());
    eprintln!();

    // Prompt for name
    let name: String = Input::with_theme(&theme)
        .with_prompt("Worktree name")
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if validation::validate_worktree_name(input).is_ok() {
                Ok(())
            } else {
                Err("Invalid name (use lowercase letters, numbers, hyphens, underscores)")
            }
        })
        .interact_text()?;

    // Prompt for branch name (if not provided via flag)
    let branch = if branch.is_some() {
        branch
    } else if Confirm::with_theme(&theme)
        .with_prompt("Create new branch?")
        .default(true)
        .interact()?
    {
        Some(
            Input::with_theme(&theme)
                .with_prompt("Branch name")
                .default(name.clone())
                .interact_text()?,
        )
    } else {
        None
    };

    // Determine no_branch flag
    let no_branch = if no_branch {
        true
    } else {
        branch.is_none()
    };

    // Prompt for base branch (if not provided via flag)
    let from = if from.is_some() {
        from
    } else if Confirm::with_theme(&theme)
        .with_prompt("Create from specific branch?")
        .default(false)
        .interact()?
    {
        Some(
            Input::with_theme(&theme)
                .with_prompt("Base branch")
                .default("main".to_string())
                .interact_text()?,
        )
    } else {
        None
    };

    eprintln!();

    // Section 2: Template Selection
    eprintln!("{}", "📦 Template Configuration".bold().cyan());
    eprintln!("{}", "─".repeat(60).bright_black());
    eprintln!();

    // List available templates
    let template = if template.is_some() {
        template
    } else {
        let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;
        match crate::templates::list_templates(&repo_root) {
            Ok(templates) if !templates.is_empty() => {
                if Confirm::with_theme(&theme)
                    .with_prompt("Use a template?")
                    .default(false)
                    .interact()?
                {
                    let template_names: Vec<String> =
                        templates.iter().map(|t| t.name.clone()).collect();
                    let selection = Select::with_theme(&theme)
                        .with_prompt("Select template")
                        .items(&template_names)
                        .default(0)
                        .interact()?;
                    Some(template_names[selection].clone())
                } else {
                    None
                }
            }
            _ => {
                eprintln!("{}", "No templates found in .hn-templates/".dimmed());
                None
            }
        }
    };

    eprintln!();

    // Section 3: Advanced Options
    eprintln!("{}", "⚙️  Advanced Options".bold().cyan());
    eprintln!("{}", "─".repeat(60).bright_black());
    eprintln!();

    // Prompt for sparse checkout
    let sparse_paths = if sparse_paths.is_some() {
        sparse_paths
    } else if Confirm::with_theme(&theme)
        .with_prompt("Use sparse checkout? (for large monorepos)")
        .default(false)
        .interact()?
    {
        let mut paths = Vec::new();
        loop {
            let path: String = Input::with_theme(&theme)
                .with_prompt("Path (empty to finish)")
                .allow_empty(true)
                .interact_text()?;
            if path.is_empty() {
                break;
            }
            paths.push(path);
        }
        if paths.is_empty() {
            None
        } else {
            Some(paths)
        }
    } else {
        None
    };

    eprintln!();

    // Section 4: Summary & Confirmation
    eprintln!("{}", "═".repeat(60).bright_blue());
    eprintln!("{}", "  Configuration Summary".bold().bright_yellow());
    eprintln!("{}", "═".repeat(60).bright_blue());
    eprintln!();

    // Display summary
    eprintln!("  {} {}", "Name:".bold(), name.bright_white());
    eprintln!("  {} {}", "Branch:".bold(),
        branch.as_deref().unwrap_or("(existing branch)").bright_white());
    if let Some(ref from_branch) = from {
        eprintln!("  {} {}", "From:".bold(), from_branch.bright_white());
    }
    if let Some(ref tmpl) = template {
        eprintln!("  {} {}", "Template:".bold(), tmpl.bright_green());
    }
    if let Some(ref paths) = sparse_paths {
        eprintln!("  {} {} paths", "Sparse:".bold(), paths.len().to_string().bright_white());
        for path in paths {
            eprintln!("    • {}", path.dimmed());
        }
    }

    // Check Docker configuration from config
    let repo_root = Config::find_repo_root(&std::env::current_dir()?)?;
    if let Ok(config) = Config::load(&repo_root) {
        if config.docker.enabled {
            eprintln!("  {} {}", "Docker:".bold(), "enabled".bright_green());
            if !config.docker.ports.base.is_empty() {
                eprintln!("    Services: {}",
                    config.docker.ports.base.keys()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                        .dimmed());
            }
        } else {
            eprintln!("  {} {}", "Docker:".bold(), "disabled".dimmed());
        }

        // Show hooks status
        let has_hooks = config.hooks.post_create.is_some()
            || !config.hooks.post_create_conditions.is_empty()
            || config.hooks.pre_create.is_some()
            || !config.hooks.pre_create_conditions.is_empty();
        if has_hooks {
            eprintln!("  {} {}", "Hooks:".bold(), "enabled".bright_green());
        }
    }

    eprintln!();
    eprintln!("{}", "═".repeat(60).bright_blue());
    eprintln!();

    // Final confirmation
    if !Confirm::with_theme(&theme)
        .with_prompt("Create worktree with this configuration?")
        .default(true)
        .interact()?
    {
        eprintln!();
        eprintln!("{}", "Worktree creation cancelled.".yellow());
        std::process::exit(0);
    }

    eprintln!();
    Ok((name, branch, from, no_branch, sparse_paths, template))
}
