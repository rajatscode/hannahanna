// Template system for pre-configured worktree environments
//
// Templates allow users to define pre-configured setups for different
// types of worktrees (e.g., "microservice", "frontend", "experiment")

use crate::config::Config;
use crate::errors::{HnError, Result};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Template metadata and configuration
#[derive(Debug, Clone, serde::Serialize)]
pub struct Template {
    pub name: String,
    pub description: Option<String>,
    pub config_path: PathBuf,
}

/// Template parameter types (v0.6)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ParameterType {
    String {
        default: Option<String>,
        #[serde(default)]
        required: bool,
    },
    Integer {
        default: Option<i64>,
        #[serde(default)]
        required: bool,
        min: Option<i64>,
        max: Option<i64>,
    },
    Boolean {
        default: bool,
    },
    Choice {
        choices: Vec<String>,
        default: Option<String>,
    },
}

/// Template parameter definition (v0.6)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParameter {
    pub name: String,
    pub description: Option<String>,
    #[serde(flatten)]
    pub param_type: ParameterType,
}

/// Template configuration with parameters (v0.6)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: Vec<TemplateParameter>,
}

/// Copy template files to worktree with variable substitution (v0.5)
#[allow(dead_code)]
pub fn copy_template_files(
    template_name: &str,
    repo_root: &Path,
    worktree_path: &Path,
    worktree_name: &str,
) -> Result<()> {
    let template_dir = repo_root.join(".hn-templates").join(template_name);
    let files_dir = template_dir.join("files");

    // If files directory doesn't exist, that's fine - just return
    if !files_dir.exists() {
        return Ok(());
    }

    // Copy all files from template files/ directory
    copy_dir_recursive(&files_dir, worktree_path, worktree_name, worktree_path)?;

    Ok(())
}

/// Recursively copy directory with variable substitution
#[allow(dead_code)]
fn copy_dir_recursive(
    src: &Path,
    dst: &Path,
    worktree_name: &str,
    worktree_path: &Path,
) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if src_path.is_dir() {
            // Create directory and recurse
            fs::create_dir_all(&dst_path)?;
            copy_dir_recursive(&src_path, &dst_path, worktree_name, worktree_path)?;
        } else {
            // Copy file with variable substitution
            copy_file_with_substitution(&src_path, &dst_path, worktree_name, worktree_path)?;
        }
    }

    Ok(())
}

/// Copy a single file with variable substitution
#[allow(dead_code)]
fn copy_file_with_substitution(
    src: &Path,
    dst: &Path,
    worktree_name: &str,
    worktree_path: &Path,
) -> Result<()> {
    // Read source file
    let content = fs::read_to_string(src).unwrap_or_else(|_| {
        // If not UTF-8, just copy bytes
        let bytes = fs::read(src).unwrap();
        String::from_utf8_lossy(&bytes).to_string()
    });

    // Perform variable substitution
    let substituted = content
        .replace("${HNHN_NAME}", worktree_name)
        .replace("${HNHN_PATH}", &worktree_path.to_string_lossy())
        .replace("${HNHN_BRANCH}", worktree_name); // Branch typically matches name

    // Write to destination
    fs::write(dst, substituted)?;

    // Preserve permissions (Unix only)
    #[cfg(unix)]
    {
        let metadata = fs::metadata(src)?;
        let permissions = metadata.permissions();
        fs::set_permissions(dst, permissions)?;
    }

    Ok(())
}

/// Find available templates in the repository
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
pub fn apply_template(repo_root: &Path, worktree_path: &Path, template_name: &str) -> Result<()> {
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

/// Template package manifest (v0.6)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateManifest {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub created: String,
    pub hannahanna_version: String,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
}

impl TemplateManifest {
    pub fn new(name: String, description: Option<String>) -> Self {
        Self {
            name,
            version: "1.0.0".to_string(),
            author: std::env::var("USER")
                .ok()
                .or_else(|| std::env::var("USERNAME").ok()),
            description,
            created: chrono::Utc::now().to_rfc3339(),
            hannahanna_version: format!(">={}", env!("CARGO_PKG_VERSION")),
            tags: Vec::new(),
            dependencies: Vec::new(),
        }
    }
}

