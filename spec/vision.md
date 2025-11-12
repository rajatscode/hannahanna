# hannahanna (hn) - Implementation Plan

## Project Structure

```
hannahanna/
├── src/
│   ├── main.rs                 # CLI entry point
│   ├── cli/                    # Command parsing & dispatch
│   │   ├── mod.rs
│   │   ├── add.rs
│   │   ├── list.rs
│   │   ├── remove.rs
│   │   ├── switch.rs
│   │   ├── info.rs
│   │   ├── each.rs
│   │   ├── integrate.rs
│   │   ├── return.rs
│   │   ├── sync.rs
│   │   ├── config.rs
│   │   ├── docker.rs
│   │   ├── state.rs
│   │   └── ports.rs
│   ├── core/                   # Core domain logic
│   │   ├── mod.rs
│   │   ├── worktree.rs        # Worktree model
│   │   ├── registry.rs        # Worktree registry
│   │   ├── parent_child.rs    # Parent/child tracking
│   │   └── fuzzy.rs           # Fuzzy matching
│   ├── vcs/                    # VCS abstraction
│   │   ├── mod.rs
│   │   ├── traits.rs          # VCS trait
│   │   ├── git.rs             # Git implementation
│   │   ├── hg.rs              # Mercurial (future)
│   │   └── jj.rs              # Jujutsu (future)
│   ├── config/                 # Configuration
│   │   ├── mod.rs
│   │   ├── parser.rs          # YAML parsing
│   │   ├── merge.rs           # Config hierarchy merge
│   │   ├── validation.rs      # Config validation
│   │   └── template.rs        # Template variable rendering
│   ├── docker/                 # Docker integration
│   │   ├── mod.rs
│   │   ├── compose.rs         # Compose file generation
│   │   ├── container.rs       # Container lifecycle
│   │   ├── ports.rs           # Port allocation
│   │   └── health.rs          # Health checks
│   ├── hooks/                  # Hook execution
│   │   ├── mod.rs
│   │   ├── executor.rs        # Shell command execution
│   │   └── context.rs         # Hook context variables
│   ├── state/                  # State management
│   │   ├── mod.rs
│   │   ├── directory.rs       # State directory management
│   │   └── cleanup.rs         # Orphaned state cleanup
│   ├── env/                    # Environment setup
│   │   ├── mod.rs
│   │   ├── symlinks.rs        # Symlink management
│   │   ├── copies.rs          # File copying
│   │   └── compatibility.rs   # Dependency compatibility
│   ├── errors.rs               # Error types
│   └── utils.rs                # Utilities
├── tests/
│   ├── integration/
│   │   ├── test_lifecycle.rs
│   │   ├── test_docker.rs
│   │   ├── test_scenarios.rs
│   │   └── test_vcs.rs
│   └── fixtures/
│       └── test-repos/
├── benches/
│   └── benchmarks.rs
├── spec/
│   ├── spec.md                 # Feature specification
│   └── plan.md                 # This file
├── docs/
│   ├── getting-started.md
│   ├── configuration.md
│   └── docker.md
└── Cargo.toml
```

---

## Phase 1: Core Foundation (MVP)

**Goal:** Working worktree manager with basic operations, no Docker yet.

### Dependencies
```toml
clap = { version = "4.5", features = ["derive"] }     # CLI framework
git2 = "0.18"                                         # Git operations
serde = { version = "1.0", features = ["derive"] }   # Serialization
serde_yaml = "0.9"                                    # YAML config
anyhow = "1.0"                                        # Error handling
thiserror = "1.0"                                     # Custom errors
colored = "2.1"                                       # Terminal colors
dialoguer = "0.11"                                    # Interactive prompts
```

### 1.1 Project Setup

- [x] Initialize Cargo project with workspace structure
- [ ] Set up basic CLI with clap
  - Define main commands: `add`, `list`, `remove`, `switch`, `info`
  - Add `--help` and `--version` flags
- [ ] Define error types with thiserror
  - `VcsError`, `ConfigError`, `WorktreeError`, `ValidationError`
- [ ] Set up logging (consider `tracing` crate)

### 1.2 VCS Abstraction Layer

**Goal:** Abstract VCS operations so we can support Git initially, then Hg/Jj later.

**Files:** `src/vcs/traits.rs`, `src/vcs/git.rs`, `src/vcs/mod.rs`

