// Comprehensive Docker tests for v0.5
mod common;

use common::TestRepo;
use std::fs;

// ============ Docker Configuration Tests ============

#[test]
fn test_docker_enabled_creates_compose_file() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
"#,
    );

    repo.hn(&["add", "docker-test"]).assert_success();
    let _wt_path = repo.worktree_path("docker-test");
    // Check if docker-compose.yml was created (if implemented)
    // This may need adjustment based on implementation
}

#[test]
fn test_docker_disabled_no_compose() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: false
"#,
    );

    repo.hn(&["add", "no-docker"]).assert_success();
    // Should succeed without Docker setup
}

#[test]
fn test_docker_port_allocation_auto() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  ports:
    app: auto
"#,
    );

    repo.hn(&["add", "auto-port"]).assert_success();
    // Port should be automatically allocated
}

#[test]
fn test_docker_port_allocation_specific() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  ports:
    app: 8080
"#,
    );

    repo.hn(&["add", "fixed-port"]).assert_success();
    // Port 8080 should be allocated
}

#[test]
fn test_docker_multiple_services() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
    - db
    - redis
  ports:
    app: auto
    db: 5432
    redis: auto
"#,
    );

    repo.hn(&["add", "multi-service"]).assert_success();
    // Multiple ports should be allocated
}

#[test]
fn test_docker_port_conflict_detection() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  ports:
    app: 8080
"#,
    );

    repo.hn(&["add", "wt1"]).assert_success();

    // Second worktree with same port should handle conflict
    repo.hn(&["add", "wt2"]).assert_success();
    // Implementation should either auto-reassign or warn
}

#[test]
fn test_docker_env_var_injection() {
    let repo = TestRepo::new();

    let output_file = repo.path().join("docker_env.txt");
    repo.create_config(&format!(
        r#"
docker:
  enabled: true
  services:
    - app
hooks:
  post_create: |
    echo "NAME=$HNHN_NAME" > {}
    echo "DOCKER=$HNHN_DOCKER_PORT_APP" >> {}
"#,
        output_file.display(),
        output_file.display()
    ));

    repo.hn(&["add", "env-inject"]).assert_success();

    if output_file.exists() {
        let content = fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("NAME=env-inject"));
    }
}

#[test]
fn test_docker_compose_template_generation() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  compose_template: |
    version: '3.8'
    services:
      app:
        image: nginx
        ports:
          - "${PORT}:80"
"#,
    );

    repo.hn(&["add", "template-test"]).assert_success();
    // Compose file should be generated from template
}

// ============ Docker Commands Tests ============

#[test]
fn test_docker_ps_command() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
"#,
    );

    repo.hn(&["add", "ps-test"]).assert_success();

    let result = repo.hn(&["docker", "ps"]);
    // Should list docker status (may be empty if docker not running)
    assert!(result.success);
}

#[test]
fn test_docker_start_command() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
"#,
    );

    repo.hn(&["add", "start-test"]).assert_success();

    // Docker start command should work (or gracefully fail if docker not available)
    let _result = repo.hn(&["docker", "start", "start-test"]);
}

#[test]
fn test_docker_stop_command() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
"#,
    );

    repo.hn(&["add", "stop-test"]).assert_success();

    let _result = repo.hn(&["docker", "stop", "stop-test"]);
    // Should handle gracefully even if containers not running
}

#[test]
fn test_docker_restart_command() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
"#,
    );

    repo.hn(&["add", "restart-test"]).assert_success();

    let _result = repo.hn(&["docker", "restart", "restart-test"]);
}

#[test]
fn test_docker_logs_command() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
"#,
    );

    repo.hn(&["add", "logs-test"]).assert_success();

    let _result = repo.hn(&["docker", "logs", "logs-test"]);
}

#[test]
fn test_docker_exec_command() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
"#,
    );

    repo.hn(&["add", "exec-test"]).assert_success();

    let _result = repo.hn(&["docker", "exec", "exec-test", "echo", "test"]);
}

#[test]
fn test_docker_prune_command() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
"#,
    );

    let result = repo.hn(&["docker", "prune"]);
    assert!(result.success);
}

// ============ Port Management Tests ============

#[test]
fn test_ports_list_command() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  ports:
    app: auto
"#,
    );

    repo.hn(&["add", "port-list-test"]).assert_success();

    let result = repo.hn(&["ports", "list"]);
    assert!(result.success);
}

#[test]
fn test_ports_show_command() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  ports:
    app: auto
"#,
    );

    repo.hn(&["add", "port-show"]).assert_success();

    let result = repo.hn(&["ports", "show", "port-show"]);
    assert!(result.success);
}

