// Template system for pre-configured worktree environments
//
// Templates allow users to define pre-configured setups for different
// types of worktrees (e.g., "microservice", "frontend", "experiment")

use crate::config::Config;
use crate::errors::{HnError, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Template metadata and configuration
#[derive(Debug, Clone)]
pub struct Template {
    #[allow(dead_code)] // Used by list_templates(), reserved for v0.4.1 `hn templates list`
    pub name: String,
    pub description: Option<String>,
    pub config_path: PathBuf,
}

/// Find available templates in the repository
/// Reserved for v0.4.1 `hn templates list` command
#[allow(dead_code)]
pub fn list_templates(repo_root: &Path) -> Result<Vec<Template>> {
    let templates_dir = repo_root.join(".hn-templates");

    if !templates_dir.exists() {
        return Ok(Vec::new());
    }

    let mut templates = Vec::new();

    for entry in fs::read_dir(&templates_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let template_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();

            let config_path = path.join(".hannahanna.yml");

            // Only include if it has a config file
            if config_path.exists() {
                let description = read_template_description(&path);

                templates.push(Template {
                    name: template_name,
                    description,
                    config_path,
                });
            }
        }
    }

    Ok(templates)
}

/// Get a specific template by name
pub fn get_template(repo_root: &Path, template_name: &str) -> Result<Template> {
    let templates_dir = repo_root.join(".hn-templates");
    let template_dir = templates_dir.join(template_name);

    if !template_dir.exists() {
        return Err(HnError::ConfigError(format!(
            "Template '{}' not found in .hn-templates/",
            template_name
        )));
    }

    let config_path = template_dir.join(".hannahanna.yml");

    if !config_path.exists() {
        return Err(HnError::ConfigError(format!(
            "Template '{}' is missing .hannahanna.yml",
            template_name
        )));
    }

    let description = read_template_description(&template_dir);

    Ok(Template {
        name: template_name.to_string(),
        description,
        config_path,
    })
}

/// Apply a template to a worktree by merging its config with the repo config
pub fn apply_template(
    repo_root: &Path,
    worktree_path: &Path,
    template_name: &str,
) -> Result<()> {
    let template = get_template(repo_root, template_name)?;

    // Load the template config
    let template_config_str = fs::read_to_string(&template.config_path)?;
    let template_config: Config = serde_yml::from_str(&template_config_str)
        .map_err(|e| HnError::ConfigError(format!("Failed to parse template config: {}", e)))?;

    // Write template-specific config to worktree's local config
    let local_config_path = worktree_path.join(".hannahanna.local.yml");

    // Serialize the template config
    let config_yaml = serde_yml::to_string(&template_config)
        .map_err(|e| HnError::ConfigError(format!("Failed to serialize template config: {}", e)))?;

    fs::write(&local_config_path, config_yaml)?;

    eprintln!("✓ Applied template '{}' to worktree", template_name);

    if let Some(desc) = &template.description {
        eprintln!("  {}", desc);
    }

    Ok(())
}

/// Read template description from README.md or description.txt
fn read_template_description(template_dir: &Path) -> Option<String> {
    // Try README.md first
    let readme_path = template_dir.join("README.md");
    if let Ok(content) = fs::read_to_string(&readme_path) {
        // Extract first line or first paragraph
        let first_line = content.lines().find(|l| !l.trim().is_empty())?;
        return Some(first_line.trim_start_matches('#').trim().to_string());
    }

    // Try description.txt
    let desc_path = template_dir.join("description.txt");
    if let Ok(content) = fs::read_to_string(&desc_path) {
        return Some(content.trim().to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_list_templates_empty() {
        let temp_dir = TempDir::new().unwrap();
        let templates = list_templates(temp_dir.path()).unwrap();
        assert_eq!(templates.len(), 0);
    }

    #[test]
    fn test_get_template() {
        let temp_dir = TempDir::new().unwrap();
        let templates_dir = temp_dir.path().join(".hn-templates").join("test-template");
        fs::create_dir_all(&templates_dir).unwrap();

        // Create a template config
        let config = r#"
docker:
  enabled: true
hooks:
  post_create: |
    echo "Test template setup"
"#;
        fs::write(templates_dir.join(".hannahanna.yml"), config).unwrap();

        // Create description
        fs::write(templates_dir.join("description.txt"), "Test template").unwrap();

        let template = get_template(temp_dir.path(), "test-template").unwrap();
        assert_eq!(template.name, "test-template");
        assert_eq!(template.description, Some("Test template".to_string()));
    }

    #[test]
    fn test_template_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let result = get_template(temp_dir.path(), "nonexistent");
        assert!(result.is_err());
    }
}
