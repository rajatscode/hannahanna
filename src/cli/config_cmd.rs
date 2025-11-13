// Config command: Manage hannahanna configuration
use crate::config::Config;
use crate::errors::{HnError, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;
use std::process::Command;

const CONFIG_FILE: &str = ".hannahanna.yml";

const TEMPLATE_CONFIG: &str = r#"# hannahanna configuration file
#
# This file configures how worktrees share resources and manage their environments.
# Learn more: https://docs.hannahanna.dev/configuration

# Shared resources (symlinked from main repository)
shared_resources:
  - source: node_modules
    target: node_modules
    compatibility: package-lock.json  # Only share if this file is identical
  # - source: vendor
  #   target: vendor
  #   compatibility: composer.lock

# Files to copy (not symlink) to each worktree
shared:
  copy:
    - .env.template -> .env
    # - config/local.yml.example -> config/local.yml

# Lifecycle hooks
hooks:
  # Run before worktree creation
  # pre_create: |
  #   echo "Preparing to create worktree..."

  # Run after worktree creation
  post_create: |
    echo "✓ Worktree created successfully!"
    # npm install
    # make setup

  # Run before worktree removal
  pre_remove: |
    echo "Cleaning up worktree..."
    # make cleanup

  # Run after worktree removal
  # post_remove: |
  #   echo "Worktree removed successfully"

  # Run after switching to a worktree
  # post_switch: |
  #   echo "Switched to worktree successfully"

  # Run before merge/integrate operations
  # pre_integrate: |
  #   echo "Preparing for integration..."

  # Run after merge/integrate operations
  # post_integrate: |
  #   echo "Integration complete!"

  # Conditional hooks based on branch name patterns
  # post_create_conditions:
  #   - condition: "branch.startsWith('feature-')"
  #     command: "make setup-dev"
  #   - condition: "branch.contains('api')"
  #     command: "docker compose up -d api-deps"

  # Hook execution timeout in seconds (default: 300 = 5 minutes)
  # Prevents hooks from hanging indefinitely
  timeout_seconds: 300

# Docker configuration (optional)
docker:
  enabled: false
  compose_file: docker-compose.yml

  # Port allocation strategy
  ports:
    base: 3000
    services:
      app: 3000
      postgres: 5432
      redis: 6379

  # Shared resources across all worktrees
  shared:
    volumes:
      - postgres-data
      - redis-data
    networks:
      - myapp-net

  # Isolated resources per worktree
  isolated:
    volumes:
      - app-cache
      - logs

# Command aliases
aliases:
  # Short aliases for common commands
  # sw: switch
  # ls: list
  # rm: remove

  # Aliases with arguments
  # lt: list --tree
  # stat: state list

  # Chained aliases (aliases can reference other aliases)
  # Note: Circular references are detected and will cause an error
"#;

/// Initialize a new config file
pub fn init() -> Result<()> {
    let config_path = Path::new(CONFIG_FILE);

    if config_path.exists() {
        return Err(HnError::ConfigError(format!(
            "{} already exists. Remove it first or edit manually.",
            CONFIG_FILE
        )));
    }

    // Write template to file
    fs::write(config_path, TEMPLATE_CONFIG)?;

    println!("{}", "✓ Configuration file created!".bright_green());
    println!("\nCreated: {}", CONFIG_FILE.bright_cyan());
    println!("\nNext steps:");
    println!(
        "  1. Review the configuration: {}",
        format!("cat {}", CONFIG_FILE).bright_cyan()
    );
    println!("  2. Customize for your project");
    println!("  3. Commit to version control");
    println!(
        "\nValidate your config anytime with: {}",
        "hn config validate".bright_cyan()
    );

    Ok(())
}

/// Validate config file syntax
pub fn validate() -> Result<()> {
    let config_path = Path::new(CONFIG_FILE);

    if !config_path.exists() {
        println!("{}", "⚠ No configuration file found".bright_yellow());
        println!("\n{}:", "Suggestions".bright_yellow());
        println!("  • Create one: {}", "hn config init".bright_cyan());
        println!("  • hannahanna works without a config file");
        println!("  • Config is optional for customization");
        return Ok(());
    }

    // Try to load and parse the config
    print!("Validating {}...", CONFIG_FILE);

    let current_dir = std::env::current_dir()?;
    match Config::load(&current_dir) {
        Ok(config) => {
            println!(" {}", "✓".bright_green());
            println!("\n{}", "Configuration is valid!".bright_green().bold());

            // Show summary
            println!("\n{}:", "Summary".bright_cyan().bold());

            // Shared resources
            if !config.shared_resources.is_empty() {
                println!(
                    "  • {} shared resources configured",
                    config.shared_resources.len()
                );
                for resource in &config.shared_resources {
                    print!("    - {}", resource.source);
                    if let Some(compat) = &resource.compatibility {
                        print!(" (compatibility: {})", compat);
                    }
                    println!();
                }
            } else {
                println!("  • No shared resources configured");
            }

            // Copied files
            if let Some(shared) = &config.shared {
                if !shared.copy.is_empty() {
                    println!("  • {} files to copy", shared.copy.len());
                    for copy_resource in &shared.copy {
                        println!("    - {} -> {}", copy_resource.source, copy_resource.target);
                    }
                }
            }

            // Hooks
            let hooks_configured = config.hooks.pre_create.is_some()
                || config.hooks.post_create.is_some()
                || config.hooks.pre_remove.is_some()
                || config.hooks.post_remove.is_some()
                || config.hooks.post_switch.is_some()
                || config.hooks.pre_integrate.is_some()
                || config.hooks.post_integrate.is_some()
                || !config.hooks.pre_create_conditions.is_empty()
                || !config.hooks.post_create_conditions.is_empty()
                || !config.hooks.pre_remove_conditions.is_empty()
                || !config.hooks.post_remove_conditions.is_empty()
                || !config.hooks.post_switch_conditions.is_empty()
                || !config.hooks.pre_integrate_conditions.is_empty()
                || !config.hooks.post_integrate_conditions.is_empty();

            if hooks_configured {
                println!("  • Lifecycle hooks configured");
                if config.hooks.pre_create.is_some() {
                    println!("    - pre_create");
                }
                if config.hooks.post_create.is_some() {
                    println!("    - post_create");
                }
                if config.hooks.pre_remove.is_some() {
                    println!("    - pre_remove");
                }
                if config.hooks.post_remove.is_some() {
                    println!("    - post_remove");
                }
                if config.hooks.post_switch.is_some() {
                    println!("    - post_switch");
                }
                if config.hooks.pre_integrate.is_some() {
                    println!("    - pre_integrate");
                }
                if config.hooks.post_integrate.is_some() {
                    println!("    - post_integrate");
                }

                // Show conditional hooks count
                let conditional_count = config.hooks.pre_create_conditions.len()
                    + config.hooks.post_create_conditions.len()
                    + config.hooks.pre_remove_conditions.len()
                    + config.hooks.post_remove_conditions.len()
                    + config.hooks.post_switch_conditions.len()
                    + config.hooks.pre_integrate_conditions.len()
                    + config.hooks.post_integrate_conditions.len();
                if conditional_count > 0 {
                    println!("    - {} conditional hooks", conditional_count);
                }
            } else {
                println!("  • No hooks configured");
            }

            // Docker
            if config.docker.enabled {
                println!("  • Docker integration enabled");
                println!("    - Compose file: {}", config.docker.compose_file);
            } else {
                println!("  • Docker integration disabled");
            }

            Ok(())
        }
        Err(e) => {
            println!(" {}", "✗".bright_red());
            println!(
                "\n{}: Configuration is invalid",
                "Error".bright_red().bold()
            );
            println!("\n{}", e);

            println!("\n{}:", "Suggestions".bright_yellow());
            println!("  • Check YAML syntax");
            println!(
                "  • Validate online: {}",
                "https://www.yamllint.com/".bright_cyan()
            );
            println!(
                "  • See example: {}",
                "hn config init --force".bright_cyan()
            );
            println!(
                "  • Edit config: {}",
                format!("$EDITOR {}", CONFIG_FILE).bright_cyan()
            );

            Err(e)
        }
    }
}

/// Show current configuration
pub fn show() -> Result<()> {
    let current_dir = std::env::current_dir()?;

    // Get list of loaded config files
    let loaded_paths = Config::get_loaded_config_paths(&current_dir);

    if loaded_paths.is_empty() {
        println!("{}", "⚠ No configuration files found".bright_yellow());
        println!("\n{}:", "Suggestions".bright_yellow());
        println!("  • Create one: {}", "hn config init".bright_cyan());
        println!("  • hannahanna uses default configuration");
        println!("  • Config files are optional");
        println!(
            "\n{}:",
            "Config hierarchy (highest priority first)".bright_cyan()
        );
        println!("  1. .hannahanna.local.yml  (local, gitignored)");
        println!("  2. .hannahanna.yml        (repo, committed)");
        println!("  3. ~/.config/hannahanna/config.yml  (user)");
        println!("  4. /etc/hannahanna/config.yml       (system)");
        return Ok(());
    }

    // Load and display the merged config
    let config = Config::load(&current_dir)?;

    println!("{}", "Merged Configuration".bright_cyan().bold());
    println!("{}", "=".repeat(70));

    // Display which configs were loaded
    println!(
        "\n{}:",
        "Loaded config files (highest priority first)".bright_green()
    );
    for (i, path) in loaded_paths.iter().enumerate() {
        let priority = match i {
            0 => "HIGHEST",
            _ if i == loaded_paths.len() - 1 => "LOWEST",
            _ => "MEDIUM",
        };
        println!(
            "  {}. {} {}",
            i + 1,
            path.display().to_string().bright_cyan(),
            format!("[{}]", priority).bright_yellow()
        );
    }

    println!("\n{}", "=".repeat(70));
    println!("{}", "Merged Result:".bright_cyan().bold());
    println!("{}", "=".repeat(70));

    // Display as formatted YAML
    let yaml = serde_yml::to_string(&config)
        .map_err(|e| HnError::ConfigError(format!("Failed to serialize config: {}", e)))?;

    println!("{}", yaml);

    println!("{}", "=".repeat(70));
    println!("\n{}:", "Info".bright_cyan());
    println!("  • {} config files merged", loaded_paths.len());
    println!("  • Merge strategy: Arrays append, primitives override");
    println!("  • Validate: {}", "hn config validate".bright_cyan());
    println!(
        "  • Edit repo config: {}",
        format!("$EDITOR {}", CONFIG_FILE).bright_cyan()
    );
    println!(
        "  • Create local override: {}",
        "$EDITOR .hannahanna.local.yml".bright_cyan()
    );

    Ok(())
}

/// Edit configuration file in $EDITOR
pub fn edit() -> Result<()> {
    let config_path = Path::new(CONFIG_FILE);

    if !config_path.exists() {
        println!("{}", "⚠ No configuration file found".bright_yellow());
        println!("\nCreate one first: {}", "hn config init".bright_cyan());
        return Err(HnError::ConfigError(format!(
            "{} does not exist",
            CONFIG_FILE
        )));
    }

    // Get editor from environment or use default
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    println!("Opening {} in {}...", CONFIG_FILE, editor);

    // Open editor
    let status = Command::new(&editor)
        .arg(config_path)
        .status()
        .map_err(|e| HnError::ConfigError(format!("Failed to open editor: {}", e)))?;

    if !status.success() {
        return Err(HnError::ConfigError(
            "Editor exited with non-zero status".to_string(),
        ));
    }

    // Validate after editing
    println!("\nValidating changes...");
    validate()?;

    Ok(())
}
