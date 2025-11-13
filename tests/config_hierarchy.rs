// Config hierarchy tests: Multi-level config merging
use hannahanna::config::Config;
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

/// Create a Git repository in the given directory
fn init_git_repo(dir: &Path) {
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .expect("Failed to init git repo");

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .output()
        .expect("Failed to set git email");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir)
        .output()
        .expect("Failed to set git name");
}

/// Write a config file
fn write_config(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("Failed to create parent directory");
    }
    let mut file = fs::File::create(path).expect("Failed to create config file");
    file.write_all(content.as_bytes())
        .expect("Failed to write config file");
}

#[test]
fn test_load_single_repo_config() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    let config_content = r#"
shared_resources:
  - source: node_modules
    target: node_modules
    compatibility: package-lock.json

hooks:
  post_create: "npm install"
"#;

    write_config(&temp_dir.path().join(".hannahanna.yml"), config_content);

    let config = Config::load(temp_dir.path()).unwrap();

    assert_eq!(config.shared_resources.len(), 1);
    assert_eq!(config.shared_resources[0].source, "node_modules");
    assert_eq!(config.hooks.post_create, Some("npm install".to_string()));
}

#[test]
fn test_local_overrides_repo_config() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Repo config
    let repo_config = r#"
shared_resources:
  - source: node_modules
    target: node_modules

hooks:
  post_create: "npm install"
"#;
    write_config(&temp_dir.path().join(".hannahanna.yml"), repo_config);

    // Local config (overrides)
    let local_config = r#"
hooks:
  post_create: "yarn install"
"#;
    write_config(&temp_dir.path().join(".hannahanna.local.yml"), local_config);

    let config = Config::load(temp_dir.path()).unwrap();

    // Local hook should override repo hook
    assert_eq!(config.hooks.post_create, Some("yarn install".to_string()));

    // Shared resources from repo should still be present
    assert_eq!(config.shared_resources.len(), 1);
}

#[test]
fn test_arrays_append_not_replace() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Repo config with some shared resources
    let repo_config = r#"
shared_resources:
  - source: node_modules
    target: node_modules

sparse:
  enabled: true
  paths:
    - services/api/
"#;
    write_config(&temp_dir.path().join(".hannahanna.yml"), repo_config);

    // Local config with additional shared resources
    let local_config = r#"
shared_resources:
  - source: vendor
    target: vendor

sparse:
  paths:
    - libs/utils/
"#;
    write_config(&temp_dir.path().join(".hannahanna.local.yml"), local_config);

    let config = Config::load(temp_dir.path()).unwrap();

    // Arrays should be merged (appended), not replaced
    assert_eq!(config.shared_resources.len(), 2);
    assert_eq!(config.shared_resources[0].source, "node_modules");
    assert_eq!(config.shared_resources[1].source, "vendor");

    // Sparse paths should also be appended
    assert_eq!(config.sparse.paths.len(), 2);
    assert!(config.sparse.paths.contains(&"services/api/".to_string()));
    assert!(config.sparse.paths.contains(&"libs/utils/".to_string()));
}

#[test]
fn test_primitives_override() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Repo config
    let repo_config = r#"
sparse:
  enabled: false
  paths:
    - services/api/

docker:
  enabled: false
  auto_start: false

hooks:
  timeout_seconds: 300
"#;
    write_config(&temp_dir.path().join(".hannahanna.yml"), repo_config);

    // Local config (overrides booleans)
    let local_config = r#"
sparse:
  enabled: true

docker:
  enabled: true
  auto_start: true

hooks:
  timeout_seconds: 600
"#;
    write_config(&temp_dir.path().join(".hannahanna.local.yml"), local_config);

    let config = Config::load(temp_dir.path()).unwrap();

    // Primitives should be overridden
    assert!(config.sparse.enabled);
    assert!(config.docker.enabled);
    assert!(config.docker.auto_start);
    assert_eq!(config.hooks.timeout_seconds, 600);

    // Arrays should still be preserved
    assert_eq!(config.sparse.paths.len(), 1);
}

#[test]
fn test_empty_config_files() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Empty repo config
    write_config(&temp_dir.path().join(".hannahanna.yml"), "");

    // Empty local config
    write_config(&temp_dir.path().join(".hannahanna.local.yml"), "");

    let config = Config::load(temp_dir.path()).unwrap();

    // Should get default config
    assert!(config.shared_resources.is_empty());
    assert!(config.hooks.post_create.is_none());
}

#[test]
fn test_no_config_files() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // No config files at all
    let config = Config::load(temp_dir.path()).unwrap();

    // Should get default config
    assert!(config.shared_resources.is_empty());
    assert!(config.hooks.post_create.is_none());
    assert!(!config.sparse.enabled);
    assert!(!config.docker.enabled);
}

#[test]
fn test_get_loaded_config_paths() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Create repo and local configs
    write_config(&temp_dir.path().join(".hannahanna.yml"), "{}");
    write_config(&temp_dir.path().join(".hannahanna.local.yml"), "{}");

    let paths = Config::get_loaded_config_paths(temp_dir.path());

    // Should find both files, with local first (highest priority)
    assert_eq!(paths.len(), 2);
    assert!(paths[0].ends_with(".hannahanna.local.yml"));
    assert!(paths[1].ends_with(".hannahanna.yml"));
}

#[test]
fn test_get_loaded_config_paths_empty() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // No config files
    let paths = Config::get_loaded_config_paths(temp_dir.path());

    assert_eq!(paths.len(), 0);
}

#[test]
fn test_docker_env_vars_override() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Repo config with some env vars
    let repo_config = r#"