#### Define VCS Trait
```rust
pub trait VcsBackend {
    fn detect(path: &Path) -> Result<bool>;
    fn create_workspace(&self, name: &str, branch: Option<&str>, base_branch: Option<&str>) -> Result<WorkspacePath>;
    fn list_workspaces(&self) -> Result<Vec<WorkspaceInfo>>;
    fn remove_workspace(&self, name: &str) -> Result<()>;
    fn get_workspace_info(&self, name: &str) -> Result<WorkspaceInfo>;
    fn get_current_branch(&self) -> Result<String>;
    fn has_uncommitted_changes(&self) -> Result<bool>;
    fn integrate(&self, source: &str, target: Option<&str>, strategy: MergeStrategy) -> Result<()>;
}

pub struct WorkspaceInfo {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub commit_hash: String,
    pub vcs_type: VcsType,
}
```

#### Implement Git Backend
- Use `git2` crate (libgit2 bindings)
- Implement all trait methods
- Handle git worktree operations:
  - `git worktree add`
  - `git worktree list`
  - `git worktree remove`
- Store metadata in git config:
  ```
  worktree.<name>.parent = <parent-worktree>
  worktree.<name>.created-at = <timestamp>
  ```

**Tests:**
- Unit test each trait method
- Integration test: create, list, remove worktree
- Test parent tracking via git config

### 1.3 Core Worktree Model

**Files:** `src/core/worktree.rs`, `src/core/registry.rs`

#### Worktree Structure
```rust
pub struct Worktree {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub vcs_type: VcsType,
    pub state_dir: PathBuf,
}
```

#### Registry
- Maintains in-memory view of all worktrees
- Loads from VCS on startup
- Tracks parent/child relationships
- Methods:
  - `load()` - load all worktrees from VCS
  - `get(name)` - get by name with fuzzy matching
  - `list()` - list all
  - `add(worktree)` - register new
  - `remove(name)` - unregister
  - `find_children(name)` - get children
  - `build_tree()` - build parent/child tree

**Tests:**
- Test fuzzy matching (exact > substring > error on ambiguous)
- Test parent/child tracking
- Test tree building

### 1.4 Configuration System

**Files:** `src/config/parser.rs`, `src/config/merge.rs`, `src/config/validation.rs`

#### Config Structure
```rust
#[derive(Deserialize, Serialize)]
pub struct Config {
    pub vcs: VcsConfig,
    pub sparse: SparseConfig,
    pub shared: SharedConfig,
    pub hooks: HooksConfig,
    pub docker: DockerConfig,
    pub build: BuildConfig,
    pub state: StateConfig,
    pub aliases: HashMap<String, String>,
}
```

#### Config Hierarchy
- Load configs from (highest to lowest priority):
  1. `.wt/config.local.yaml` (gitignored)
  2. `.wt/config.yaml` (committed)
  3. `~/.config/wt/config.yaml` (user)
  4. `/etc/wt/config.yaml` (system)
- Deep merge configs (not replace)
- Arrays append, primitives override

#### Template Rendering
```rust
pub struct TemplateContext {
    pub worktree_name: String,
    pub worktree_path: PathBuf,
    pub branch: String,
    pub repo_name: String,
    pub worktree_index: usize,
    // Port variables added in Phase 2
}

pub fn render_template(template: &str, context: &TemplateContext) -> String;
```

**Tests:**
- Test config parsing (valid & invalid YAML)
- Test hierarchy merging
- Test template rendering
- Test validation (required fields, type checking)

### 1.5 Environment Setup

**Files:** `src/env/symlinks.rs`, `src/env/copies.rs`, `src/env/compatibility.rs`

#### Symlink Management
```rust
pub fn create_symlinks(worktree: &Worktree, config: &SharedConfig) -> Result<()>;
pub fn validate_symlink(target: &Path) -> Result<()>; // Security: prevent traversal
```

#### Compatibility Checking
```rust
pub struct CompatibilityChecker {
    checks: HashMap<String, String>, // dir -> lockfile
}

impl CompatibilityChecker {
    pub fn is_compatible(&self, dir: &str, main_repo: &Path, worktree: &Path) -> Result<bool>;
}
```

**Logic:**
1. Check if lockfile exists in both main and worktree
2. Compare hashes
3. If identical → symlink
4. If different → isolated (create separate dir)

#### File Copying
```rust
pub fn copy_files(worktree: &Worktree, config: &SharedConfig) -> Result<()>;
// Handle "template -> dest" syntax
```

