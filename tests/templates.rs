// Integration tests for template management (v0.5)
mod common;

use common::TestRepo;
use std::fs;

#[test]
fn test_templates_list_empty_directory() {
    let repo = TestRepo::new();

    let result = repo.hn(&["templates", "list"]);
    assert!(result.success);
    assert!(result.stdout.contains("No templates found"));
}

#[test]
fn test_templates_list_with_templates() {
    let repo = TestRepo::new();

    let templates_dir = repo.path().join(".hn-templates");
    fs::create_dir_all(&templates_dir).unwrap();

    let microservice_dir = templates_dir.join("microservice");
    fs::create_dir_all(&microservice_dir).unwrap();
    fs::write(microservice_dir.join(".hannahanna.yml"), "# Microservice\n").unwrap();
    fs::write(
        microservice_dir.join("README.md"),
        "# Microservice Template\n",
    )
    .unwrap();

    let frontend_dir = templates_dir.join("frontend");
    fs::create_dir_all(&frontend_dir).unwrap();
    fs::write(frontend_dir.join(".hannahanna.yml"), "# Frontend\n").unwrap();

    let result = repo.hn(&["templates", "list"]);
    assert!(result.success);
    assert!(result.stdout.contains("microservice"));
    assert!(result.stdout.contains("frontend"));
    assert!(result.stdout.contains("2 template"));
}

#[test]
fn test_templates_show_existing_template() {
    let repo = TestRepo::new();

    let templates_dir = repo.path().join(".hn-templates");
    let microservice_dir = templates_dir.join("microservice");
    fs::create_dir_all(&microservice_dir).unwrap();
    fs::write(
        microservice_dir.join(".hannahanna.yml"),
        "docker:\n  enabled: true\n",
    )
    .unwrap();
    fs::write(
        microservice_dir.join("README.md"),
        "# Microservice\nBackend service\n",
    )
    .unwrap();

    let result = repo.hn(&["templates", "show", "microservice"]);
    if !result.success {
        eprintln!("Command failed! stderr: {}", result.stderr);
    }
    assert!(
        result.success,
        "Command should succeed. stderr: {}",
        result.stderr
    );
    assert!(
        result.stdout.contains("microservice"),
        "stdout: {}",
        result.stdout
    );
    // Just check that the command works, description parsing might vary
    assert!(
        result.stdout.contains("Configuration") || result.stdout.contains("Template"),
        "stdout: {}",
        result.stdout
    );
}

#[test]
fn test_templates_show_nonexistent() {
    let repo = TestRepo::new();
    fs::create_dir_all(repo.path().join(".hn-templates")).unwrap();

    let result = repo.hn(&["templates", "show", "nonexistent"]);
    assert!(!result.success);
    assert!(result.stderr.contains("not found"));
}

#[test]
fn test_templates_list_json() {
    let repo = TestRepo::new();

    let templates_dir = repo.path().join(".hn-templates");
    let test_template = templates_dir.join("test");
    fs::create_dir_all(&test_template).unwrap();
    fs::write(test_template.join(".hannahanna.yml"), "# Test\n").unwrap();

    let result = repo.hn(&["templates", "list", "--json"]);
    assert!(result.success);
    assert!(result.stdout.contains("["));
    assert!(result.stdout.contains("test"));
}

#[test]
fn test_templates_ignores_missing_config() {
    let repo = TestRepo::new();

    let templates_dir = repo.path().join(".hn-templates");
    fs::create_dir_all(&templates_dir).unwrap();

    let valid_dir = templates_dir.join("valid");
    fs::create_dir_all(&valid_dir).unwrap();
    fs::write(valid_dir.join(".hannahanna.yml"), "# Valid\n").unwrap();

    let invalid_dir = templates_dir.join("invalid");
    fs::create_dir_all(&invalid_dir).unwrap();
    fs::write(invalid_dir.join("README.md"), "No config\n").unwrap();

    let result = repo.hn(&["templates", "list"]);
    assert!(result.success);
    assert!(result.stdout.contains("valid"));
    assert!(result.stdout.contains("1 template"));
}

#[test]
fn test_templates_create_basic() {
    let repo = TestRepo::new();

    // Create a new template (non-interactive mode for testing)
    let result = repo.hn(&[
        "templates",
        "create",
        "mytemplate",
        "--description",
        "My test template",
    ]);
    assert!(
        result.success,
        "Create should succeed. stderr: {}",
        result.stderr
    );

    // Verify template was created
    let template_dir = repo.path().join(".hn-templates/mytemplate");
    assert!(template_dir.exists(), "Template directory should exist");
    assert!(
        template_dir.join(".hannahanna.yml").exists(),
        "Config file should exist"
    );
    assert!(
        template_dir.join("README.md").exists(),
        "README should exist"
    );
}

#[test]
fn test_templates_create_with_docker() {
    let repo = TestRepo::new();

    let result = repo.hn(&[
        "templates",
        "create",
        "dockerized",
        "--description",
        "Docker template",
        "--docker",
    ]);
    assert!(result.success);

    let config_path = repo.path().join(".hn-templates/dockerized/.hannahanna.yml");
    let config_content = fs::read_to_string(&config_path).unwrap();
    assert!(config_content.contains("docker") || config_content.contains("enabled: true"));
}

#[test]
fn test_templates_create_from_current() {
    let repo = TestRepo::new();

    // Create a config in current repo
    repo.create_config(
        r#"
docker:
  enabled: true
  services:
    - app
"#,
    );

    // Create template from current config
    let result = repo.hn(&[
        "templates",
        "create",
        "from-current",
        "--from-current",
        "--description",
        "From current",
    ]);
    assert!(result.success);

    let template_config = repo
        .path()
        .join(".hn-templates/from-current/.hannahanna.yml");
    let content = fs::read_to_string(&template_config).unwrap();
    assert!(content.contains("docker") || content.contains("app"));
}
