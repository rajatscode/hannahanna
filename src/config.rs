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
    #[serde(default)]
    pub sparse: SparseConfig,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HooksConfig {
    // Worktree lifecycle hooks
    pub pre_create: Option<String>,
    pub post_create: Option<String>,
    pub pre_remove: Option<String>,
    pub post_remove: Option<String>,
    pub post_switch: Option<String>,

    // Integration hooks
    pub pre_integrate: Option<String>,
    pub post_integrate: Option<String>,

    /// Hook execution timeout in seconds (default: 300 = 5 minutes)
    #[serde(default = "default_hook_timeout")]
    pub timeout_seconds: u64,

    /// Conditional hooks that run based on branch name patterns
    #[serde(default)]
    pub pre_create_conditions: Vec<ConditionalHook>,
    #[serde(default)]
    pub post_create_conditions: Vec<ConditionalHook>,
    #[serde(default)]
    pub pre_remove_conditions: Vec<ConditionalHook>,
    #[serde(default)]
    pub post_remove_conditions: Vec<ConditionalHook>,
    #[serde(default)]
    pub post_switch_conditions: Vec<ConditionalHook>,
    #[serde(default)]
    pub pre_integrate_conditions: Vec<ConditionalHook>,
    #[serde(default)]
    pub post_integrate_conditions: Vec<ConditionalHook>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConditionalHook {
    /// Condition to evaluate (e.g., "branch.startsWith('feature/')")
    pub condition: String,
    /// Command to run if condition matches
    pub command: String,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            pre_create: None,
            post_create: None,
            pre_remove: None,
            post_remove: None,
            post_switch: None,
            pre_integrate: None,
            post_integrate: None,
            timeout_seconds: default_hook_timeout(),
            pre_create_conditions: Vec::new(),
            post_create_conditions: Vec::new(),
            pre_remove_conditions: Vec::new(),
            post_remove_conditions: Vec::new(),
            post_switch_conditions: Vec::new(),
            pre_integrate_conditions: Vec::new(),
            post_integrate_conditions: Vec::new(),
        }
    }
}