**Tests:**
- Test symlink creation
- Test symlink security (no traversal)
- Test compatibility checking (identical vs different lockfiles)
- Test file copying with templating

### 1.6 Hooks System

**Files:** `src/hooks/executor.rs`, `src/hooks/context.rs`

#### Hook Executor
```rust
pub enum HookType {
    PreCreate,
    PostCreate,
    PreRemove,
    PostRemove,
    PostSwitch,
    PreIntegrate,
    PostIntegrate,
}

pub struct HookContext {
    pub env_vars: HashMap<String, String>,
    pub worktree: Worktree,
}

pub fn execute_hook(hook_type: HookType, script: &str, context: &HookContext) -> Result<()>;
```

#### Environment Variables
- `$HNHN_NAME`
- `$HNHN_PATH`
- `$HNHN_BRANCH`
- `$HNHN_PARENT`
- `$HNHN_STATE_DIR`

**Tests:**
- Test hook execution (success & failure)
- Test environment variable injection
- Test hook failure rollback

### 1.7 Commands Implementation

#### `hn add <name> [branch]`
**Algorithm:**
1. Validate name (no special chars, no duplicates)
2. Detect VCS type
3. Get base branch (--from or current)
4. Track parent (current worktree or main branch)
5. Run `pre_create` hooks
6. Create VCS workspace
7. Setup environment:
   - Create state directory
   - Setup symlinks (with compatibility check)
   - Copy template files
8. Run `post_create` hooks
9. Register in registry
10. Print success message with path

**Flags:**
- `--from=<branch>` - base branch
- `--no-branch` - checkout existing
- `--no-setup` - skip env setup
- `--vcs=<git|hg|jj>` - explicit VCS

**Error Handling:**
- Rollback on failure (remove partial worktree)
- Or keep with `--on-failure=keep`

#### `hn list [options]`
**Output (table format):**
```
NAME             BRANCH           COMMIT   STATUS
feature-auth     feature/auth     a1b2c3d
* feature-x      feature/x        d4e5f6g
refactor-db      refactor/db      g7h8i9j
```

**Flags:**
- `--all` - include main workspace
- `--verbose` - full paths, metadata
- `--tree` - parent/child tree view
- `--format=<json|yaml|table>` - output format

**Tree View:**
```
main
├── feature-auth
│   └── fix-oauth-bug
└── refactor-db
```

#### `hn remove <name>`
**Algorithm:**
1. Fuzzy match worktree name
2. Check for uncommitted changes (warn unless --force)
3. Check for children (warn, require --cascade or refuse)
4. Run `pre_remove` hooks
5. Remove VCS workspace
6. Clean state directory (unless --keep-state)
7. Unregister from registry
8. Optionally delete branch (--delete-branch)

**Flags:**
- `--force` - ignore uncommitted changes
- `--delete-branch` - delete branch
- `--keep-state` - preserve state
- `--cascade` - remove children

#### `hn switch <name>`
**Note:** This outputs a path for shell wrapper to `cd` to.

**Algorithm:**
1. Fuzzy match worktree name
2. Run `post_switch` hooks
3. Output worktree path to stdout
4. Print info to stderr (branch, commit, etc.)

**Shell Wrapper (install script):**
```bash
hn() {
    if [ "$1" = "switch" ]; then
        local output=$(command hn switch "$2")
        local path=$(echo "$output" | head -n1)
        cd "$path"
    else
        command hn "$@"
    fi
}
```

#### `hn info [name]`
**Display:**
- Name, path, branch, commit hash
- Parent worktree
- Child worktrees
- VCS type
- Shared resources (symlinks)
- Git status summary (clean, modified, untracked)
- Disk usage

#### `hn each <command>`
**Algorithm:**
1. Get all worktrees
2. For each worktree:
   - Print separator: `==> feature-x`
   - cd to worktree
   - Execute command
   - Print output (colorized)
   - Continue on error (default) or stop (--stop-on-error)

**Flags:**
- `--parallel` - parallel execution (use rayon)
- `--stop-on-error` - abort on first failure
- `--filter=<pattern>` - regex filter on names

### 1.8 Integration Commands (Basic)

#### `hn integrate <source> [--into=<target>]`
**Algorithm:**
1. Fuzzy match source & target
2. Check uncommitted changes
3. Switch to target worktree
4. Merge source branch into target
5. Handle conflicts (interactive)
6. Run `pre_integrate` and `post_integrate` hooks

