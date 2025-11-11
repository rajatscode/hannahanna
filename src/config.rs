use crate::errors::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub shared_resources: Vec<SharedResource>,
    #[serde(default)]
    pub shared: Option<SharedConfig>,
    #[serde(default)]
    pub hooks: HooksConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SharedResource {
    pub source: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct SharedConfig {
    #[serde(default, deserialize_with = "deserialize_copy_list")]
    pub copy: Vec<CopyResource>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CopyResource {
    pub source: String,
    pub target: String,
}

/// Custom deserializer for copy list that handles "source -> target" format
fn deserialize_copy_list<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<CopyResource>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let items: Vec<String> = Vec::deserialize(deserializer)?;
    let mut resources = Vec::new();

    for item in items {
        // Parse "source -> target" format
        let parts: Vec<&str> = item.split("->").map(|s| s.trim()).collect();
        if parts.len() != 2 {
            return Err(D::Error::custom(format!(
                "Invalid copy format '{}'. Expected 'source -> target'",
                item
            )));
        }

        resources.push(CopyResource {
            source: parts[0].to_string(),
            target: parts[1].to_string(),
        });
    }

    Ok(resources)
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct HooksConfig {
    pub post_create: Option<String>,
    pub pre_remove: Option<String>,
}

impl Config {
    /// Load config from .hannahanna.yml in repository root
    pub fn load(repo_root: &Path) -> Result<Self> {
        let config_path = repo_root.join(".hannahanna.yml");

        if !config_path.exists() {
            // No config file, return defaults
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&config_path)?;
        let config: Config = serde_yml::from_str(&content).map_err(|e| {
            crate::errors::HnError::ConfigError(format!("Failed to parse config: {}", e))
        })?;

        Ok(config)
    }

    /// Get the repository root by finding the .git directory
    pub fn find_repo_root(start_path: &Path) -> Result<PathBuf> {
        let mut current = start_path;

        loop {
            let git_path = current.join(".git");
            if git_path.exists() {
                return Ok(current.to_path_buf());
            }

            match current.parent() {
                Some(parent) => current = parent,
                None => return Err(crate::errors::HnError::NotInRepository),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.shared_resources.is_empty());
        assert!(config.hooks.post_create.is_none());
        assert!(config.hooks.pre_remove.is_none());
    }

    #[test]
    fn test_load_missing_config() {
        let temp_dir = TempDir::new().unwrap();
        let config = Config::load(temp_dir.path()).unwrap();
        assert!(config.shared_resources.is_empty());
    }

    #[test]
    fn test_load_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".hannahanna.yml");

        let yaml = r#"
shared_resources:
  - source: node_modules
    target: node_modules
    compatibility: package-lock.json
  - source: vendor
    target: vendor

hooks:
  post_create: "npm install"
  pre_remove: "echo 'Cleaning up...'"
"#;

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::load(temp_dir.path()).unwrap();
        assert_eq!(config.shared_resources.len(), 2);
        assert_eq!(config.shared_resources[0].source, "node_modules");
        assert_eq!(
            config.shared_resources[0].compatibility,
            Some("package-lock.json".to_string())
        );
        assert_eq!(config.hooks.post_create, Some("npm install".to_string()));
    }
}
