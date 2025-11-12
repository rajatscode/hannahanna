# Changelog

All notable changes to hannahanna will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-11-12

### 🎉 Major Features

This release adds **sparse checkout support**, **config hierarchy**, and **conditional hooks** - making hannahanna production-ready for large monorepos and complex workflows.

#### ✨ Sparse Checkout (Git & Jujutsu)
- **Monorepo support** - Only checkout the parts you need
- **Git**: Full support via `git sparse-checkout` (cone mode, requires Git 2.25+)
- **Jujutsu**: Full support via `jj sparse set`
- **CLI flag**: `--sparse <path>` (repeatable for multiple paths)
- **Config support**: `sparse.enabled` and `sparse.paths`
- **Per-worktree override** - Different worktrees can have different sparse paths
- **Graceful fallback** - Clear error messages if sparse checkout fails
- **12 comprehensive tests** covering all scenarios
- **Mercurial**: Deferred to v0.3 (will show helpful error message)

**Example:**
```bash
# Only checkout API service and shared libs
hn add feature-api --sparse services/api/ --sparse libs/shared/

# Or configure default sparse paths in .hannahanna.yml:
sparse:
  enabled: true
  paths:
    - services/api/
    - libs/shared/
```

#### 🔧 Config Hierarchy
- **4-level configuration system** with intelligent merging:
  1. `/etc/hannahanna/config.yml` - System-wide defaults (lowest priority)
  2. `~/.config/hannahanna/config.yml` - User preferences
  3. `.hannahanna.yml` - Project config (committed to git)
  4. `.hannahanna.local.yml` - Local overrides (gitignored, highest priority)
- **Deep merge strategy**: Arrays append, primitives override
- **Enhanced `hn config show`** - Displays merged config with source tracking
- **13 comprehensive tests** for all merge scenarios
- **Fully backward compatible** with single-file configs

**Example:**
```yaml
# ~/.config/hannahanna/config.yml (user defaults)
sparse:
  paths:
    - common/libs/

# .hannahanna.yml (project config)
sparse:
  paths:
    - services/api/
hooks:
  post_create: "npm install"

# .hannahanna.local.yml (your overrides)
hooks:
  post_create: "yarn install"

# Result: sparse.paths = ["common/libs/", "services/api/"]
#         hooks.post_create = "yarn install"
```

#### 🪝 Advanced Conditional Hooks
- **Branch pattern matching** for conditional hook execution
- **Three condition types**:
  - `branch.startsWith('prefix')` - Match branch prefix
  - `branch.endsWith('suffix')` - Match branch suffix
  - `branch.contains('substring')` - Match substring anywhere
- **Supported hooks**: `post_create_conditions` and `pre_remove_conditions`
- **Both single and double quotes** supported in conditions
- **Conditional hooks append** across config hierarchy levels
- **Both regular and conditional hooks execute** (regular runs first)
- **19 comprehensive tests** (10 unit + 9 integration)
- **Production-ready** with graceful error handling

**Example:**
```yaml
hooks:
  # Regular hook runs for ALL worktrees
  post_create: "npm install"

  # Conditional hooks run only when branch name matches
  post_create_conditions:
    - condition: "branch.startsWith('feature-')"
      command: "make setup-dev"
    - condition: "branch.startsWith('hotfix-')"
      command: "make setup-prod"
    - condition: "branch.contains('api')"
      command: "docker compose up -d api-deps"
```

### 🔨 Improvements

#### Naming Consistency
- **Standardized state directory** to `.hn-state` (was `.wt-state`)
- **Consistent naming** throughout codebase and documentation
- All references updated to `hannahanna`/`hn`

#### Enhanced Testing
- **236 total tests** (all passing):
  - 78 unit tests (lib)
  - 12 sparse checkout tests
  - 13 config hierarchy tests
  - 17 hooks tests (including conditional hooks)
  - 23 multi-VCS tests
  - 93 other integration tests
- **Enhanced test coverage** for all new features
- **Comprehensive error path testing**

#### Better Error Messages
- Clear error for sparse checkout failures
- Helpful messages for config validation errors
- Improved VCS detection errors

#### Documentation Updates
- Updated README with sparse checkout examples
- Config hierarchy documentation
- Conditional hooks guide
- Monorepo best practices

### 🐛 Bug Fixes
- Fixed unused import warning in hooks module
- Improved config merge behavior for edge cases
- Better handling of missing config files

### 📊 Performance
- No performance regressions
- Config loading remains fast even with 4-level hierarchy
- Sparse checkout significantly reduces disk usage for large monorepos

### 🔜 What's Next: v0.3

See [spec/plan.md](spec/plan.md) for the v0.3 roadmap:
- Additional hooks (pre_create, post_remove, post_switch, pre_integrate, post_integrate)
- More config commands (init, edit templates)
- Extended Docker commands (restart, exec, prune enhancements)
- Aliases support
- Sparse checkout for Mercurial
- Performance optimizations

---

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

See [spec/plan.md](spec/plan.md) and [spec/vision.md](spec/vision.md) for planned features in upcoming releases:
- **v0.3**: Additional hooks, config commands, extended Docker features, aliases
- **v0.4**: Performance optimizations, advanced features
- **v1.0**: Production polish and stabilization

[0.2.0]: https://github.com/rajatscode/hannahanna/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/rajatscode/hannahanna/releases/tag/v0.1.0