**Flags:**
- `--into=<target>` - target worktree
- `--no-ff` - force merge commit
- `--squash` - squash commits
- `--strategy=<strategy>` - merge strategy

#### `hn return [--merge]`
**Algorithm:**
1. Get current worktree's parent
2. If no parent, error
3. If `--merge`:
   - Merge current into parent
   - Handle conflicts
4. Switch to parent worktree
5. If `--delete`, remove current worktree

**Flags:**
- `--merge` - merge current into parent
- `--delete` - remove current after merge
- `--no-ff` - force merge commit

### 1.9 State Management

**Files:** `src/state/directory.rs`, `src/state/cleanup.rs`

#### State Directory
- Location: `.hn-state/<worktree-name>/`
- Created on worktree creation
- Removed on worktree removal
- Gitignored (add to `.gitignore` automatically)

#### Commands
```bash
hn state list           # List all state directories
hn state clean          # Remove orphaned state
hn state size [name]    # Disk usage
```

**Tests:**
- Test state directory creation/removal
- Test orphaned state detection
- Test disk usage calculation

### 1.10 Fuzzy Matching

**Files:** `src/core/fuzzy.rs`

**Algorithm:**
1. Try exact match first
2. Try substring match (case-insensitive)
3. If multiple matches, error with suggestions
4. If no matches, error with suggestions (Levenshtein distance)

**Tests:**
- Test exact match preferred
- Test substring match
- Test ambiguous error
- Test no match with suggestions

### 1.11 Testing & Documentation

#### Unit Tests
- VCS trait implementations
- Fuzzy matching
- Config parsing & merging
- Template rendering
- Hook execution
- Symlink management
- Compatibility checking

#### Integration Tests
- Full worktree lifecycle (add, switch, remove)
- Parent/child tracking
- Hook execution in real repos
- Config hierarchy

#### Documentation
- `README.md` - Getting started
- `docs/getting-started.md` - Installation, basic usage
- `docs/configuration.md` - Config file format
- `docs/hooks.md` - Hook system

---

## Phase 2: Docker Integration

**Goal:** Per-worktree Docker containers with automatic port management.

### Dependencies
```toml
bollard = "0.16"                # Docker API client
# Or use Docker CLI via std::process::Command
```

### 2.1 Port Management

**Files:** `src/docker/ports.rs`

#### Port Registry
**Location:** `.wt/state/port-registry.yaml`

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

#### Port Allocator
```rust
pub struct PortAllocator {
    registry: PortRegistry,
    config: PortConfig,
}

impl PortAllocator {
    pub fn allocate(&mut self, worktree: &str, services: &[String]) -> Result<HashMap<String, u16>>;
    pub fn release(&mut self, worktree: &str) -> Result<()>;
    pub fn is_available(&self, port: u16) -> bool;
    pub fn find_next_available(&self, service: &str) -> u16;
}
```

**Algorithm:**
1. Load registry from disk
2. For each service:
   - Try base port + offset
   - If taken, increment until available
   - Check OS port availability (bind test)
3. Save allocation to registry
4. Return port map

#### Commands
```bash
hn ports list              # Show all allocations
hn ports show <name>       # Ports for worktree
hn ports release <name>    # Release ports
hn ports reassign <name>   # Get new ports
```

**Tests:**
- Test port allocation (sequential worktrees)
- Test port exhaustion (range limit)
- Test port conflict detection
- Test registry persistence

### 2.2 Docker Compose Generation

**Files:** `src/docker/compose.rs`

#### Compose Override Generator
```rust
pub struct ComposeGenerator {
    config: DockerConfig,
}

impl ComposeGenerator {
    pub fn generate_override(&self, worktree: &Worktree, ports: &HashMap<String, u16>) -> Result<String>;
}
```

**Generated File:** `<worktree>/.hn-state/docker-compose.override.yml`

**Template:**
```yaml
# Auto-generated by hn - do not edit
# Worktree: {{worktree_name}}

services:
  app:
    ports:
      - "{{port.app}}:3000"
    environment:
      PORT: "{{port.app}}"
      DATABASE_URL: "postgres://localhost:{{port.postgres}}/myapp_{{worktree_name}}"
    volumes:
      - .:/app
      - {{main_repo}}/node_modules:/app/node_modules:ro

volumes:
  postgres-data:
    external: true
    name: myapp-postgres-data
```

**Shared vs Isolated Resources:**
- Shared volumes: External, named with repo prefix
- Isolated volumes: Per-worktree, named with worktree suffix
- Shared networks: External

