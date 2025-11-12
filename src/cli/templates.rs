// Template management CLI commands (v0.5)

use crate::config::Config;
use crate::errors::{HnError, Result};
use crate::templates;
use colored::*;
use std::env;
use std::fs;

/// List all available templates
pub fn list(json: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;

    // Load templates
    let templates_list = templates::list_templates(&repo_root)?;

    if json {
        // JSON output
        let json_output = serde_json::to_string_pretty(&templates_list)?;
        println!("{}", json_output);
        return Ok(());
    }

    // Table output
    if templates_list.is_empty() {
        println!("{}", "No templates found".yellow());
        println!();
        println!("Create templates in: {}", format!("{}/.hn-templates/", repo_root.display()).cyan());
        println!();
        println!("Example structure:");
        println!("  .hn-templates/");
        println!("    microservice/");
        println!("      .hannahanna.yml  {} Template configuration", "←".dimmed());
        println!("      README.md         {} Template description", "←".dimmed());
        return Ok(());
    }

    println!();
    println!("{}", "Available Templates".bold());
    println!("{}", "═".repeat(60));

    for template in &templates_list {
        // Template name
        print!("{:<20}", template.name.cyan().bold());

        // Description (first line only)
        if let Some(ref desc) = template.description {
            let first_line = desc.lines().next().unwrap_or("");
            print!(" │ {}", first_line.dimmed());
        }

        println!();
    }

    println!("{}", "═".repeat(60));
    println!(
        "{} template{} found in {}",
        templates_list.len().to_string().green().bold(),
        if templates_list.len() == 1 { "" } else { "s" },
        ".hn-templates/".cyan()
    );
    println!();
    println!("Usage: {} <name> {} <template-name>", "hn add".bold(), "--template".dimmed());

    Ok(())
}

/// Show details about a specific template
pub fn show(name: &str) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;

    // Load templates
    let templates_list = templates::list_templates(&repo_root)?;

    // Find the template
    let template = templates_list
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| {
            HnError::TemplateError(format!(
                "Template '{}' not found. Run 'hn templates list' to see available templates.",
                name
            ))
        })?;

    println!();
    println!("{}: {}", "Template".bold(), template.name.cyan().bold());
    println!("{}: {}", "Location".bold(), template.config_path.parent().unwrap().display().to_string().dimmed());
    println!();

    // Show description
    if let Some(ref desc) = template.description {
        println!("{}", "Description".bold());
        println!("{}", "─".repeat(60));
        for line in desc.lines().take(10) {
            // Limit to 10 lines
            println!("{}", line);
        }
        println!();
    }

    // Show configuration preview
    println!("{}", "Configuration".bold());
    println!("{}", "─".repeat(60));

    if template.config_path.exists() {
        let config_content = fs::read_to_string(&template.config_path)?;
        let lines: Vec<&str> = config_content.lines().collect();

        // Show first 15 lines
        for line in lines.iter().take(15) {
            println!("{}", line.dimmed());
        }

        if lines.len() > 15 {
            println!("{}", format!("... ({} more lines)", lines.len() - 15).dimmed());
        }
    } else {
        println!("{}", "No configuration file found".yellow());
    }

    println!();
    println!("{}", "Usage".bold());
    println!("{}", "─".repeat(60));
    println!("  {} <worktree-name> {} {}", "hn add".bold(), "--template".dimmed(), name.cyan());
    println!();

    Ok(())
}

/// Create a new template
pub fn create(name: &str, description: Option<&str>, enable_docker: bool, from_current: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo_root = Config::find_repo_root(&cwd)?;

    // Validate template name
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.starts_with('.') {
        return Err(HnError::TemplateError(format!(
            "Invalid template name '{}'. Template names must be simple directory names without special characters.",
            name
        )));
    }

    // Create templates directory if it doesn't exist
    let templates_dir = repo_root.join(".hn-templates");
    if !templates_dir.exists() {
        fs::create_dir_all(&templates_dir)?;
    }

    // Create template directory
    let template_dir = templates_dir.join(name);
    if template_dir.exists() {
        return Err(HnError::TemplateError(format!(
            "Template '{}' already exists at {}",
            name,
            template_dir.display()
        )));
    }

    fs::create_dir_all(&template_dir)?;

    // Generate config content
    let config_content = if from_current {
        // Copy from current .hannahanna.yml if it exists
        let current_config = repo_root.join(".hannahanna.yml");
        if current_config.exists() {
            fs::read_to_string(&current_config)?
        } else {
            generate_template_config(enable_docker)
        }
    } else {
        generate_template_config(enable_docker)
    };

    // Write config file
    fs::write(template_dir.join(".hannahanna.yml"), config_content)?;

    // Generate README
    let readme_content = generate_readme(name, description);
    fs::write(template_dir.join("README.md"), readme_content)?;

    // Create files/ directory for template file copying
    fs::create_dir_all(template_dir.join("files"))?;

    println!();
    println!("{} Template '{}' created successfully!", "✓".green().bold(), name.cyan().bold());
    println!();
    println!("{}: {}", "Location".bold(), template_dir.display().to_string().dimmed());
    println!();
    println!("{}", "Next steps:".bold());
    println!("  1. Edit {}/.hannahanna.yml to customize configuration", template_dir.display());
    println!("  2. Add files to {}/ to copy to new worktrees", template_dir.join("files").display());
    println!("  3. Use with: {} <name> {} {}", "hn add".bold(), "--template".dimmed(), name.cyan());
    println!();

    Ok(())
}

/// Generate template configuration
fn generate_template_config(enable_docker: bool) -> String {
    if enable_docker {
        r#"# Template configuration
# This template includes Docker support

docker:
  enabled: true
  services:
    - app
  ports:
    app: auto

hooks:
  post_create: |
    echo "Setting up Docker environment..."
    # Add your setup commands here
"#.to_string()
    } else {
        r#"# Template configuration

hooks:
  post_create: |
    echo "Worktree created from template"
    # Add your setup commands here
"#.to_string()
    }
}

/// Generate README for template
fn generate_readme(name: &str, description: Option<&str>) -> String {
    let desc = description.unwrap_or("Template for hannahanna worktrees");

    format!(
        r#"# {} Template

{}

## Usage

```bash
hn add <worktree-name> --template {}
```

## Configuration

See `.hannahanna.yml` for template configuration.

## Files

Any files in the `files/` directory will be copied to new worktrees created with this template.

## Customization

Edit `.hannahanna.yml` to customize:
- Hooks (setup commands)
- Docker configuration
- Environment variables
- Sparse checkout paths
"#,
        name, desc, name
    )
}