/// Export a template to a .hnhn package (v0.6)
pub fn export_template(repo_root: &Path, template_name: &str, output_path: &Path) -> Result<()> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use tar::Builder;

    // Get template
    let template = get_template(repo_root, template_name)?;
    let template_dir = repo_root.join(".hn-templates").join(template_name);

    // Create manifest
    let manifest = TemplateManifest::new(template_name.to_string(), template.description.clone());
    let manifest_yaml = serde_yml::to_string(&manifest)
        .map_err(|e| HnError::ConfigError(format!("Failed to serialize manifest: {}", e)))?;

    // Create temporary directory for package assembly
    let temp_dir = tempfile::tempdir()?;
    let package_dir = temp_dir.path().join("package");
    fs::create_dir_all(&package_dir)?;

    // Write manifest
    fs::write(package_dir.join("manifest.yml"), manifest_yaml)?;

    // Copy config
    let config_content = fs::read_to_string(&template.config_path)?;
    fs::write(package_dir.join("config.yml"), config_content)?;

    // Copy files directory if it exists
    let files_dir = template_dir.join("files");
    if files_dir.exists() {
        let package_files_dir = package_dir.join("files");
        copy_dir_all(&files_dir, &package_files_dir)?;
    }

    // Copy README if it exists
    let readme_path = template_dir.join("README.md");
    if readme_path.exists() {
        fs::copy(&readme_path, package_dir.join("README.md"))?;
    }

    // Create tar.gz archive
    let tar_gz = fs::File::create(output_path)?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = Builder::new(enc);
    tar.append_dir_all(".", &package_dir)?;
    tar.finish()?;

    Ok(())
}

/// Import a template from a .hnhn package (v0.6)
pub fn import_template(
    repo_root: &Path,
    package_path: &Path,
    template_name: Option<&str>,
) -> Result<String> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    // Extract to temporary directory first
    let temp_dir = tempfile::tempdir()?;
    let extract_dir = temp_dir.path().join("extract");
    fs::create_dir_all(&extract_dir)?;

    // Open and extract tar.gz
    let tar_gz = fs::File::open(package_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(&extract_dir)?;

    // Read manifest
    let manifest_path = extract_dir.join("manifest.yml");
    if !manifest_path.exists() {
        return Err(HnError::ConfigError(
            "Invalid package: missing manifest.yml".to_string(),
        ));
    }

    let manifest_content = fs::read_to_string(&manifest_path)?;
    let manifest: TemplateManifest = serde_yml::from_str(&manifest_content)
        .map_err(|e| HnError::ConfigError(format!("Invalid manifest: {}", e)))?;

    // Validate version compatibility
    validate_version_compatibility(&manifest.hannahanna_version)?;

    // Determine template name (use provided name or manifest name)
    let final_name = template_name.unwrap_or(&manifest.name).to_string();

    // Check if template already exists
    let templates_dir = repo_root.join(".hn-templates");
    let dest_dir = templates_dir.join(&final_name);
    if dest_dir.exists() {
        return Err(HnError::ConfigError(format!(
            "Template '{}' already exists. Remove it first or use a different name.",
            final_name
        )));
    }

    // Create templates directory if it doesn't exist
    fs::create_dir_all(&templates_dir)?;

    // Copy template to destination
    fs::create_dir_all(&dest_dir)?;

    // Copy config
    let config_src = extract_dir.join("config.yml");
    if config_src.exists() {
        fs::copy(&config_src, dest_dir.join(".hannahanna.yml"))?;
    } else {
        return Err(HnError::ConfigError(
            "Invalid package: missing config.yml".to_string(),
        ));
    }

    // Copy files directory if it exists
    let files_src = extract_dir.join("files");
    if files_src.exists() {
        let files_dest = dest_dir.join("files");
        copy_dir_all(&files_src, &files_dest)?;
    }

    // Copy README if it exists
    let readme_src = extract_dir.join("README.md");
    if readme_src.exists() {
        fs::copy(&readme_src, dest_dir.join("README.md"))?;
    }

    Ok(final_name)
}