**Tests:**
- Test compose generation with templates
- Test shared vs isolated volumes
- Test port injection
- Test environment variable rendering

### 2.3 Container Lifecycle

**Files:** `src/docker/container.rs`

#### Container Manager
```rust
pub struct ContainerManager {
    docker: Docker, // bollard client
}

impl ContainerManager {
    pub fn start(&self, worktree: &Worktree) -> Result<()>;
    pub fn stop(&self, worktree: &Worktree) -> Result<()>;
    pub fn restart(&self, worktree: &Worktree) -> Result<()>;
    pub fn status(&self, worktree: &Worktree) -> Result<Vec<ContainerStatus>>;
    pub fn logs(&self, worktree: &Worktree, service: Option<&str>) -> Result<String>;
    pub fn exec(&self, worktree: &Worktree, command: &[String]) -> Result<String>;
    pub fn prune_orphaned(&self) -> Result<()>;
}
```

**Docker Compose Integration:**
- Use Docker Compose CLI: `docker compose -f docker-compose.yml -f .hn-state/docker-compose.override.yml up -d`
- Or use bollard API directly (more control, but complex)

**Project Naming:**
- Template: `{{repo_name}}-{{worktree_name}}`
- Ensures unique container names

**Auto-start on Create:**
- If `docker.auto_start: true`, start containers after `hn add`

**Auto-stop Others:**
- If `docker.auto_stop_others: true`, stop other worktrees' containers on switch

**Tests:**
- Test container start/stop/restart
- Test status retrieval
- Test logs retrieval
- Test orphaned container cleanup

### 2.4 Health Checks

**Files:** `src/docker/health.rs`

#### Health Checker
```rust
pub struct HealthChecker {
    config: HealthCheckConfig,
}

impl HealthChecker {
    pub fn wait_for_healthy(&self, worktree: &Worktree) -> Result<()>;
    pub fn check_service(&self, worktree: &Worktree, service: &str) -> Result<HealthStatus>;
}
```

**Algorithm:**
1. Start containers
2. Poll container health status
3. Timeout if not healthy within `healthcheck.timeout`
4. Print progress spinner

**Tests:**
- Test health check success
- Test health check timeout
- Test unhealthy container detection

### 2.5 Docker Commands

#### `hn docker ps`
**Output:**
```
WORKTREE         SERVICE      STATUS      PORTS
feature-auth     app          Up          0.0.0.0:3000->3000/tcp
feature-auth     postgres     Up          0.0.0.0:5432->5432/tcp
feature-billing  app          Up          0.0.0.0:3001->3000/tcp
feature-billing  postgres     Up          0.0.0.0:5433->5432/tcp
```

#### `hn docker start <name>`
Start containers for worktree.

#### `hn docker stop <name>`
Stop containers for worktree.

#### `hn docker logs <name> [service]`
Show logs. If service specified, show only that service.

#### `hn docker exec <name> <cmd>`
Execute command in container.

#### `hn docker prune`
Remove orphaned containers (worktrees that no longer exist).

### 2.6 Update Core Commands

#### `hn add` - Add Docker Setup
1. Allocate ports
2. Generate compose override
3. Start containers (if auto_start)
4. Wait for health checks
5. Add port info to template context

#### `hn remove` - Add Docker Cleanup
1. Stop containers
2. Remove compose override
3. Release ports
4. Optionally remove volumes

#### `hn switch` - Add Docker Management
1. Optionally start containers (--start-docker)
2. Optionally stop others (--stop-others)

#### `hn list` - Add Docker Status
```
NAME             BRANCH           COMMIT   PORTS          STATUS
feature-auth     feature/auth     a1b2c3d  :3000,:5432    RUNNING
* feature-x      feature/x        d4e5f6g  :3001,:5433    STOPPED
refactor-db      refactor/db      g7h8i9j  -              -
```

#### `hn info` - Add Docker Info
```
Worktree: feature-auth
Path: /path/to/repo-worktrees/feature-auth
Branch: feature/auth
Commit: a1b2c3d

Docker: RUNNING
  app:      localhost:3000
  postgres: localhost:5432
  redis:    localhost:6379

Shared Resources:
  node_modules -> ../node_modules (symlink)
```

### 2.7 Testing

#### Integration Tests
- Test full Docker lifecycle (create, start, stop, remove)
- Test port allocation & reuse
- Test compose override generation
- Test shared vs isolated volumes
- Test health checks

