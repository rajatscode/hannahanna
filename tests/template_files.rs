// Integration tests for template file copying (v0.5)
mod common;

use common::TestRepo;
use std::fs;

#[test]
fn test_template_with_files_directory() {
    let repo = TestRepo::new();

    // Create a template with files
    let templates_dir = repo.path().join(".hn-templates");
    let template_dir = templates_dir.join("with-files");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(template_dir.join(".hannahanna.yml"), "# Config\n").unwrap();

    // Create files directory with content
    let files_dir = template_dir.join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(files_dir.join(".env.example"), "PORT=3000\nDB_HOST=localhost\n").unwrap();
    fs::write(files_dir.join("README.txt"), "Template README\n").unwrap();

    // Create nested directory
    fs::create_dir_all(files_dir.join("scripts")).unwrap();
    fs::write(files_dir.join("scripts/setup.sh"), "#!/bin/bash\necho 'Setup'\n").unwrap();

    // Create worktree with template
    let result = repo.hn(&["add", "test-wt", "--template", "with-files"]);
    assert!(result.success, "Add should succeed. stderr: {}", result.stderr);

    // Verify files were copied
    let worktree_path = repo.worktree_path("test-wt");
    assert!(worktree_path.join(".env.example").exists(), ".env.example should be copied");
    assert!(worktree_path.join("README.txt").exists(), "README.txt should be copied");
    assert!(worktree_path.join("scripts/setup.sh").exists(), "Nested file should be copied");

    // Verify content
    let env_content = fs::read_to_string(worktree_path.join(".env.example")).unwrap();
    assert!(env_content.contains("PORT=3000"));
}

#[test]
fn test_template_variable_substitution() {
    let repo = TestRepo::new();

    // Create template with variable substitution
    let templates_dir = repo.path().join(".hn-templates");
    let template_dir = templates_dir.join("with-vars");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(template_dir.join(".hannahanna.yml"), "# Config\n").unwrap();

    let files_dir = template_dir.join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(
        files_dir.join("config.txt"),
        "Worktree: ${HNHN_NAME}\nPath: ${HNHN_PATH}\n"
    ).unwrap();

    // Create worktree
    let result = repo.hn(&["add", "var-test", "--template", "with-vars"]);
    assert!(result.success);

    // Verify variables were substituted
    let worktree_path = repo.worktree_path("var-test");
    let config_content = fs::read_to_string(worktree_path.join("config.txt")).unwrap();
    assert!(config_content.contains("Worktree: var-test"), "HNHN_NAME should be substituted");
    assert!(config_content.contains("Path:"), "HNHN_PATH should be substituted");
}

#[test]
fn test_template_without_files_directory() {
    let repo = TestRepo::new();

    // Create template without files directory
    let templates_dir = repo.path().join(".hn-templates");
    let template_dir = templates_dir.join("no-files");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(template_dir.join(".hannahanna.yml"), "# Config\n").unwrap();
    // No files/ directory created

    // Should still work
    let result = repo.hn(&["add", "test-wt", "--template", "no-files"]);
    assert!(result.success);
}

#[test]
fn test_template_files_preserve_permissions() {
    let repo = TestRepo::new();

    // Create template with executable file
    let templates_dir = repo.path().join(".hn-templates");
    let template_dir = templates_dir.join("with-exec");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(template_dir.join(".hannahanna.yml"), "# Config\n").unwrap();

    let files_dir = template_dir.join("files");
    fs::create_dir_all(&files_dir).unwrap();
    fs::write(files_dir.join("script.sh"), "#!/bin/bash\necho 'test'\n").unwrap();

    // Make executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(files_dir.join("script.sh")).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(files_dir.join("script.sh"), perms).unwrap();
    }

    // Create worktree
    let result = repo.hn(&["add", "exec-test", "--template", "with-exec"]);
    assert!(result.success);

    // Verify file exists and is executable
    let worktree_path = repo.worktree_path("exec-test");
    assert!(worktree_path.join("script.sh").exists());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::metadata(worktree_path.join("script.sh")).unwrap().permissions();
        assert!(perms.mode() & 0o111 != 0, "Script should be executable");
    }
}

#[test]
fn test_template_files_empty_directory() {
    let repo = TestRepo::new();

    // Create template with empty files directory
    let templates_dir = repo.path().join(".hn-templates");
    let template_dir = templates_dir.join("empty-files");
    fs::create_dir_all(&template_dir).unwrap();
    fs::write(template_dir.join(".hannahanna.yml"), "# Config\n").unwrap();
    fs::create_dir_all(template_dir.join("files")).unwrap();

    // Should work fine
    let result = repo.hn(&["add", "empty-test", "--template", "empty-files"]);
    assert!(result.success);
}
