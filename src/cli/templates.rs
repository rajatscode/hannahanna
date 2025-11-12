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