#### Scenario Tests
- Scenario 1: Multiple worktrees with different ports
- Scenario 4: Isolated database testing

---

## Phase 3: Advanced Features

**Goal:** Parent/child workflows, sync, compatibility checking, sparse checkout.

### 3.1 Parent/Child Workflows

#### Enhanced Parent Tracking
- Store in VCS config:
  ```
  worktree.<name>.parent = <parent-name>
  ```
- Maintain bidirectional links (parent knows children)

#### Tree View (`hn list --tree`)
```rust
pub struct WorktreeTree {
    pub root: String,
    pub children: Vec<WorktreeTree>,
}

pub fn build_tree(registry: &Registry) -> WorktreeTree;
```

**Rendering:**
```
main
├── feature-auth (localhost:3000) [RUNNING]
│   └── fix-oauth-bug (localhost:3001) [STOPPED]
└── refactor-db (localhost:3002) [RUNNING]
    ├── optimize-queries (localhost:3003) [STOPPED]
    └── add-indexes (localhost:3004) [RUNNING]
```

#### Cascade Remove
`hn remove feature-auth --cascade`
- Remove feature-auth and all children
- Prompt for confirmation

**Tests:**
- Test parent tracking across create/remove
- Test tree building
- Test cascade remove

### 3.2 Sync Command

#### `hn sync [source-branch] [--strategy=<merge|rebase>]`
**Algorithm:**
1. Get source branch (default: main)
2. Check uncommitted changes
3. Optionally stash (--autostash)
4. Merge or rebase source into current
5. Handle conflicts (interactive)
6. Unstash if stashed

**Flags:**
- `--strategy=<merge|rebase>` - sync strategy
- `--autostash` - stash before, pop after
- `--no-commit` - don't auto-commit

**Tests:**
- Test merge strategy
- Test rebase strategy
- Test conflict handling
- Test autostash

### 3.3 Dependency Compatibility

**Files:** `src/env/compatibility.rs`

#### Enhanced Compatibility Checking
```rust
pub struct DependencyManager {
    checks: HashMap<String, Vec<String>>, // dir -> [lockfiles]
}

impl DependencyManager {
    pub fn check_compatibility(&self, dir: &str, main: &Path, worktree: &Path) -> Result<CompatibilityStatus>;
}

pub enum CompatibilityStatus {
    Compatible,      // Lockfiles identical
    Incompatible,    // Lockfiles differ
    NoLockfile,      // No lockfile found
}
```

#### Behavior on `hn add`
1. Check lockfile in main repo
2. If exists, hash it
3. Create worktree
4. Check lockfile in worktree
5. If hashes match → symlink
6. If differ → warning + isolated

**Warning Message:**
```
Warning: Dependencies differ from main repo
  node_modules: package-lock.json changed
  Using isolated node_modules for this worktree
```

#### Manual Override
```bash
hn add feature-x --isolated=node_modules  # Force isolated
hn add feature-x --shared=node_modules    # Force shared
```

**Tests:**
- Test compatible lockfiles → symlink
- Test incompatible lockfiles → isolated
- Test missing lockfiles
- Test manual override flags

### 3.4 Sparse Checkout

**Files:** `src/vcs/sparse.rs`

#### Sparse Config
```yaml
sparse:
  enabled: true
  paths:
    - services/api/
    - libs/shared/
    - tools/scripts/
```

#### Git Sparse Checkout
```rust
pub fn setup_sparse_checkout(repo: &Path, paths: &[String]) -> Result<()> {
    // git sparse-checkout init
    // git sparse-checkout set <paths>
}
```

#### Override Per Worktree
```bash
hn add feature-api --sparse=services/api/,libs/utils/
```

**Tests:**
- Test sparse checkout (only specified paths)
- Test different sparse paths per worktree
- Test override flag

### 3.5 Config Commands

#### `hn config init [--template=<name>]`
Create `.wt/config.yaml` with template.

**Templates:**
- `default` - Minimal config
- `docker` - With Docker setup
- `monorepo` - With sparse checkout
- `full` - All options documented

#### `hn config validate`
Validate config syntax and semantics.

#### `hn config show`
Display current config (merged from hierarchy).

#### `hn config edit`
Open config in `$EDITOR`.

**Tests:**
- Test config init with templates
- Test validation (valid & invalid)
- Test show (hierarchy merge)

### 3.6 Aliases

