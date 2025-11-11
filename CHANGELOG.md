# Changelog

All notable changes to hannahanna will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-01-11

### 🎉 Initial Release

The first stable release of hannahanna (hn) - a Git worktree manager with isolated development environments.

### ✨ Features

#### Core Commands
- **`hn add <name>`** - Create worktrees with intelligent branch management
  - Support for `--from` to specify base branch
  - Support for `--no-branch` to checkout existing branches
  - Automatic parent/child relationship tracking
- **`hn list [--tree]`** - List all worktrees with optional tree view
- **`hn switch <name>`** - Switch to worktree with fuzzy matching support
- **`hn return [--merge] [--delete]`** - Return to parent worktree with optional merge
- **`hn info [name]`** - Show detailed worktree information
- **`hn remove <name> [--force]`** - Remove worktrees safely
- **`hn prune`** - Clean up orphaned state directories

#### Batch Operations
- **`hn each <command>`** - Execute commands across all worktrees
  - `--parallel` flag for concurrent execution
  - `--stop-on-error` to halt on failures
  - `--filter=<pattern>` for regex-based filtering
  - `--docker-running` to target only active containers

#### Docker Integration
- **`hn docker ps`** - Show container status for all worktrees
- **`hn docker start/stop/restart <name>`** - Container lifecycle management
- **`hn docker logs <name> [service]`** - View container logs
- **`hn docker prune`** - Clean up orphaned containers
- **`hn ports list/show/release`** - Manage port allocations
- Automatic port allocation to avoid conflicts
- Support for both `docker compose` and legacy `docker-compose`

#### Configuration Management
- **`hn config init`** - Create configuration file from template
- **`hn config validate`** - Validate configuration syntax
- **`hn config show`** - Display current configuration
- **`hn config edit`** - Edit configuration in $EDITOR
- Support for `.hannahanna.yml` configuration file

#### Environment Management
- **Shared Resources** - Intelligent symlink/copy of dependencies
  - `node_modules`, `vendor`, `.venv`, etc.
  - Compatibility checking for lockfiles
  - Automatic fallback to copying when needed
- **Hooks System** - Automated setup/teardown
  - `post_create` - Run after worktree creation
  - `pre_remove` - Run before worktree removal
  - `post_switch` - Run after switching worktrees
  - Timeout support with configurable limits

### 🔒 Security & Safety
- Command injection protection in all external command execution
- File locking for concurrent operation safety
- Validation of worktree and service names
- Clear error messages with actionable suggestions
- `--no-hooks` flag for untrusted repositories

### 🧪 Testing
- 132 comprehensive tests covering all functionality
- Integration tests for full lifecycle scenarios
- Docker integration tests
- Concurrency and stress tests
- 85%+ code coverage

### 🛠️ Developer Experience
- Pre-commit hooks (formatting + linting)
- Pre-push hooks (full test suite)
- Installation script: `./scripts/install-git-hooks.sh`
- Fuzzy matching for worktree names
- Colorized, user-friendly output
- Shell integration for `hn switch` and `hn return`

### 📚 Documentation
- Comprehensive README with examples
- Inline code documentation
- Configuration templates with comments
- Error messages with helpful suggestions

### 🎯 Performance
- List 100 worktrees in < 100ms
- Create worktree (no hooks) in < 500ms
- Fuzzy search in < 10ms

### 🔧 Technical Details
- Built with Rust for speed and safety
- Git worktree management via git2-rs
- Concurrent-safe state management with file locking
- Platform support: Linux, macOS, WSL2

---

## Future Releases

See [spec/vision.md](spec/vision.md) for planned features in upcoming releases:
- **v0.2**: Enhanced Docker features, performance monitoring
- **v0.3**: Advanced hooks, configuration hierarchy
- **v0.4**: Team coordination features

[0.1.0]: https://github.com/rajatscode/hannahanna/releases/tag/v0.1.0