docker:
  env:
    DATABASE_URL: "postgres://localhost:5432/myapp"
    PORT: "3000"
"#;
    write_config(&temp_dir.path().join(".hannahanna.yml"), repo_config);

    // Local config overrides one env var and adds another
    let local_config = r#"
docker:
  env:
    PORT: "4000"
    REDIS_URL: "redis://localhost:6379"
"#;
    write_config(&temp_dir.path().join(".hannahanna.local.yml"), local_config);

    let config = Config::load(temp_dir.path()).unwrap();

    // PORT should be overridden
    assert_eq!(config.docker.env.get("PORT"), Some(&"4000".to_string()));

    // DATABASE_URL should be preserved from repo config
    assert_eq!(
        config.docker.env.get("DATABASE_URL"),
        Some(&"postgres://localhost:5432/myapp".to_string())
    );

    // REDIS_URL should be added from local config
    assert_eq!(
        config.docker.env.get("REDIS_URL"),
        Some(&"redis://localhost:6379".to_string())
    );
}

#[test]
fn test_docker_ports_merge() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Repo config with some port mappings
    let repo_config = r#"
docker:
  ports:
    base:
      app: 3000
      postgres: 5432
"#;
    write_config(&temp_dir.path().join(".hannahanna.yml"), repo_config);

    // Local config overrides one port and adds another
    let local_config = r#"
docker:
  ports:
    base:
      app: 4000
      redis: 6379
"#;
    write_config(&temp_dir.path().join(".hannahanna.local.yml"), local_config);

    let config = Config::load(temp_dir.path()).unwrap();

    // app port should be overridden
    assert_eq!(config.docker.ports.base.get("app"), Some(&4000));

    // postgres port should be preserved from repo config
    assert_eq!(config.docker.ports.base.get("postgres"), Some(&5432));

    // redis port should be added from local config
    assert_eq!(config.docker.ports.base.get("redis"), Some(&6379));
}

#[test]
fn test_docker_volumes_append() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Repo config with shared volumes
    let repo_config = r#"
docker:
  shared:
    volumes:
      - postgres-data
    networks:
      - myapp-net
  isolated:
    volumes:
      - app-cache
"#;
    write_config(&temp_dir.path().join(".hannahanna.yml"), repo_config);

    // Local config adds more volumes
    let local_config = r#"
docker:
  shared:
    volumes:
      - redis-data
    networks:
      - debug-net
  isolated:
    volumes:
      - logs
"#;
    write_config(&temp_dir.path().join(".hannahanna.local.yml"), local_config);

    let config = Config::load(temp_dir.path()).unwrap();

    // Volumes should be appended
    assert_eq!(config.docker.shared.volumes.len(), 2);
    assert!(config
        .docker
        .shared
        .volumes
        .contains(&"postgres-data".to_string()));
    assert!(config
        .docker
        .shared
        .volumes
        .contains(&"redis-data".to_string()));

    // Networks should be appended
    assert_eq!(config.docker.shared.networks.len(), 2);
    assert!(config
        .docker
        .shared
        .networks
        .contains(&"myapp-net".to_string()));
    assert!(config
        .docker
        .shared
        .networks
        .contains(&"debug-net".to_string()));

    // Isolated volumes should be appended
    assert_eq!(config.docker.isolated.volumes.len(), 2);
    assert!(config
        .docker
        .isolated
        .volumes
        .contains(&"app-cache".to_string()));
    assert!(config.docker.isolated.volumes.contains(&"logs".to_string()));
}

#[test]
fn test_copy_resources_append() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // Repo config with some copy resources
    let repo_config = r#"
shared:
  copy:
    - .env.template -> .env
"#;
    write_config(&temp_dir.path().join(".hannahanna.yml"), repo_config);

    // Local config adds more copy resources
    let local_config = r#"
shared:
  copy:
    - config/local.yml.example -> config/local.yml
"#;
    write_config(&temp_dir.path().join(".hannahanna.local.yml"), local_config);

    let config = Config::load(temp_dir.path()).unwrap();

    // Copy resources should be appended
    assert_eq!(config.shared.as_ref().unwrap().copy.len(), 2);
    assert_eq!(
        config.shared.as_ref().unwrap().copy[0].source,
        ".env.template"
    );
    assert_eq!(
        config.shared.as_ref().unwrap().copy[1].source,
        "config/local.yml.example"
    );
}

#[test]
fn test_priority_order_four_levels() {
    let temp_dir = TempDir::new().unwrap();
    init_git_repo(temp_dir.path());

    // We can't easily test system and user configs in a unit test,
    // but we can test that the priority order is correct for repo and local

    // Repo config
    let repo_config = r#"
hooks:
  post_create: "from_repo"
  timeout_seconds: 100

shared_resources:
  - source: from_repo
    target: repo_target
"#;
    write_config(&temp_dir.path().join(".hannahanna.yml"), repo_config);

    // Local config (should override)
    let local_config = r#"
hooks:
  post_create: "from_local"
  timeout_seconds: 200

shared_resources:
  - source: from_local
    target: local_target
"#;
    write_config(&temp_dir.path().join(".hannahanna.local.yml"), local_config);

    let config = Config::load(temp_dir.path()).unwrap();

    // Local should win for primitives
    assert_eq!(config.hooks.post_create, Some("from_local".to_string()));
    assert_eq!(config.hooks.timeout_seconds, 200);

    // Arrays should be merged (both present)
    assert_eq!(config.shared_resources.len(), 2);
    assert_eq!(config.shared_resources[0].source, "from_repo");
    assert_eq!(config.shared_resources[1].source, "from_local");
}