**Config:**
```yaml
aliases:
  sw: switch
  rm: remove
  mk: add
  dk: docker
  ls: list
```

**Usage:**
```bash
hn sw feature-x    # → hn switch feature-x
hn dk ps           # → hn docker ps
```

**Implementation:**
- Resolve alias at CLI parsing stage
- Recursive alias resolution (with cycle detection)

**Tests:**
- Test alias resolution
- Test recursive aliases
- Test cycle detection

---

## Phase 4: Multi-VCS & Polish - ✅ COMPLETED

**Goal:** Support Mercurial and Jujutsu, performance optimization, comprehensive testing.

**Status:** All multi-VCS backends implemented and production-ready in v0.1.0

### 4.1 Mercurial Support - ✅ COMPLETED

**Files:** `src/vcs/mercurial.rs` (464 lines)

#### Mercurial Backend
- ✅ Use `hg share` for workspace creation
- ✅ Registry in `.hg/wt-registry.json` for metadata tracking
- ✅ Commands implemented:
  - Create: `hg share <source> <dest>`
  - List: Parse registry
  - Remove: `rm -rf` + registry update
  - Parent tracking: Registry-based
  - Status: `hg status` integration

**Implementation Details:**
- ✅ Full VcsBackend trait implementation
- ✅ Parent tracking via JSON registry
- ✅ Workspace status detection
- ✅ Production-ready

**Tests:**
- ✅ Hg backend implementation tested
- ✅ Share creation/removal verified
- ✅ Registry management validated

### 4.2 Jujutsu Support - ✅ COMPLETED

**Files:** `src/vcs/jujutsu.rs` (331 lines)

#### Jujutsu Backend
- ✅ Native `jj workspace add` integration
- ✅ Native workspace list/remove support
- ✅ Change tracking via `jj log -r @`

**Implementation Details:**
- ✅ Full VcsBackend trait implementation
- ✅ Native workspace commands
- ✅ Branch/change tracking
- ✅ Production-ready

**Tests:**
- ✅ Jj backend implementation tested
- ✅ Native workspace operations verified

### 4.3 CLI Integration - ✅ COMPLETED

**Implementation:**
- ✅ Global `--vcs` flag for explicit VCS selection
- ✅ Auto-detection of VCS type (Jujutsu → Git → Mercurial)
- ✅ All 11 commands support multi-VCS:
  - add, list, remove, switch, info
  - integrate, sync, return
  - each, prune, cleanup
- ✅ Backend initialization helper (`backend_init.rs`)
- ✅ Seamless VCS switching

**Tests:**
- ✅ 186 total tests (all passing)
- ✅ 23 multi-VCS integration tests
- ✅ End-to-end VCS switching validated
- ✅ All backends verified in production scenarios

### 4.4 Performance Optimization

#### Caching
- Cache worktree list (invalidate on changes)
- Cache port registry (in-memory)
- Cache config (invalidate on file changes)

#### Lazy Loading
- Load worktree info on-demand
- Avoid git operations when unnecessary

#### Benchmarks
```rust
use criterion::*;

fn bench_list_100_worktrees(c: &mut Criterion) { ... }
fn bench_fuzzy_match_1000(c: &mut Criterion) { ... }
fn bench_port_allocation(c: &mut Criterion) { ... }
```

**Targets:**
- List 100 worktrees: < 50ms
- Fuzzy search 1000 worktrees: < 10ms
- Port allocation: < 1ms

### 4.4 Comprehensive Testing

#### Stress Tests
```rust
#[test]
fn stress_500_worktrees() { ... }

#[test]
fn stress_port_exhaustion() { ... }
```

#### Cross-Platform Tests
- Linux CI
- macOS CI
- Windows WSL CI

#### Security Tests
```rust
#[test]
fn test_no_command_injection() { ... }

#[test]
fn test_symlink_traversal() { ... }
```

#### End-to-End Scenario Tests
- Scenario 1: Multiple features in parallel
- Scenario 2: Hotfix during feature work
- Scenario 3: Nested worktrees
- Scenario 4: Isolated database testing
- Scenario 5: Code review
- Scenario 6: Monorepo with sparse checkout

### 4.5 Documentation

#### User Docs
- `docs/getting-started.md` - Installation, first worktree
- `docs/configuration.md` - Config file reference
- `docs/docker.md` - Docker integration guide
- `docs/hooks.md` - Hook system
- `docs/workflows.md` - Common workflows
- `docs/troubleshooting.md` - Common issues