fn default_hook_timeout() -> u64 {
    300 // 5 minutes
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct SparseConfig {
    /// Enable sparse checkout by default for new worktrees
    #[serde(default)]
    pub enabled: bool,
    /// Default sparse paths (applied when sparse is enabled)
    /// Example: ["services/api/", "libs/utils/"]
    #[serde(default)]
    pub paths: Vec<String>,
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
    /// This is kept for backwards compatibility, but internally uses load_hierarchy
    pub fn load(repo_root: &Path) -> Result<Self> {
        Self::load_hierarchy(repo_root)
    }

    /// Load config from multiple levels and merge them
    /// Priority (highest to lowest):
    /// 1. Local: .hannahanna.local.yml (gitignored, highest priority)
    /// 2. Repo: .hannahanna.yml (committed)
    /// 3. User: ~/.config/hannahanna/config.yml
    /// 4. System: /etc/hannahanna/config.yml
    pub fn load_hierarchy(repo_root: &Path) -> Result<Self> {
        let mut config = Config::default();

        // 4. System config (lowest priority)
        if let Some(system_config) = Self::load_from_path(Path::new("/etc/hannahanna/config.yml"))? {
            config.merge_with(system_config);
        }

        // 3. User config
        if let Some(user_home) = dirs::home_dir() {
            let user_config_path = user_home.join(".config/hannahanna/config.yml");
            if let Some(user_config) = Self::load_from_path(&user_config_path)? {
                config.merge_with(user_config);
            }
        }

        // 2. Repo config (committed)
        let repo_config_path = repo_root.join(".hannahanna.yml");
        if let Some(repo_config) = Self::load_from_path(&repo_config_path)? {
            config.merge_with(repo_config);
        }

        // 1. Local config (gitignored, highest priority)
        let local_config_path = repo_root.join(".hannahanna.local.yml");
        if let Some(local_config) = Self::load_from_path(&local_config_path)? {
            config.merge_with(local_config);
        }

        Ok(config)
    }

    /// Load a single config file from path, returning None if it doesn't exist
    fn load_from_path(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path)?;
        let config: Config = serde_yml::from_str(&content).map_err(|e| {
            crate::errors::HnError::ConfigError(format!(
                "Failed to parse config at {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(Some(config))
    }

    /// Deep merge another config into this one
    /// Arrays are appended (not replaced)
    /// Primitives are overridden
    pub fn merge_with(&mut self, other: Config) {
        // Merge shared_resources (append arrays)
        self.shared_resources.extend(other.shared_resources);

        // Merge shared config
        if let Some(other_shared) = other.shared {
            if let Some(ref mut self_shared) = self.shared {
                // Append copy resources
                self_shared.copy.extend(other_shared.copy);
            } else {
                self.shared = Some(other_shared);
            }
        }

        // Merge hooks (override primitives, append conditional arrays)
        if other.hooks.pre_create.is_some() {
            self.hooks.pre_create = other.hooks.pre_create;
        }
        if other.hooks.post_create.is_some() {
            self.hooks.post_create = other.hooks.post_create;
        }
        if other.hooks.pre_remove.is_some() {
            self.hooks.pre_remove = other.hooks.pre_remove;
        }
        if other.hooks.post_remove.is_some() {
            self.hooks.post_remove = other.hooks.post_remove;
        }
        if other.hooks.post_switch.is_some() {
            self.hooks.post_switch = other.hooks.post_switch;
        }
        if other.hooks.pre_integrate.is_some() {
            self.hooks.pre_integrate = other.hooks.pre_integrate;
        }
        if other.hooks.post_integrate.is_some() {
            self.hooks.post_integrate = other.hooks.post_integrate;
        }
        // Override timeout only if explicitly set (different from default)
        if other.hooks.timeout_seconds != default_hook_timeout() {
            self.hooks.timeout_seconds = other.hooks.timeout_seconds;
        }
        // Append conditional hooks (arrays append)
        self.hooks.pre_create_conditions.extend(other.hooks.pre_create_conditions);
        self.hooks.post_create_conditions.extend(other.hooks.post_create_conditions);
        self.hooks.pre_remove_conditions.extend(other.hooks.pre_remove_conditions);
        self.hooks.post_remove_conditions.extend(other.hooks.post_remove_conditions);
        self.hooks.post_switch_conditions.extend(other.hooks.post_switch_conditions);
        self.hooks.pre_integrate_conditions.extend(other.hooks.pre_integrate_conditions);
        self.hooks.post_integrate_conditions.extend(other.hooks.post_integrate_conditions);

        // Merge docker config (override primitives, append arrays)
        if other.docker.enabled {
            self.docker.enabled = true;
        }
        if other.docker.strategy != default_strategy() {
            self.docker.strategy = other.docker.strategy;
        }
        if other.docker.compose_file != default_compose_file() {
            self.docker.compose_file = other.docker.compose_file;
        }
        if other.docker.auto_start {
            self.docker.auto_start = true;
        }
        if other.docker.auto_stop_others {
            self.docker.auto_stop_others = true;
        }

        // Merge docker ports
        if other.docker.ports.strategy != default_port_strategy() {
            self.docker.ports.strategy = other.docker.ports.strategy;
        }
        for (key, value) in other.docker.ports.base {
            self.docker.ports.base.insert(key, value);
        }
        if other.docker.ports.range.is_some() {
            self.docker.ports.range = other.docker.ports.range;
        }

        // Merge docker shared resources (append arrays)
        self.docker.shared.volumes.extend(other.docker.shared.volumes);
        self.docker.shared.networks.extend(other.docker.shared.networks);

        // Merge docker isolated resources (append arrays)
        self.docker.isolated.volumes.extend(other.docker.isolated.volumes);

        // Merge docker env vars (override)
        for (key, value) in other.docker.env {
            self.docker.env.insert(key, value);
        }

        // Merge healthcheck
        if other.docker.healthcheck.enabled {
            self.docker.healthcheck.enabled = true;
        }
        if other.docker.healthcheck.timeout != default_healthcheck_timeout() {
            self.docker.healthcheck.timeout = other.docker.healthcheck.timeout;
        }

        // Merge sparse config
        if other.sparse.enabled {
            self.sparse.enabled = true;
        }
        self.sparse.paths.extend(other.sparse.paths);
    }

    /// Get list of config file paths that exist and would be loaded
    /// Returns paths in order of priority (highest first)
    pub fn get_loaded_config_paths(repo_root: &Path) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Check in priority order (highest first)

        // 1. Local config (gitignored, highest priority)
        let local_config_path = repo_root.join(".hannahanna.local.yml");
        if local_config_path.exists() {
            paths.push(local_config_path);
        }

        // 2. Repo config (committed)
        let repo_config_path = repo_root.join(".hannahanna.yml");
        if repo_config_path.exists() {
            paths.push(repo_config_path);
        }

        // 3. User config
        if let Some(user_home) = dirs::home_dir() {
            let user_config_path = user_home.join(".config/hannahanna/config.yml");
            if user_config_path.exists() {
                paths.push(user_config_path);
            }
        }

        // 4. System config (lowest priority)
        let system_config_path = Path::new("/etc/hannahanna/config.yml");
        if system_config_path.exists() {
            paths.push(system_config_path.to_path_buf());
        }

        paths
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
        assert_eq!(
            config.docker.env.get("PORT"),
            Some(&"{{port.app}}".to_string())
        );

        // Verify healthcheck
        assert!(config.docker.healthcheck.enabled);
        assert_eq!(config.docker.healthcheck.timeout, "30s");
    }
}