/// Validate a template configuration (v0.6)
pub fn validate_template(repo_root: &Path, template_name: &str) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    let template = get_template(repo_root, template_name)?;
    let template_dir = repo_root.join(".hn-templates").join(template_name);

    // Validate config syntax
    let config_content = fs::read_to_string(&template.config_path)?;
    match serde_yml::from_str::<Config>(&config_content) {
        Ok(_) => {}
        Err(e) => {
            return Err(HnError::ConfigError(format!(
                "Invalid configuration syntax: {}",
                e
            )));
        }
    }

    // Check for README
    let readme_path = template_dir.join("README.md");
    if !readme_path.exists() {
        warnings.push("Missing README.md - recommended for documentation".to_string());
    }

    // Check files directory
    let files_dir = template_dir.join("files");
    if files_dir.exists() {
        // Validate file paths don't escape template directory
        validate_template_files(&files_dir, &template_dir)?;
    }

    // Validate hooks if present
    let config: Config = serde_yml::from_str(&config_content)
        .map_err(|e| HnError::ConfigError(format!("Failed to parse config: {}", e)))?;

    if let Some(ref hook) = config.hooks.post_create {
        if hook.trim().is_empty() {
            warnings.push("post_create hook is empty".to_string());
        }
    }

    if let Some(ref hook) = config.hooks.pre_create {
        if hook.trim().is_empty() {
            warnings.push("pre_create hook is empty".to_string());
        }
    }

    Ok(warnings)
}

/// Validate that template files don't escape the template directory
fn validate_template_files(files_dir: &Path, _template_dir: &Path) -> Result<()> {
    for entry in fs::read_dir(files_dir)? {
        let entry = entry?;
        let path = entry.path();

        // Check for symlinks or paths that escape
        if path.is_symlink() {
            return Err(HnError::ConfigError(format!(
                "Template contains symlink: {} (not allowed for security)",
                path.display()
            )));
        }

        // Recursively validate subdirectories
        if path.is_dir() {
            validate_template_files(&path, _template_dir)?;
        }
    }

    Ok(())
}

/// Validate version compatibility
fn validate_version_compatibility(required_version: &str) -> Result<()> {
    // Simple validation: check if current version meets minimum requirement
    // Format: ">=0.5.0"
    let current_version = env!("CARGO_PKG_VERSION");

    if required_version.starts_with(">=") {
        let min_version = required_version.trim_start_matches(">=");
        if version_compare::compare(current_version, min_version)
            .map(|ord| ord == version_compare::Cmp::Lt)
            .unwrap_or(false)
        {
            return Err(HnError::ConfigError(format!(
                "Template requires hannahanna {} but current version is {}",
                required_version, current_version
            )));
        }
    }

    Ok(())
}

/// Recursively copy a directory
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Parse CLI parameters in the format key=value
pub fn parse_cli_parameters(params: &[String]) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();

    for param in params {
        let parts: Vec<&str> = param.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(HnError::ConfigError(format!(
                "Invalid parameter format: '{}'. Expected 'key=value'",
                param
            )));
        }

        map.insert(parts[0].to_string(), parts[1].to_string());
    }

    Ok(map)
}

/// Collect parameter values interactively or from CLI
pub fn collect_template_parameters(
    template_config: &TemplateConfig,
    cli_params: &HashMap<String, String>,
) -> Result<HashMap<String, String>> {
    let theme = ColorfulTheme::default();
    let mut values = HashMap::new();

    for param in &template_config.parameters {
        // Check if provided via CLI
        if let Some(value) = cli_params.get(&param.name) {
            values.insert(param.name.clone(), value.clone());
            continue;
        }

        // Prompt based on type
        let prompt_text = if let Some(ref desc) = param.description {
            format!("{} ({})", param.name, desc)
        } else {
            param.name.clone()
        };

        match &param.param_type {
            ParameterType::String { default, required } => {
                let mut input = Input::<String>::with_theme(&theme).with_prompt(&prompt_text);

                if let Some(def) = default {
                    input = input.default(def.clone());
                }

                if !required {
                    input = input.allow_empty(true);
                }

                let value = input.interact_text()?;
                if !value.is_empty() {
                    values.insert(param.name.clone(), value);
                }
            }
            ParameterType::Integer {
                default,
                min,
                max,
                required: _,
            } => {
                let mut input = Input::<i64>::with_theme(&theme).with_prompt(&prompt_text);

                if let Some(def) = default {
                    input = input.default(*def);
                }

                let value = input
                    .validate_with(|val: &i64| -> std::result::Result<(), String> {
                        if let Some(min_val) = min {
                            if val < min_val {
                                return Err(format!("Value must be at least {}", min_val));
                            }
                        }
                        if let Some(max_val) = max {
                            if val > max_val {
                                return Err(format!("Value must be at most {}", max_val));
                            }
                        }
                        Ok(())
                    })
                    .interact_text()?;

                values.insert(param.name.clone(), value.to_string());
            }
            ParameterType::Boolean { default } => {
                let value = Confirm::with_theme(&theme)
                    .with_prompt(&prompt_text)
                    .default(*default)
                    .interact()?;

                values.insert(param.name.clone(), value.to_string());
            }
            ParameterType::Choice { choices, default } => {
                let mut select = Select::with_theme(&theme)
                    .with_prompt(&prompt_text)
                    .items(choices);

                if let Some(def) = default {
                    if let Some(idx) = choices.iter().position(|c| c == def) {
                        select = select.default(idx);
                    }
                }

                let idx = select.interact()?;
                values.insert(param.name.clone(), choices[idx].clone());
            }
        }
    }

    Ok(values)
}