#### API Docs
- Rustdoc for all public APIs
- Architecture overview
- VCS trait documentation

#### Examples
- `examples/basic/` - Basic usage
- `examples/docker/` - Docker setup
- `examples/monorepo/` - Sparse checkout
- `examples/hooks/` - Custom hooks

### 4.6 Distribution

#### Cargo
```toml
[package]
name = "hannahanna"
version = "0.1.0"
authors = ["..."]
edition = "2021"
description = "Git worktree manager with Docker integration"
license = "MIT OR Apache-2.0"
repository = "https://github.com/..."
```

**Publish:**
```bash
cargo publish
```

**Install:**
```bash
cargo install hannahanna
```

#### Pre-built Binaries
- GitHub Actions CI
- Build for: Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (WSL)
- Attach to GitHub releases

#### Package Managers
- **Homebrew:**
  ```ruby
  class Hannahanna < Formula
    desc "Git worktree manager with Docker integration"
    homepage "https://github.com/..."
    url "https://github.com/.../archive/v0.1.0.tar.gz"
    # ...
  end
  ```
- **apt/deb:** Create `.deb` package
- **AUR:** Arch Linux user repository

#### Shell Integration
**Install script:**
```bash
# Install shell wrapper for `hn switch`
hn shell install
```

**Generates:**
```bash
# ~/.config/hn/hn.bash
hn() {
    if [ "$1" = "switch" ]; then
        local output=$(command hn switch "$2")
        local path=$(echo "$output" | head -n1)
        cd "$path"
    else
        command hn "$@"
    fi
}
```

**User adds to `~/.bashrc`:**
```bash
source ~/.config/hn/hn.bash
```

---

## Development Workflow

### Test-Driven Development
1. Write failing test
2. Implement feature
3. Pass test
4. Refactor
5. Repeat

### Continuous Integration
- Run tests on every commit
- Lint with `clippy`
- Format with `rustfmt`
- Check test coverage (aim for 80%+)

### Version Control
- Use conventional commits
- Feature branches
- PR reviews

---

## Success Criteria

### Phase 1: Core Foundation - ✅ COMPLETED
- [x] All core commands work (add, list, remove, switch, info, each)
- [x] Git worktrees created/managed correctly
- [x] Config system loads (single YAML file)
- [x] Hooks execute successfully
- [x] Symlinks created with compatibility checking
- [x] Parent/child tracking works
- [x] Fuzzy matching works
- [x] 80%+ test coverage
- [x] Documentation exists

### Phase 2: Docker & Integration - ✅ COMPLETED
- [x] Docker containers start/stop per worktree
- [x] Ports auto-allocated and managed
- [x] Compose overrides generated correctly
- [x] Shared/isolated volumes work
- [x] Integration commands (integrate, sync) implemented
- [x] Return command (Graphite-style workflows)
- [x] Integration tests pass
- [x] All scenarios work end-to-end

### Phase 3: Multi-VCS Foundation - ✅ COMPLETED
- [x] VCS abstraction layer (VcsBackend trait)
- [x] Git backend refactored to trait
- [x] Mercurial backend fully implemented
- [x] Jujutsu backend fully implemented
- [x] Auto-detection of VCS type
- [x] Tree view renders correctly
- [x] Parent/child workflows complete
- [x] Dependency compatibility checking works

### Phase 4: Multi-VCS CLI & Polish - ✅ COMPLETED
- [x] Hg and Jj backends integrated into CLI
- [x] All commands support multi-VCS
- [x] Global --vcs flag functional
- [x] 186 tests passing (all backends tested)
- [x] Production-ready multi-VCS support
- [ ] Performance benchmarks met (deferred to v0.2)
- [ ] Stress tests (500+ worktrees) (deferred to v0.2)
- [ ] Published to crates.io (pending)
- [ ] Pre-built binaries (pending)

---

## Next Steps

1. **Start with Phase 1:** Core foundation
2. **Create Cargo project:** Initialize with dependencies
3. **Setup CLI framework:** Define commands with clap
4. **Implement VCS abstraction:** Define trait, implement Git backend
5. **Build core model:** Worktree, Registry, fuzzy matching
6. **Add config system:** Parse, merge, validate
7. **Implement commands:** Start with `add`, then `list`, `remove`, `switch`
8. **Write tests:** Unit and integration tests for each feature
9. **Document:** Write docs as you go

**Ready to start?** Let's begin with Phase 1!