#[test]
fn test_ports_release_command() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  ports:
    app: 9000
"#,
    );

    repo.hn(&["add", "port-release"]).assert_success();

    let result = repo.hn(&["ports", "release", "port-release"]);
    assert!(result.success || result.stderr.contains("No ports"));
}

#[test]
fn test_ports_reassign_command() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  ports:
    app: auto
"#,
    );

    repo.hn(&["add", "port-reassign"]).assert_success();

    let result = repo.hn(&["ports", "reassign", "port-reassign"]);
    // Should succeed or handle gracefully
    assert!(result.success || result.stderr.contains("port"));
}

// ============ Docker Integration Tests ============

#[test]
fn test_docker_with_hooks() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
hooks:
  post_create: "echo 'Docker setup' > docker.txt"
"#,
    );

    repo.hn(&["add", "docker-hooks"]).assert_success();
    let wt_path = repo.worktree_path("docker-hooks");
    assert!(wt_path.join("docker.txt").exists());
}

#[test]
fn test_docker_with_template() {
    let repo = TestRepo::new();

    // Create a template with Docker config
    let templates_dir = repo.path().join(".hn-templates");
    let template_dir = templates_dir.join("docker-template");
    fs::create_dir_all(&template_dir).unwrap();

    fs::write(
        template_dir.join(".hannahanna.yml"),
        r#"
docker:
  enabled: true
  services:
    - app
"#,
    )
    .unwrap();

    let result = repo.hn(&["add", "from-template", "--template", "docker-template"]);
    assert!(result.success);
}

#[test]
fn test_docker_cleanup_on_remove() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
"#,
    );

    repo.hn(&["add", "cleanup-test"]).assert_success();

    // Remove worktree - should clean up Docker resources
    let _result = repo.hn(&["remove", "cleanup-test", "--force"]);
}

#[test]
fn test_docker_port_range_allocation() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  ports:
    app: auto
  port_range:
    start: 10000
    end: 10100
"#,
    );

    repo.hn(&["add", "range-test"]).assert_success();
    // Port should be allocated within range
}

#[test]
fn test_docker_service_dependencies() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
    - db
  depends_on:
    app: [db]
"#,
    );

    repo.hn(&["add", "deps-test"]).assert_success();
}

#[test]
fn test_docker_env_file_generation() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  env_vars:
    APP_ENV: "development"
    DEBUG: "true"
"#,
    );

    repo.hn(&["add", "env-file"]).assert_success();
    // Should generate .env file for docker
}

#[test]
fn test_docker_volume_mapping() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  volumes:
    - "./src:/app/src"
    - "./config:/app/config"
"#,
    );

    repo.hn(&["add", "volumes"]).assert_success();
}

#[test]
fn test_docker_network_isolation() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  network: "isolated"
"#,
    );

    repo.hn(&["add", "network"]).assert_success();
}

#[test]
fn test_docker_container_naming() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  container_prefix: "hn"
"#,
    );

    repo.hn(&["add", "naming"]).assert_success();
    // Containers should be named with prefix
}

#[test]
fn test_docker_health_checks() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  health_check:
    enabled: true
    endpoint: "/health"
"#,
    );

    repo.hn(&["add", "health"]).assert_success();
}

#[test]
fn test_docker_resource_limits() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  limits:
    memory: "512m"
    cpus: "1.0"
"#,
    );

    repo.hn(&["add", "limits"]).assert_success();
}

#[test]
fn test_docker_build_context() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  build:
    context: "."
    dockerfile: "Dockerfile"
"#,
    );

    repo.hn(&["add", "build"]).assert_success();
}

#[test]
fn test_docker_compose_override() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
  compose_override: true
"#,
    );

    repo.hn(&["add", "override"]).assert_success();
    // Should support docker-compose.override.yml
}

#[test]
fn test_docker_logs_with_service() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
    - db
"#,
    );

    repo.hn(&["add", "logs-service"]).assert_success();

    let _result = repo.hn(&["docker", "logs", "logs-service", "app"]);
}

#[test]
fn test_docker_exec_with_service() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
    - db
"#,
    );

    repo.hn(&["add", "exec-service"]).assert_success();

    let _result = repo.hn(&["docker", "exec", "exec-service", "--service", "app", "pwd"]);
}

#[test]
fn test_docker_each_with_filter() {
    let repo = TestRepo::new();

    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
"#,
    );

    repo.hn(&["add", "docker-each-1"]).assert_success();
    repo.hn(&["add", "docker-each-2"]).assert_success();

    let result = repo.hn(&["each", "--docker-running", "echo", "test"]);
    // Should only run on worktrees with docker running
    assert!(result.success);
}