/// Load template configuration with parameters
pub fn load_template_config(repo_root: &Path, template_name: &str) -> Result<TemplateConfig> {
    let template_dir = repo_root.join(".hn-templates").join(template_name);
    let config_path = template_dir.join("template.yml");

    if !config_path.exists() {
        // No template.yml, return basic config with no parameters
        return Ok(TemplateConfig {
            name: template_name.to_string(),
            description: None,
            parameters: Vec::new(),
        });
    }

    let content = fs::read_to_string(&config_path)?;
    let config: TemplateConfig = serde_yml::from_str(&content)
        .map_err(|e| HnError::TemplateError(format!("Invalid template.yml: {}", e)))?;

    Ok(config)
}

/// Apply template with parameters (v0.6)
pub fn apply_template_with_parameters(
    repo_root: &Path,
    worktree_path: &Path,
    template_name: &str,
    worktree_name: &str,
    cli_params: &[String],
) -> Result<()> {
    // Load template configuration
    let template_config = load_template_config(repo_root, template_name)?;

    // Parse CLI parameters
    let cli_param_map = parse_cli_parameters(cli_params)?;

    // Collect parameter values (interactive + CLI)
    let param_values = collect_template_parameters(&template_config, &cli_param_map)?;

    // Copy template files with parameter substitution
    let template_dir = repo_root.join(".hn-templates").join(template_name);
    let files_dir = template_dir.join("files");

    if !files_dir.exists() {
        return Ok(());
    }

    // Copy with both built-in and custom parameters
    copy_dir_with_params(
        &files_dir,
        worktree_path,
        worktree_name,
        worktree_path,
        &param_values,
    )?;

    Ok(())
}

/// Recursively copy directory with parameter substitution
fn copy_dir_with_params(
    src: &Path,
    dst: &Path,
    worktree_name: &str,
    worktree_path: &Path,
    params: &HashMap<String, String>,
) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_with_params(&src_path, &dst_path, worktree_name, worktree_path, params)?;
        } else {
            copy_file_with_params(&src_path, &dst_path, worktree_name, worktree_path, params)?;
        }
    }

    Ok(())
}

/// Copy file with parameter substitution
fn copy_file_with_params(
    src: &Path,
    dst: &Path,
    worktree_name: &str,
    worktree_path: &Path,
    params: &HashMap<String, String>,
) -> Result<()> {
    let content = fs::read_to_string(src).unwrap_or_else(|_| {
        let bytes = fs::read(src).unwrap();
        String::from_utf8_lossy(&bytes).to_string()
    });

    // Start with built-in variable substitution
    let mut substituted = content
        .replace("${HNHN_NAME}", worktree_name)
        .replace("${HNHN_PATH}", &worktree_path.to_string_lossy())
        .replace("${HNHN_BRANCH}", worktree_name);

    // Apply custom parameter substitution
    for (key, value) in params {
        let placeholder = format!("${{PARAM_{}}}", key.to_uppercase());
        substituted = substituted.replace(&placeholder, value);

        // Also support lowercase variant
        let placeholder_lower = format!("${{{}}}", key);
        substituted = substituted.replace(&placeholder_lower, value);
    }

    fs::write(dst, substituted)?;

    // Preserve permissions (Unix only)
    #[cfg(unix)]
    {
        let metadata = fs::metadata(src)?;
        let permissions = metadata.permissions();
        fs::set_permissions(dst, permissions)?;
    }

    Ok(())
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
