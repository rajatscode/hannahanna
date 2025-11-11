# hannahanna (hn) - Feature Specification

**Version:** 1.0
**Command:** `hn`
**Name Origin:** Hannahanna - Hittite mother goddess

> **Note:** This document describes the complete feature set and vision for hannahanna.
> - For the **MVP implementation plan** (what we're building first), see [`plan.md`](plan.md)
> - For the **long-term comprehensive roadmap** (full vision), see [`vision.md`](vision.md)

## Goal

Enable parallel work on the same codebase across different branches with complete, isolated development environments.

## Core Philosophy

`hannahanna` extends VCS workspace concepts (git worktrees, hg shares, jj workspaces) to provide complete development environment isolation: dependencies, configuration, Docker containers, build artifacts, and state.

**Each worktree = one complete, reproducible development environment**

---

## 1. Core Worktree Operations

### 1.1 Create

```bash
hn add <name> [branch] [options]
```

**Behavior:**
- Create VCS workspace (auto-detect or explicit `--vcs`)
- Support creating from any branch via `--from=<branch>`
- Track parent/child relationships automatically
- Execute lifecycle hooks
- Setup environment (symlinks, copies, isolated resources)
- Allocate Docker ports if configured
- Generate worktree-specific configs
- Support sparse checkout if configured

**Options:**
- `--from=<branch>` - base branch (default: current)
- `--no-branch` - checkout existing branch
- `--template=<name>` - environment template
- `--no-setup` - skip environment setup
- `--no-docker` - skip Docker
- `--vcs=<git|hg|jj>` - explicit VCS type

**Parent Tracking:**
- Created from within worktree → track that worktree as parent
- Created from main repo → track current branch as parent
- Store in VCS config (git config, hgrc, jj config)

### 1.2 List

```bash
hn list [options]
```

**Display:**
- All workspaces with current marker
- Branch/change name
- Commit hash (short)
- Docker status (running/stopped/none)
- Allocated ports
- Parent/child relationships (with `--tree`)

**Options:**
- `--all` - include primary workspace
- `--verbose` - full paths, metadata
- `--tree` - parent/child tree view
- `--docker` - Docker container status
- `--ports` - port allocations
- `--format=<json|yaml|table>` - output format

**Tree View Example:**
```
main
├── feature-auth (localhost:3000) [RUNNING]
│   └── fix-oauth-bug (localhost:3001) [STOPPED]
└── refactor-db (localhost:3002) [RUNNING]
```

### 1.3 Remove

```bash
hn remove <name> [options]
```

**Behavior:**
- Run pre-remove hooks
- Stop Docker containers
- Remove VCS workspace
- Clean up state directory
- Release port allocations
- Optionally delete branch
- Warn if uncommitted changes or children exist

**Options:**
- `--force` - ignore uncommitted changes
- `--delete-branch` - delete branch too
- `--keep-state` - preserve state directory
- `--cascade` - remove child worktrees

### 1.4 Switch

```bash
hn switch <name>
```

**Behavior:**
- Output path for shell wrapper to cd
- Run post-switch hooks
- Optionally start Docker
- Update environment variables
- Display worktree info

**Options:**
- `--start-docker` - start containers
- `--stop-others` - stop other containers
- `--no-hooks` - skip post-switch hooks

### 1.5 Info

```bash
hn info [name]
```

**Display:**
- Path, branch, commit
- Parent worktree
- Child worktrees
- VCS type
- Docker status & ports
- Shared/isolated resources
- Git status summary
- Disk usage

### 1.6 Each

```bash
hn each <command>
```

**Behavior:**
- Execute command in each worktree
- Show worktree name before output
- Colorized output
- Continue on error (default) or `--stop-on-error`

**Options:**
- `--parallel` - parallel execution
- `--stop-on-error` - abort on first failure
- `--filter=<pattern>` - only matching names
- `--docker-running` - only worktrees with Docker running

---

## 2. Integration Operations

### 2.1 Integrate

```bash
hn integrate <source> [options]
```

**Behavior:**
- Merge source (worktree name, branch name, or commit) into target
- Target defaults to current worktree
- Check for uncommitted changes
- Support merge strategies

**Options:**
- `--into=<target>` - target worktree/branch
- `--no-ff` - force merge commit
- `--squash` - squash commits
- `--strategy=<strategy>` - merge strategy

**Examples:**
```bash
hn integrate fix-123              # Merge fix-123 into current
hn integrate fix-123 --into main  # Merge into main
hn integrate main                 # Pull main into current
```

### 2.2 Return

```bash
hn return [options]
```

**Behavior:**
- Return to parent worktree
- Optionally merge current into parent first
- Handle missing parent worktree gracefully
- Switch to parent after merge

**Options:**
- `--merge` - merge current into parent
- `--delete` - delete current worktree after merge
- `--no-ff` - force merge commit

**Example Workflow:**
```bash
hn add feature-x
hn add fix-oauth    # Child of feature-x
# Fix bug...
hn return --merge   # Merge into feature-x, switch back
```

### 2.3 Sync

```bash
hn sync [source-branch] [options]
```

**Behavior:**
- Sync current worktree with another branch (typically main)
- Support merge or rebase strategy
- Handle conflicts
- Stash uncommitted changes if needed

**Options:**
- `--strategy=<merge|rebase>` - sync strategy
- `--autostash` - stash before, pop after
- `--no-commit` - don't auto-commit

---

## 3. Environment Management

### 3.1 Configuration File

**Location:** `.wt/config.yaml` in repository root

**Full Structure:**

```yaml
# VCS-specific overrides
vcs:
  git:
    # Git-specific settings
  hg:
    # Mercurial settings
  jj:
    # Jujutsu settings

# Sparse checkout (for monorepos)
sparse:
  enabled: false
  paths:
    - services/api/
    - libs/shared/
  # When enabled, only these paths checked out in worktrees

# Shared resources (symlinked from main repo)
shared:
  symlinks:
    - node_modules     # Share if compatible
    - .build-cache
    - vendor
    - docker-volumes

  # Detect compatibility before sharing
  compatibility_check:
    node_modules: "package-lock.json"   # Share if lockfile identical
    vendor: "composer.lock"

  # If incompatible, fall back to isolated
  fallback_to_isolated: true

  # Files to copy (not symlink)
  copy:
    - .env.template -> .env
    - config/local.yml.example -> config/local.yml

# Lifecycle hooks
hooks:
  post_create: |
    echo "Setting up worktree..."
    npm install
    make setup

  pre_remove: |
    make cleanup

  post_switch: |
    echo "Switched to: {{worktree_name}}"

  # Conditional hooks
  post_create_conditions:
    - condition: "branch.startsWith('feature/')"
      command: "make setup-dev"
    - condition: "branch.startsWith('hotfix/')"
      command: "make setup-prod"

# Docker configuration
docker:
  strategy: "per-worktree"  # or "shared" or "manual"

  # Compose file in main repo
  compose_file: "docker-compose.yml"

  # Port management
  ports:
    strategy: "auto-offset"
    base: 3000
    range: [3000, 4000]
    fallback: "find-available"

    # Service port offsets
    services:
      app: 3000
      postgres: 5432
      redis: 6379

  # Resource sharing
  shared:
    volumes:
      - postgres-data    # Shared database
      - redis-data
    networks:
      - myapp-net

  isolated:
    volumes:
      - app-cache        # Per-worktree cache
      - logs

  # Behavior
  auto_start: true
  auto_stop_others: false

  # Project naming
  project_name_template: "{{repo_name}}-{{worktree_name}}"

  # Environment variables (templated)
  env:
    DATABASE_URL: "postgres://localhost:{{port.postgres}}/myapp_{{worktree_name}}"
    REDIS_URL: "redis://localhost:{{port.redis}}/0"
    PORT: "{{port.app}}"

  # Health checks
  healthcheck:
    enabled: true
    timeout: 30s
    services:
      - app
      - postgres

# Build configuration
build:
  # Shared build caches
  cache_dirs:
    - target/        # Rust
    - .gradle/
    - .ccache/

  # Build command on create
  setup_command: "make build"

# State management
state:
  enabled: true
  location: ".wt-state"  # Gitignored in main repo
  per_worktree:
    - logs/
    - temp-uploads/

# Available template variables:
# {{worktree_name}}, {{worktree_path}}, {{branch}},
# {{repo_name}}, {{worktree_index}}, {{port.<service>}}
```

### 3.2 Sparse Checkout Support

**Purpose:** For large monorepos, only checkout relevant paths

**Configuration:**
```yaml
sparse:
  enabled: true
  paths:
    - services/api/
    - libs/shared/
    - tools/scripts/
```

**Behavior:**
- When creating worktree, only checkout specified paths
- Different worktrees can have different sparse paths
- Overridable per worktree:
  ```bash
  hn add feature-api --sparse=services/api/,libs/utils/
  ```

**VCS Support:**
- **Git:** ✅ Full support via `git sparse-checkout` (cone mode)
- **Jujutsu:** ✅ Full support via `jj sparse set`
- **Mercurial:** ⏸️ Not supported in v0.2 (deferred to v0.3)
  - Mercurial has experimental sparse checkout via `hg sparse`
  - Requires additional testing for `hg share` compatibility
  - Will gracefully fail with error message if attempted

**Use Cases:**
- 100GB monorepo, only need 5GB for your service
- Faster checkout and disk usage
- Reduce node_modules/build artifact size

### 3.3 Dependency Compatibility

**Problem:** Different worktrees need different dependency versions

**Solution:** Compatibility checking before sharing

```yaml
shared:
  symlinks:
    - node_modules

  compatibility_check:
    node_modules: "package-lock.json"

  fallback_to_isolated: true
```

**Behavior:**
```bash
hn add feature-a
# Shares node_modules (package-lock.json identical to main)

# Edit package.json in feature-a, change react version
npm install

hn add feature-b
# Detects different package-lock.json
# Warning: "Dependencies differ from main. Using isolated node_modules."
# Creates feature-b with own node_modules/
```

### 3.4 Config Commands

```bash
hn config init [--template=<name>]  # Create .wt/config.yaml
hn config validate                   # Validate syntax
hn config show                       # Display current config
hn config edit                       # Open in $EDITOR
```

---

## 4. Docker Management

### 4.1 Docker Commands

```bash
hn docker ps                    # Container status for all worktrees
hn docker start <name>          # Start containers
hn docker stop <name>           # Stop containers
hn docker restart <name>        # Restart containers
hn docker logs <name> [service] # View logs
hn docker exec <name> <cmd>     # Execute in container
hn docker prune                 # Clean orphaned containers
hn docker ports [name]          # Show port allocations
```

### 4.2 Port Management

**Port Registry:** `.wt/state/port-registry.yaml`

```yaml
allocations:
  feature-x:
    app: 3000
    postgres: 5432
    redis: 6379
  feature-y:
    app: 3001
    postgres: 5433
    redis: 6380

next_available:
  app: 3002
  postgres: 5434
  redis: 6381
```

**Port Commands:**
```bash
hn ports list              # Show all allocations
hn ports show <name>       # Ports for worktree
hn ports release <name>    # Release ports
hn ports reassign <name>   # Get new ports
```

**Behavior:**
- Auto-allocate on create
- Detect conflicts, auto-fallback to next available
- Persistent across sessions
- Release on remove

### 4.3 Docker Compose Override

Auto-generated per worktree:

```yaml
# Auto-generated by hn - do not edit
# Worktree: feature-x

services:
  app:
    ports:
      - "3000:3000"
    environment:
      PORT: "3000"
      DATABASE_URL: "postgres://localhost:5432/myapp_feature_x"
    volumes:
      - .:/app
      - /path/to/main/node_modules:/app/node_modules:ro

volumes:
  postgres-data:
    external: true
    name: myapp-postgres-data
```

---

## 5. State Management

### 5.1 State Directory

**Location:** `.wt-state/<worktree-name>/` (gitignored)

**Contents:**
- `docker-compose.override.yml`
- Worktree-specific configs
- Logs, temp files, cache
- Local databases (if isolated)

**Commands:**
```bash
hn state list             # Show all state
hn state clean            # Remove orphaned state
hn state size [name]      # Disk usage
```

---

## 6. VCS Abstraction

### 6.1 Supported VCS

**Git:**
- Native worktrees via libgit2 (git2 crate)
- Metadata in git config

**Mercurial:**
- Workspaces via hg share
- Registry for tracking shares
- Metadata in hgrc or registry

**Jujutsu:**
- Native workspace support
- Metadata in jj config

### 6.2 VCS Detection

Auto-detect order:
1. Check `.jj/` → Jujutsu
2. Check `.git/` → Git
3. Check `.hg/` → Mercurial
4. Error if none found

Override: `hn add feature --vcs=git`

### 6.3 Compatibility Matrix

| Feature | Git | Hg | Jj |
|---------|-----|----|-----|
| Create workspace | ✅ | ✅ | ✅ |
| List workspaces | ✅ | ✅ | ✅ |
| Remove workspace | ✅ | ✅ | ✅ |
| Parent tracking | ✅ | ✅ | ✅ |
| Metadata storage | ✅ Config | ✅ Registry | ✅ Config |
| Sparse checkout | ✅ | ⏸️ v0.2+ | ✅ |
| Production Status | ✅ v0.1.0 | ✅ v0.1.0 | ✅ v0.1.0 |

---

## 7. Advanced Features

### 7.1 Fuzzy Matching

**Behavior:**
- Match by substring
- Exact match preferred
- Case-insensitive
- Error on ambiguous matches

**Examples:**
```bash
hn switch feat      # → feature-auth (unique match)
hn remove auth      # → feature-auth (unique match)
hn switch bug       # Error: ambiguous (bugfix-123, debug-tool)
```

### 7.2 Hooks

**Available Hooks:**
- `pre_create`, `post_create`
- `pre_remove`, `post_remove`
- `post_switch`
- `pre_integrate`, `post_integrate`

**Context Variables:**
- `$WT_NAME` - worktree name
- `$WT_PATH` - worktree path
- `$WT_BRANCH` - branch name
- `$WT_PARENT` - parent worktree
- `$WT_PORTS_*` - allocated ports
- `$WT_STATE_DIR` - state directory

**Example:**
```yaml
hooks:
  post_create: |
    echo "Created worktree: $WT_NAME"
    echo "Branch: $WT_BRANCH"
    echo "App port: $WT_PORTS_APP"
```

### 7.3 Aliases

**Config:**
```yaml
aliases:
  sw: switch
  rm: remove
  mk: add
  dk: docker
```

**Usage:**
```bash
hn sw feature-x    # → hn switch feature-x
hn dk ps           # → hn docker ps
```

---

## 8. Configuration Hierarchy

**Status:** ✅ Implemented in v0.2

hannahanna supports multi-level configuration merging, allowing users to have system-wide defaults, user preferences, project settings, and local overrides.

**Priority (highest to lowest):**
1. `.hannahanna.local.yml` - Repo-specific, gitignored (highest priority, local overrides)
2. `.hannahanna.yml` - Repo-specific, committed (project defaults)
3. `~/.config/hannahanna/config.yml` - User global preferences
4. `/etc/hannahanna/config.yml` - System-wide defaults

**Merge Strategy:**
- **Deep merge** (not replace) - configs are combined intelligently
- **Arrays append** - shared_resources, sparse paths, docker volumes, etc. are combined from all levels
- **Primitives override** - boolean and string values from higher priority configs override lower ones

**Example:**

User config (`~/.config/hannahanna/config.yml`):
```yaml
sparse:
  enabled: true
  paths:
    - common/libs/

docker:
  ports:
    base:
      app: 3000
```

Project config (`.hannahanna.yml`):
```yaml
sparse:
  paths:
    - services/api/

hooks:
  post_create: "npm install"

docker:
  enabled: true
  ports:
    base:
      postgres: 5432
```

Local override (`.hannahanna.local.yml`):
```yaml
hooks:
  post_create: "yarn install"

docker:
  ports:
    base:
      app: 4000
```

**Merged Result:**
```yaml
sparse:
  enabled: true          # From user config
  paths:
    - common/libs/       # From user config
    - services/api/      # From project config (appended)

hooks:
  post_create: "yarn install"  # From local (overrides project)

docker:
  enabled: true          # From project config
  ports:
    base:
      app: 4000          # From local (overrides user)
      postgres: 5432     # From project config
```

**Commands:**
```bash
hn config show      # Display merged configuration with sources
hn config validate  # Validate all config files in hierarchy
hn config edit      # Edit project config (.hannahanna.yml)
```

**View Merged Config:**
```bash
hn config show
```

Output shows:
- Which config files were loaded
- Priority order (highest first)
- The final merged result
- Merge strategy information

---

## 9. Error Handling

**Requirements:**
- Clear, actionable error messages
- Suggest fixes when possible
- Safe rollback on failure
- Validate inputs early

**Example Messages:**

```
Error: Worktree 'feature-x' already exists

Suggestions:
  Remove: hn remove feature-x
  Rename: hn add feature-x-v2
  Switch: hn switch feature-x
```

```
Error: Port 3000 in use by process 12345

Auto-assigning port 3005 instead.
```

**Rollback:**
- If hook fails → remove worktree (default)
- Or keep partial via `--on-failure=keep`

---

## Usage Scenarios

### Scenario 1: Multiple Features in Parallel

```bash
# Frontend developer, 3 features simultaneously
hn add feature-auth
hn add feature-billing
hn add feature-dashboard

hn list --docker
# feature-auth      (localhost:3000) [RUNNING]
# feature-billing   (localhost:3001) [RUNNING]
# feature-dashboard (localhost:3002) [RUNNING]

# Work on auth
hn switch feature-auth
# Browser at localhost:3000

# Quick fix on billing
hn switch feature-billing
# Browser at localhost:3001

# Auth done
hn integrate feature-auth --into=main
hn remove feature-auth --delete-branch
```

### Scenario 2: Hotfix During Feature Work

```bash
# Deep in refactor
hn add refactor-db --from=main

# Urgent bug!
hn add hotfix-critical --from=main  # Not from refactor-db

# Fix bug
hn switch hotfix-critical
# Fix, test, commit

# Merge to main
hn integrate hotfix-critical --into=main
hn remove hotfix-critical --delete-branch

# Back to refactor
hn switch refactor-db
```

### Scenario 3: Nested Worktrees

```bash
# Working on big feature
hn add feature-redesign --from=main

# Discover bug while implementing
hn add fix-button-bug  # Auto-parent: feature-redesign

# Fix bug
hn switch fix-button-bug
# Fix, commit

# Merge to parent
hn return --merge  # → feature-redesign

# Continue feature

# View tree
hn list --tree
# main
# └── feature-redesign
#     └── fix-button-bug [merged]
```

### Scenario 4: Isolated Database Testing

```yaml
# config: each worktree gets own DB
docker:
  isolated:
    volumes: [postgres-data]
  env:
    DATABASE_URL: "postgres://localhost:{{port.postgres}}/myapp_{{worktree_name}}"
```

```bash
hn add test-migration
# Gets own postgres

# Run risky migration
make migrate

# Breaks? No problem, isolated
hn remove test-migration
```

### Scenario 5: Code Review

```bash
# Review colleague's PR
git fetch origin pull/123/head:pr-123
hn add review-pr-123 pr-123 --no-branch

# Test locally at localhost:3000
open http://localhost:3000

# Done
hn remove review-pr-123
```

### Scenario 6: Monorepo with Sparse Checkout

```yaml
# .wt/config.yaml
sparse:
  enabled: true
  paths:
    - services/api/
    - libs/shared/
```

```bash
hn add feature-api
# Only checks out api/ and libs/
# 5GB instead of 100GB
```

---

## Testing Requirements

### Unit Tests (80%+ Coverage)

**Core:**
- VCS trait implementations
- Workspace operations
- Fuzzy matching
- Port allocation
- Config parsing
- Template rendering
- Hook execution
- Sparse checkout
- Parent/child tracking

### Integration Tests

**Full Lifecycle:**
```rust
#[test]
fn test_worktree_lifecycle() {
    let wt = wt_add("feature-x", None, None);
    assert!(wt.path.exists());

    wt_switch("feature-x");
    assert_eq!(current_worktree(), "feature-x");

    wt_remove("feature-x", false);
    assert!(!wt.path.exists());
}
```

**Docker:**
```rust
#[test]
fn test_docker_lifecycle() {
    let wt = wt_add("test-docker", None, None);

    let containers = docker_ps(&wt);
    assert!(!containers.is_empty());

    wt_docker_stop("test-docker");
    assert!(docker_ps(&wt).is_empty());

    wt_remove("test-docker", false);
    assert!(!port_allocated("test-docker"));
}
```

**Multi-VCS:**
```rust
#[test]
fn test_git() { test_vcs(VcsType::Git); }

#[test]
fn test_hg() { test_vcs(VcsType::Mercurial); }

#[test]
fn test_jj() { test_vcs(VcsType::Jujutsu); }
```

**Sparse Checkout:**
```rust
#[test]
fn test_sparse_checkout() {
    let config = Config {
        sparse: SparseConfig {
            enabled: true,
            paths: vec!["services/api/".into()],
        },
        ..Default::default()
    };

    let wt = wt_add_with_config("sparse-test", None, config);

    assert!(wt.path.join("services/api").exists());
    assert!(!wt.path.join("services/worker").exists());
}
```

### End-to-End Tests

Each usage scenario should have E2E test:

```rust
#[test]
fn scenario_multiple_features() {
    wt_add("feature-auth", None, None);
    wt_add("feature-billing", None, None);

    assert_eq!(get_port("feature-auth", "app"), 3000);
    assert_eq!(get_port("feature-billing", "app"), 3001);

    // Verify node_modules shared
    assert!(is_symlinked("feature-auth", "node_modules"));
}
```

### Performance Benchmarks

**Targets:**
- List 100 worktrees: < 50ms
- Create worktree (no Docker): < 500ms
- Create worktree (with Docker): < 5s
- Fuzzy search 1000 worktrees: < 10ms
- Port allocation: < 1ms

```rust
use criterion::*;

fn bench_list(c: &mut Criterion) {
    let repo = setup_with_worktrees(100);
    c.bench_function("list 100", |b| {
        b.iter(|| wt_list())
    });
}
```

### Stress Tests

```rust
#[test]
fn stress_500_worktrees() {
    for i in 0..500 {
        wt_add(&format!("wt-{}", i), None, None);
    }
    assert_eq!(wt_list().len(), 500);
}

#[test]
fn stress_port_exhaustion() {
    // Range: 3000-3010 (10 ports)
    for i in 0..10 {
        wt_add(&format!("wt-{}", i), None, None);
    }

    // 11th fails gracefully
    let result = wt_add("wt-11", None, None);
    assert!(result.is_err());
}
```

### Cross-Platform Tests

**Matrix:**
- OS: Ubuntu, macOS, Windows (WSL)
- VCS: Git 2.30+, Hg 6.0+, Jj 0.10+
- Docker: 20.10+, 23.0+, 24.0+

### Security Tests

```rust
#[test]
fn test_no_command_injection() {
    let result = wt_add("test; rm -rf /", None, None);
    assert!(result.is_err());
}

#[test]
fn test_symlink_traversal() {
    let config = Config {
        shared: SharedConfig {
            symlinks: vec!["../../../../etc/passwd".into()],
        },
        ..Default::default()
    };

    let result = wt_add_with_config("test", None, config);
    assert!(result.is_err());
}
```

---

## Success Criteria

**Feature Complete:**
- ✅ All commands implemented
- ✅ Docker integration working
- ✅ Multi-VCS support (Git, Mercurial, Jujutsu) - v0.1.0
- ⏸️ Sparse checkout (deferred to v0.2+)
- ✅ Config system functional
- ✅ Hooks working

**Performance:**
- ✅ Meets all benchmarks
- ✅ Handles 100+ worktrees efficiently

**Reliability:**
- ✅ 80%+ test coverage (186 tests)
- ✅ All scenarios pass
- ✅ No panics
- ✅ Graceful errors

**Usability:**
- ✅ Clear error messages
- ✅ Good documentation
- ✅ Easy onboarding

**Compatibility:**
- ✅ Linux, macOS, Windows (WSL)
- ✅ Git 2.30+, Hg 6.0+, Jj 0.10+
- ✅ Docker 20.10+

**Distribution:**
- ✅ Single binary < 10MB
- ✅ Pre-built binaries
- ✅ `cargo install hannahanna`
- ✅ Package managers (brew, apt)
