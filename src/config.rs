use crate::errors::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    #[serde(default)]
    pub docker: DockerConfig,
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

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct HooksConfig {
    pub post_create: Option<String>,
    pub pre_remove: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DockerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_strategy")]
    pub strategy: String,
    #[serde(default = "default_compose_file")]
    pub compose_file: String,
    #[serde(default)]
    pub ports: PortsConfig,
    #[serde(default)]
    pub shared: DockerSharedConfig,
    #[serde(default)]
    pub isolated: DockerIsolatedConfig,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub auto_stop_others: bool,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub healthcheck: HealthCheckConfig,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            strategy: default_strategy(),
            compose_file: default_compose_file(),
            ports: PortsConfig::default(),
            shared: DockerSharedConfig::default(),
            isolated: DockerIsolatedConfig::default(),
            auto_start: false,
            auto_stop_others: false,
            env: HashMap::new(),
            healthcheck: HealthCheckConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PortsConfig {
    #[serde(default = "default_port_strategy")]
    pub strategy: String,
    #[serde(default)]
    pub base: HashMap<String, u16>,
    #[serde(default)]
    pub range: Option<[u16; 2]>,
}

impl Default for PortsConfig {
    fn default() -> Self {
        let mut base = HashMap::new();
        base.insert("app".to_string(), 3000);
        base.insert("postgres".to_string(), 5432);
        base.insert("redis".to_string(), 6379);

        Self {
            strategy: default_port_strategy(),
            base,
            range: Some([3000, 9999]),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct DockerSharedConfig {
    #[serde(default)]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub networks: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct DockerIsolatedConfig {
    #[serde(default)]
    pub volumes: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HealthCheckConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_healthcheck_timeout")]
    pub timeout: String,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeout: default_healthcheck_timeout(),
        }
    }
}

fn default_strategy() -> String {
    "per-worktree".to_string()
}

fn default_compose_file() -> String {
    "docker-compose.yml".to_string()
}

fn default_port_strategy() -> String {
    "auto-offset".to_string()
}

fn default_healthcheck_timeout() -> String {
    "30s".to_string()
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

    #[test]
    fn test_docker_config_defaults() {
        let config = Config::default();
        assert!(!config.docker.enabled);
        assert_eq!(config.docker.strategy, "per-worktree");
        assert_eq!(config.docker.compose_file, "docker-compose.yml");
        assert!(!config.docker.auto_start);
        assert!(!config.docker.auto_stop_others);
        assert_eq!(config.docker.ports.strategy, "auto-offset");
        assert_eq!(config.docker.ports.base.get("app"), Some(&3000));
        assert_eq!(config.docker.ports.base.get("postgres"), Some(&5432));
    }

    #[test]
    fn test_parse_docker_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".hannahanna.yml");

        let yaml = r#"
docker:
  enabled: true
  strategy: "per-worktree"
  compose_file: "docker-compose.yml"

  ports:
    strategy: "auto-offset"
    base:
      app: 3000
      postgres: 5432
      redis: 6379
    range: [3000, 4000]

  shared:
    volumes: [postgres-data]
    networks: [app-net]

  isolated:
    volumes: [app-cache, logs]

  auto_start: true
  auto_stop_others: false

  env:
    DATABASE_URL: "postgres://localhost:{{port.postgres}}/myapp_{{worktree_name}}"
    PORT: "{{port.app}}"

  healthcheck:
    enabled: true
    timeout: "30s"
"#;

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(yaml.as_bytes()).unwrap();

        let config = Config::load(temp_dir.path()).unwrap();

        // Verify Docker config is parsed correctly
        assert!(config.docker.enabled);
        assert_eq!(config.docker.strategy, "per-worktree");
        assert_eq!(config.docker.compose_file, "docker-compose.yml");
        assert!(config.docker.auto_start);
        assert!(!config.docker.auto_stop_others);

        // Verify ports config
        assert_eq!(config.docker.ports.strategy, "auto-offset");
        assert_eq!(config.docker.ports.base.get("app"), Some(&3000));
        assert_eq!(config.docker.ports.base.get("postgres"), Some(&5432));
        assert_eq!(config.docker.ports.base.get("redis"), Some(&6379));
        assert_eq!(config.docker.ports.range, Some([3000, 4000]));

        // Verify shared resources
        assert_eq!(config.docker.shared.volumes, vec!["postgres-data"]);
        assert_eq!(config.docker.shared.networks, vec!["app-net"]);

        // Verify isolated resources
        assert_eq!(config.docker.isolated.volumes, vec!["app-cache", "logs"]);

        // Verify env vars
        assert_eq!(
            config.docker.env.get("DATABASE_URL"),
            Some(&"postgres://localhost:{{port.postgres}}/myapp_{{worktree_name}}".to_string())
        );
        assert_eq!(config.docker.env.get("PORT"), Some(&"{{port.app}}".to_string()));

        // Verify healthcheck
        assert!(config.docker.healthcheck.enabled);
        assert_eq!(config.docker.healthcheck.timeout, "30s");
    }
}
