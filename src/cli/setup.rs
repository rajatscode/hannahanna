// Setup command: Automate hannahanna installation and configuration
//
// Handles:
// - Installing shell completions
// - Setting up shell integration (cd wrapper)
// - Creating example templates
// - Validating environment

use crate::errors::{HnError, Result};
use colored::Colorize;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Shell type for setup
#[derive(Debug, Clone, Copy)]
pub enum SetupShell {
    Bash,
    Zsh,
    Fish,
}

impl SetupShell {
    /// Detect current shell from SHELL environment variable
    pub fn detect() -> Option<Self> {
        env::var("SHELL").ok().and_then(|shell| {
            if shell.contains("bash") {
                Some(SetupShell::Bash)
            } else if shell.contains("zsh") {
                Some(SetupShell::Zsh)
            } else if shell.contains("fish") {
                Some(SetupShell::Fish)
            } else {
                None
            }
        })
    }

    pub fn name(&self) -> &str {
        match self {
            SetupShell::Bash => "bash",
            SetupShell::Zsh => "zsh",
            SetupShell::Fish => "fish",
        }
    }

    /// Get completion file path
    pub fn completion_path(&self) -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| {
            HnError::ConfigError("Could not determine home directory".to_string())
        })?;

        match self {
            SetupShell::Bash => {
                // Try XDG first, fallback to ~/.bash_completion.d
                let xdg_path = home.join(".local/share/bash-completion/completions");
                if xdg_path.exists() || fs::create_dir_all(&xdg_path).is_ok() {
                    Ok(xdg_path.join("hn"))
                } else {
                    let fallback = home.join(".bash_completion.d");
                    fs::create_dir_all(&fallback)?;
                    Ok(fallback.join("hn"))
                }
            }
            SetupShell::Zsh => {
                let path = home.join(".zsh/completions");
                fs::create_dir_all(&path)?;
                Ok(path.join("_hn"))
            }
            SetupShell::Fish => {
                let path = home.join(".config/fish/completions");
                fs::create_dir_all(&path)?;
                Ok(path.join("hn.fish"))
            }
        }
    }

    /// Get shell RC file path
    pub fn rc_file(&self) -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| {
            HnError::ConfigError("Could not determine home directory".to_string())
        })?;

        match self {
            SetupShell::Bash => Ok(home.join(".bashrc")),
            SetupShell::Zsh => Ok(home.join(".zshrc")),
            SetupShell::Fish => Ok(home.join(".config/fish/config.fish")),
        }
    }
}

pub fn run(shell: Option<String>) -> Result<()> {
    println!("{}", "Hannahanna Setup".bright_cyan().bold());
    println!("{}", "=".repeat(60));
    println!();

    // Detect or use specified shell
    let setup_shell = if let Some(shell_name) = shell {
        match shell_name.as_str() {
            "bash" => SetupShell::Bash,
            "zsh" => SetupShell::Zsh,
            "fish" => SetupShell::Fish,
            _ => {
                return Err(HnError::ConfigError(format!(
                    "Unsupported shell: {}. Use bash, zsh, or fish.",
                    shell_name
                )))
            }
        }
    } else if let Some(detected) = SetupShell::detect() {
        println!("✓ Detected shell: {}", detected.name().bright_green());
        detected
    } else {
        eprintln!("{}", "⚠ Could not detect shell from $SHELL".yellow());
        eprintln!("  Please specify: hn setup --shell bash|zsh|fish");
        return Ok(());
    };

    println!();

    // 1. Install shell completions
    println!("{}", "1. Shell Completions".bright_white().bold());
    install_completions(setup_shell)?;
    println!();

    // 2. Setup shell integration
    println!("{}", "2. Shell Integration (cd wrapper)".bright_white().bold());
    setup_shell_integration(setup_shell)?;
    println!();

    // 3. Create example templates (if in a git repo)
    println!("{}", "3. Example Templates".bright_white().bold());
    create_example_templates()?;
    println!();

    // 4. Validate environment
    println!("{}", "4. Environment Validation".bright_white().bold());
    validate_environment()?;
    println!();

    // Final instructions
    println!("{}", "✅ Setup Complete!".bright_green().bold());
    println!();
    println!("{}:", "Next steps".bright_white().bold());
    println!("  1. Reload your shell:");
    match setup_shell {
        SetupShell::Bash => println!("     source ~/.bashrc"),
        SetupShell::Zsh => println!("     source ~/.zshrc"),
        SetupShell::Fish => println!("     source ~/.config/fish/config.fish"),
    }
    println!("  2. Try: hn add <name>");
    println!("  3. Use tab completion: hn <TAB>");
    println!("  4. Explore templates: ls .hn-templates/");

    Ok(())
}

fn install_completions(shell: SetupShell) -> Result<()> {
    let completion_path = shell.completion_path()?;

    // Generate completions using hn binary
    let output = Command::new(env::current_exe()?)
        .args(["completions", shell.name()])
        .output()?;

    if !output.status.success() {
        return Err(HnError::ConfigError(
            "Failed to generate completions".to_string(),
        ));
    }

    // Write to file
    fs::write(&completion_path, output.stdout)?;

    println!(
        "  ✓ Installed completions to {}",
        completion_path.display().to_string().bright_cyan()
    );

    // For zsh, check if completions directory is in fpath
    if matches!(shell, SetupShell::Zsh) {
        println!("  {} Add to ~/.zshrc if not present:", "ℹ".bright_blue());
        println!("    fpath=(~/.zsh/completions $fpath)");
        println!("    autoload -Uz compinit && compinit");
    }

    Ok(())
}

fn setup_shell_integration(shell: SetupShell) -> Result<()> {
    let rc_file = shell.rc_file()?;

    // Check if already installed
    if rc_file.exists() {
        let content = fs::read_to_string(&rc_file)?;
        if content.contains("hn init-shell") || content.contains("hn switch wrapper") {
            println!("  ✓ Shell integration already installed");
            return Ok(());
        }
    }

    // Generate shell integration code
    let output = Command::new(env::current_exe()?).args(["init-shell"]).output()?;

    if !output.status.success() {
        return Err(HnError::ConfigError(
            "Failed to generate shell integration".to_string(),
        ));
    }

    let integration_code = String::from_utf8_lossy(&output.stdout);

    println!("  {} Add the following to {}:", "ℹ".bright_blue(), rc_file.display());
    println!();
    println!("{}", integration_code.dimmed());
    println!();
    println!("  Run: hn init-shell >> {}", rc_file.display());

    Ok(())
}

fn create_example_templates() -> Result<()> {
    // Check if we're in a git repo
    let is_git_repo = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !is_git_repo {
        println!("  ⚠ Not in a git repository - skipping template creation");
        println!("    Run 'hn setup' from your repo root to create example templates");
        return Ok(());
    }

    let templates_dir = PathBuf::from(".hn-templates");

    if templates_dir.exists() {
        println!("  ✓ Templates directory already exists");
        return Ok(());
    }

    // Create example microservice template
    let microservice_dir = templates_dir.join("microservice");
    fs::create_dir_all(&microservice_dir)?;

    let microservice_config = r#"# Microservice Template
# Apply with: hn add my-service --template microservice

docker:
  enabled: true
  ports:
    base:
      app: 3000
      db: 5432

hooks:
  post_create: |
    echo "Setting up microservice environment..."
    npm install
    npm run db:migrate
    echo "✓ Ready to develop!"
"#;

    fs::write(
        microservice_dir.join(".hannahanna.yml"),
        microservice_config,
    )?;
    fs::write(
        microservice_dir.join("README.md"),
        "# Microservice Template\n\nNode.js microservice with database and Docker support.\n",
    )?;

    // Create example frontend template
    let frontend_dir = templates_dir.join("frontend");
    fs::create_dir_all(&frontend_dir)?;

    let frontend_config = r#"# Frontend Template
# Apply with: hn add my-ui --template frontend

docker:
  enabled: true
  ports:
    base:
      app: 8080

hooks:
  post_create: |
    echo "Setting up frontend environment..."
    npm install
    npm run build
    echo "✓ Ready to develop!"
"#;

    fs::write(frontend_dir.join(".hannahanna.yml"), frontend_config)?;
    fs::write(
        frontend_dir.join("README.md"),
        "# Frontend Template\n\nReact/Vue/etc frontend with hot reload.\n",
    )?;

    println!("  ✓ Created example templates:");
    println!("    - .hn-templates/microservice/");
    println!("    - .hn-templates/frontend/");

    Ok(())
}

fn validate_environment() -> Result<()> {
    // Check git version
    let git_output = Command::new("git").arg("--version").output();
    match git_output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("  ✓ git: {}", version.trim().bright_green());
        }
        _ => {
            println!("  {} git: not found", "✗".bright_red());
        }
    }

    // Check docker (optional)
    let docker_output = Command::new("docker").arg("--version").output();
    match docker_output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("  ✓ docker: {} (optional)", version.trim().bright_green());
        }
        _ => {
            println!("  {} docker: not found (optional)", "ℹ".bright_blue());
        }
    }

    // Check current directory
    if let Ok(current_dir) = env::current_dir() {
        println!("  ✓ working directory: {}", current_dir.display().to_string().bright_cyan());
    }

    Ok(())
}
