# hannahanna (hn) - MVP Implementation Plan

**Philosophy:** Ship fast, iterate based on real usage. Focus on core worktree management, defer complexity.

**See also:** `vision.md` for the comprehensive long-term roadmap.

---

## MVP Scope: What We're Building

### Core Value Proposition
**Enable developers to work on multiple branches simultaneously with isolated environments.**

### What's In v0.1.0 (MVP - Completed)
- ✅ Git worktree management (create, list, delete, switch, info)
- ✅ Parent/child tracking for nested workflows
- ✅ Fuzzy name matching
- ✅ Shared resource management (node_modules, vendor, etc.)
- ✅ Compatibility checking (lockfile comparison)
- ✅ Basic hooks (post_create, pre_remove, post_switch)
- ✅ Simple config (one YAML file)
- ✅ Concurrency-safe state management
- ✅ Docker integration (opt-in with --docker flag)
- ✅ Port allocation system
- ✅ `hn each` command for batch operations
- ✅ `hn return` command for Graphite-style workflows

### Phase 2 (Completed) - Integration Operations
- ✅ `hn integrate` - Merge branches with fuzzy matching
- ✅ `hn sync` - Keep branches up-to-date with merge/rebase strategies

### Phase 3 (Completed) - Multi-VCS Foundation
- ✅ VCS abstraction layer (trait-based design)
- ✅ Git backend implementation (via VcsBackend trait)
- ✅ Mercurial backend (using `hg share` and registry tracking)
- ✅ Jujutsu backend (using native `jj workspace` commands)
- ✅ Auto-detection of VCS type (Jujutsu → Git → Mercurial)

### Phase 4 (Completed) - Multi-VCS CLI Integration
- ✅ Global `--vcs` flag for explicit VCS type selection
- ✅ All 11 commands support multi-VCS (add, list, remove, switch, info, integrate, sync, return, each, prune, cleanup)
- ✅ Mercurial backend fully integrated and production-ready
- ✅ Jujutsu backend fully integrated and production-ready
- ✅ Comprehensive multi-VCS test suite (23 tests)

### What's Deferred to v0.2+
- ⏸️ Sparse checkout (v0.2+ - monorepo edge case)
- ⏸️ Config hierarchy (v0.2+ - one file is enough)
- ⏸️ Advanced hooks with conditions (v0.2+)
- ⏸️ Team coordination features (v0.3+)

### What We're Never Building
- ❌ Separate Docker/state/port subcommands - keep it simple
- ❌ Windows native support - Linux/macOS/WSL2 only

---

## Commands (11 Total)

### Core Commands (v0.1.0)
```bash
hn create <name> [options]   # Create worktree
hn list [--tree]             # List worktrees
hn delete <name> [--force]   # Delete worktree
hn switch <name>             # Switch to worktree (via shell wrapper)
hn info [name]               # Show worktree details
hn prune                     # Clean orphaned state
```

### Workflow Commands (Phase 2)
```bash
hn integrate <source> [into] # Merge branches with fuzzy matching
hn sync [branch]             # Keep branch up-to-date (merge/rebase)
```

### Batch Operations (v0.1.0)
```bash
hn each <command>            # Run command in all worktrees
hn return [--merge|--delete] # Return from nested worktree (Graphite-style)
```

**Simple, focused, easy to remember.**

---

## Project Structure

```
hannahanna/
├── src/
│   ├── main.rs                 # CLI entry point (~300 lines)
│   ├── cli/                    # Command handlers (~800 lines)
│   │   ├── mod.rs
│   │   ├── create.rs
│   │   ├── list.rs
│   │   ├── delete.rs
│   │   ├── switch.rs
│   │   ├── info.rs
│   │   ├── prune.rs
│   │   ├── each.rs             # Batch operations (v0.1.0)
│   │   ├── return_cmd.rs       # Graphite-style return (v0.1.0)
│   │   ├── integrate.rs        # Branch integration (Phase 2)
│   │   └── sync.rs             # Branch sync (Phase 2)
│   ├── worktree/               # Core domain logic (~800 lines)
│   │   ├── mod.rs
│   │   ├── manager.rs          # Main API
│   │   ├── model.rs            # Worktree struct
│   │   ├── registry.rs         # Tracking worktrees
│   │   └── fuzzy.rs            # Fuzzy matching
│   ├── vcs/                    # VCS operations (~1700 lines)
│   │   ├── mod.rs
│   │   ├── traits.rs           # VCS abstraction (Phase 3)
│   │   ├── backend_init.rs     # Backend initialization (Phase 4)
│   │   ├── git.rs              # Git implementation via git2
│   │   ├── mercurial.rs        # Mercurial backend (Phase 3-4)
│   │   └── jujutsu.rs          # Jujutsu backend (Phase 3-4)
│   ├── env/                    # Environment setup (~400 lines)
│   │   ├── mod.rs
│   │   ├── symlinks.rs         # Symlink management
│   │   └── compatibility.rs    # Lockfile comparison
│   ├── config.rs               # Config loading (~200 lines)
│   ├── state.rs                # State management (~200 lines)
│   ├── hooks.rs                # Hook execution (~200 lines)
│   ├── errors.rs               # Error types (~100 lines)
│   └── utils.rs                # Utilities (~100 lines)
├── tests/
│   ├── basic_worktree.rs       # Basic worktree operations
│   ├── docker_integration.rs   # Docker lifecycle tests
│   ├── each_command.rs         # Batch operation tests
│   ├── environment.rs          # Environment setup tests
│   ├── fuzzy_matching.rs       # Fuzzy matching tests
│   ├── hooks.rs                # Hook execution tests
│   ├── integrate_sync.rs       # Phase 2 integration tests
│   ├── multi_vcs.rs            # Phase 3 VCS backend tests
│   ├── return_workflow.rs      # Return command tests
│   ├── scenarios.rs            # Real-world scenario tests
│   ├── worktree_lifecycle.rs   # Full lifecycle tests
│   └── common/
│       └── mod.rs              # Test utilities
├── spec/
│   ├── spec.md                 # Feature spec (reference)
│   ├── plan.md                 # This file (MVP plan)
│   └── vision.md               # Long-term comprehensive plan
└── Cargo.toml

Total: ~5,500 lines (v0.1.0 + Phase 2 + Phase 3 + Phase 4)
Test count: 186 tests (all passing)
```

---

## Dependencies (Minimal)

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }   # CLI framework
git2 = "0.18"                                        # Git operations via libgit2
anyhow = "1.0"                                       # Error handling
thiserror = "1.0"                                    # Custom errors
serde = { version = "1.0", features = ["derive"] }  # Serialization
serde_yml = "0.0.12"                                 # Config parsing
serde_json = "1.0"                                   # JSON for Mercurial registry (Phase 3)
sha2 = "0.10"                                        # Hashing for compatibility checks
fs2 = "0.4"                                          # File locking
colored = "2.1"                                      # Terminal colors
tempfile = "3.8"                                     # Temp dirs
regex = "1.10"                                       # Pattern matching
chrono = "0.4"                                       # Timestamps (Phase 2)

[target.'cfg(unix)'.dependencies]
libc = "0.2"                                         # Unix-specific operations

[dev-dependencies]
tempfile = "3.8"                                     # Temp dirs for testing
```

No Docker client, no async runtime. Keep it simple.

---

## MVP Implementation Phases

### Phase 0: Foundation (Week 1)

**Goal:** Working Rust project with CLI framework and basic structure.

**Tasks:**
1. Initialize Cargo project
2. Set up clap CLI with 6 commands
3. Define error types with thiserror
4. Set up integration test framework
5. Create basic project structure

**Deliverable:** `hn --help` works, shows all commands.

---

### Phase 1: Git Integration (Week 1-2)

**Goal:** Create and list git worktrees.

#### 1.1 Git Operations

**File:** `src/vcs/git.rs`

```rust
pub struct GitBackend {
    repo: Repository,  // git2::Repository
    repo_root: PathBuf,
}

impl GitBackend {
    pub fn new() -> Result<Self>;

    // Core operations
    pub fn create_worktree(&self, name: &str, opts: &CreateOpts) -> Result<Worktree>;
    pub fn list_worktrees(&self) -> Result<Vec<Worktree>>;
    pub fn remove_worktree(&self, name: &str) -> Result<()>;

    // Metadata (stored in git config)
    pub fn get_parent(&self, name: &str) -> Result<Option<String>>;
    pub fn set_parent(&self, name: &str, parent: &str) -> Result<()>;

    // Utilities
    pub fn current_branch(&self) -> Result<String>;
    pub fn has_uncommitted_changes(&self) -> Result<bool>;
}

pub struct CreateOpts {
    pub base_branch: Option<String>,  // --from
    pub new_branch: bool,              // Create new branch (default: true)
}
```

**Git Config Storage:**
```
worktree.<name>.parent = <parent-worktree-name>
worktree.<name>.created = 2025-11-10T12:34:56Z
```

**Tests:**
- Create worktree from current branch
- Create worktree from specific branch (--from)
- List worktrees
- Remove worktree
- Parent tracking persists

#### 1.2 Worktree Model

**File:** `src/worktree/model.rs`

```rust
pub struct Worktree {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub commit: String,      // Short hash
    pub parent: Option<String>,
    pub created: DateTime<Utc>,
}

pub struct WorktreeInfo {
    pub worktree: Worktree,
    pub children: Vec<String>,
    pub is_current: bool,
    pub disk_usage: Option<u64>,
}
```

#### 1.3 Registry

**File:** `src/worktree/registry.rs`

```rust
pub struct Registry {
    git: GitBackend,
    worktrees: Vec<Worktree>,
}

impl Registry {
    pub fn load(git: &GitBackend) -> Result<Self>;
    pub fn get(&self, name: &str) -> Result<&Worktree>;  // With fuzzy matching
    pub fn find_children(&self, name: &str) -> Vec<String>;
    pub fn build_tree(&self) -> WorktreeTree;
}
```

**Fuzzy Matching:**
```rust
pub fn fuzzy_match(query: &str, candidates: &[String]) -> Result<String> {
    // 1. Exact match (case-sensitive)
    // 2. Exact match (case-insensitive)
    // 3. Substring match (case-insensitive)
    // 4. Error with suggestions if ambiguous
    // 5. Error with Levenshtein suggestions if no match
}
```

#### 1.4 Commands: create, list

**File:** `src/cli/create.rs`

```rust
pub fn run(name: String, opts: CreateOpts) -> Result<()> {
    // 1. Validate name
    // 2. Load config
    // 3. Create git worktree
    // 4. Set parent (if created from within worktree)
    // 5. Print success
}
```

**File:** `src/cli/list.rs`

```rust
pub fn run(opts: ListOpts) -> Result<()> {
    // 1. Load registry
    // 2. Format output (table or tree)
    // 3. Print
}
```

**Deliverable:**
```bash
hn create feature-x
hn list
# feature-x    feature/x    a1b2c3d
```

---

### Phase 2: Environment Setup (Week 2-3)

**Goal:** Shared resource management with compatibility checking.

#### 2.1 Config System

**File:** `src/config.rs`

```rust
#[derive(Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub shared: SharedConfig,

    #[serde(default)]
    pub hooks: HooksConfig,
}

#[derive(Deserialize, Serialize, Default)]
pub struct SharedConfig {
    pub symlinks: Vec<String>,
    pub compatibility_check: HashMap<String, String>,  // dir -> lockfile
    pub fallback_to_isolated: bool,
}

#[derive(Deserialize, Serialize, Default)]
pub struct HooksConfig {
    pub post_create: Option<String>,
    pub pre_remove: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        // Load from .wt/config.yaml (if exists)
        // Otherwise return Default
    }
}
```

**Example config:**
```yaml
shared:
  symlinks:
    - node_modules
    - vendor

  compatibility_check:
    node_modules: "package-lock.json"
    vendor: "composer.lock"

  fallback_to_isolated: true

hooks:
  post_create: |
    echo "Installing dependencies..."
    npm install
```

**Tests:**
- Parse valid config
- Handle missing config (use defaults)
- Validate config structure

#### 2.2 Compatibility Checking

**File:** `src/env/compatibility.rs`

```rust
pub struct CompatibilityChecker {
    checks: HashMap<String, String>,  // dir -> lockfile
}

impl CompatibilityChecker {
    pub fn is_compatible(&self, dir: &str, main_repo: &Path, worktree: &Path) -> Result<bool> {
        // 1. Get lockfile path from config
        // 2. Check if exists in both main and worktree
        // 3. Compare file hashes (fast: first 1KB, then full if needed)
        // 4. Return true if identical, false if different
    }
}
```

**Tests:**
- Identical lockfiles → compatible
- Different lockfiles → incompatible
- Missing lockfiles → incompatible
- Non-existent directory → compatible (nothing to check)

#### 2.3 Symlink Management

**File:** `src/env/symlinks.rs`

```rust
pub struct SymlinkManager {
    config: SharedConfig,
    checker: CompatibilityChecker,
}

impl SymlinkManager {
    pub fn setup(&self, worktree: &Worktree, main_repo: &Path) -> Result<Vec<SymlinkAction>> {
        // For each symlink in config:
        //   1. Check compatibility (if check configured)
        //   2. If compatible: create symlink
        //   3. If incompatible: skip, return warning
        //   4. Return list of actions taken
    }

    fn validate_symlink_target(&self, target: &Path, repo_root: &Path) -> Result<()> {
        // Security: ensure target is within repo_root
        // Canonicalize both paths
        // Check target.starts_with(repo_root)
        // Error if traversal detected
    }
}

pub enum SymlinkAction {
    Created { source: PathBuf, target: PathBuf },
    Skipped { dir: String, reason: String },
}
```

**Tests:**
- Create symlink when compatible
- Skip symlink when incompatible
- Reject symlink traversal (../../../../etc/passwd)
- Reject symlinks outside repo root

#### 2.4 State Management

**File:** `src/state.rs`

```rust
pub struct StateManager {
    state_root: PathBuf,  // .wt-state/
}

impl StateManager {
    pub fn new() -> Result<Self>;

    pub fn create_state_dir(&self, worktree_name: &str) -> Result<PathBuf>;
    pub fn remove_state_dir(&self, worktree_name: &str) -> Result<()>;
    pub fn list_orphaned(&self, active_worktrees: &[String]) -> Vec<String>;
    pub fn clean_orphaned(&self, active_worktrees: &[String]) -> Result<Vec<String>>;
}
```

**State Directory Structure:**
```
.wt-state/
├── feature-x/
│   └── metadata.json  # Future: store worktree metadata
├── feature-y/
└── .gitignore         # Auto-generated: *
```

**Tests:**
- Create state directory
- Remove state directory
- Detect orphaned directories
- Clean orphaned directories

#### 2.5 Update create Command

**File:** `src/cli/create.rs`

```rust
pub fn run(name: String, opts: CreateOpts) -> Result<()> {
    // 1. Validate name
    // 2. Load config
    // 3. Create git worktree
    // 4. Set parent (if created from within worktree)
    // 5. Create state directory           // NEW
    // 6. Setup symlinks                   // NEW
    // 7. Run post_create hook (if set)   // NEW
    // 8. Print success + warnings
}
```

**Deliverable:**
```bash
hn create feature-x
# Creating worktree 'feature-x'...
# ✓ Git worktree created at ../feature-x
# ✓ Shared node_modules (compatible)
# ⚠ Skipped vendor (incompatible lockfile)
# ✓ Running post_create hook...
# Done!
```

---

### Phase 3: Delete, Switch, Info (Week 3)

#### 3.1 Delete Command

**File:** `src/cli/delete.rs`

```rust
pub fn run(name: String, opts: DeleteOpts) -> Result<()> {
    // 1. Fuzzy match name
    // 2. Check uncommitted changes (warn unless --force)
    // 3. Check for children (error, suggest deleting them first)
    // 4. Run pre_remove hook (if set)
    // 5. Stop any processes in worktree? (nice-to-have)
    // 6. Remove git worktree
    // 7. Remove state directory
    // 8. Print success
}

pub struct DeleteOpts {
    pub force: bool,  // Ignore uncommitted changes
}
```

**Tests:**
- Delete worktree successfully
- Refuse to delete with uncommitted changes (unless --force)
- Refuse to delete with children
- Clean up state directory

#### 3.2 Switch Command

**File:** `src/cli/switch.rs`

```rust
pub fn run(name: String) -> Result<()> {
    // 1. Fuzzy match name
    // 2. Output path to stdout (for shell wrapper)
    // 3. Print info to stderr
}
```

**Shell Wrapper:**
```bash
# Install with: hn init-shell >> ~/.bashrc

hn() {
    if [ "$1" = "switch" ]; then
        local path=$(command hn switch "$2" 2>/dev/null)
        if [ $? -eq 0 ]; then
            cd "$path"
            command hn switch "$2" >/dev/null  # Print info
        else
            command hn switch "$2"  # Print error
        fi
    else
        command hn "$@"
    fi
}
```

**Tests:**
- Output correct path
- Error on non-existent worktree
- Fuzzy matching works

#### 3.3 Info Command

**File:** `src/cli/info.rs`

```rust
pub fn run(name: Option<String>) -> Result<()> {
    // If name is None, use current worktree
    // 1. Get worktree info
    // 2. Get parent/children
    // 3. Get shared resources (symlinks)
    // 4. Get git status
    // 5. Get disk usage
    // 6. Format and print
}
```

**Output:**
```
Worktree: feature-x
Path: /home/user/repo-worktrees/feature-x
Branch: feature/x
Commit: a1b2c3d Fix authentication bug

Parent: main
Children: fix-oauth-bug

Git Status:
  Modified: 2 files
  Untracked: 1 file

Shared Resources:
  node_modules → ../node_modules (symlink)

Disk Usage: 1.2 GB
```

**Tests:**
- Show info for current worktree
- Show info for named worktree
- Display parent/children
- Display symlinks

---

### Phase 4: Hooks & Polish (Week 4)

#### 4.1 Hook Execution

**File:** `src/hooks.rs`

```rust
pub struct HookExecutor {
    config: HooksConfig,
}

impl HookExecutor {
    pub fn run_hook(&self, hook_type: HookType, worktree: &Worktree) -> Result<()> {
        let script = match hook_type {
            HookType::PostCreate => &self.config.post_create,
            HookType::PreRemove => &self.config.pre_remove,
        };

        if let Some(script) = script {
            // 1. Set environment variables
            let env = self.build_env(worktree);

            // 2. Execute shell command
            let output = Command::new("sh")
                .arg("-c")
                .arg(script)
                .current_dir(&worktree.path)
                .envs(env)
                .output()?;

            // 3. Check exit code
            if !output.status.success() {
                return Err(HookError::Failed {
                    hook: hook_type,
                    output: String::from_utf8_lossy(&output.stderr).to_string(),
                }.into());
            }
        }

        Ok(())
    }

    fn build_env(&self, worktree: &Worktree) -> HashMap<String, String> {
        // WT_NAME, WT_PATH, WT_BRANCH, WT_PARENT, WT_STATE_DIR
    }
}

pub enum HookType {
    PostCreate,
    PreRemove,
}
```

**Tests:**
- Execute hook successfully
- Fail on non-zero exit code
- Pass environment variables
- Handle missing hook (no-op)

#### 4.2 Prune Command

**File:** `src/cli/prune.rs`

```rust
pub fn run(opts: PruneOpts) -> Result<()> {
    // 1. Load registry (get active worktrees)
    // 2. Find orphaned state directories
    // 3. Print what will be deleted
    // 4. Confirm (unless --force)
    // 5. Delete orphaned directories
    // 6. Print summary
}
```

**Output:**
```bash
hn prune
# Found 3 orphaned state directories:
#   .wt-state/old-feature-1/  (1.2 GB)
#   .wt-state/old-feature-2/  (800 MB)
#   .wt-state/test-branch/    (200 MB)
#
# Total: 2.2 GB will be freed
#
# Continue? [y/N] y
# ✓ Cleaned 3 directories
```

#### 4.3 Tree View

**File:** `src/cli/list.rs` (update)

```rust
pub fn render_tree(registry: &Registry) -> String {
    // 1. Build tree structure
    let tree = registry.build_tree();

    // 2. Render with box-drawing characters
    format_tree(&tree, 0)
}

fn format_tree(node: &WorktreeTree, depth: usize) -> String {
    // Use │, ├, └ characters for tree rendering
}
```

**Output:**
```bash
hn list --tree
# main
# ├── feature-auth
# │   └── fix-oauth-bug
# └── refactor-db
#     ├── optimize-queries
#     └── add-indexes
```

---

### Phase 5: Concurrency Safety (Week 4-5)

**Goal:** Prevent race conditions and corruption at scale.

#### 5.1 File Locking

**File:** `src/state.rs` (update)

```rust
use std::fs::File;
use std::os::unix::fs::FileLockExt;  // Unix only for MVP

pub struct StateLock {
    file: File,
}

impl StateLock {
    pub fn acquire(lock_file: &Path, timeout: Duration) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(lock_file)?;

        // Try to acquire exclusive lock with timeout
        let start = Instant::now();
        loop {
            match file.try_lock_exclusive() {
                Ok(()) => return Ok(Self { file }),
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    if start.elapsed() > timeout {
                        return Err(LockError::Timeout.into());
                    }
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}

impl Drop for StateLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
```

**Usage:**
```rust
pub fn create_worktree(name: &str) -> Result<()> {
    // Acquire lock before modifying shared state
    let _lock = StateLock::acquire(".wt-state/.lock", Duration::from_secs(5))?;

    // Critical section: create worktree, update state
    // Lock automatically released when _lock drops
}
```

**Tests:**
- Acquire lock successfully
- Block concurrent access
- Timeout if lock held too long
- Release lock on drop

#### 5.2 Atomic Operations

**File:** `src/state.rs` (update)

```rust
impl StateManager {
    pub fn create_state_dir(&self, worktree_name: &str) -> Result<PathBuf> {
        let state_dir = self.state_root.join(worktree_name);

        // Atomic: create temp dir, then rename
        let temp_dir = self.state_root.join(format!(".tmp-{}", worktree_name));
        fs::create_dir_all(&temp_dir)?;

        // Write metadata
        let metadata = WorktreeMetadata {
            name: worktree_name.to_string(),
            created: Utc::now(),
        };
        let metadata_path = temp_dir.join("metadata.json");
        fs::write(metadata_path, serde_json::to_string_pretty(&metadata)?)?;

        // Atomic rename
        fs::rename(&temp_dir, &state_dir)?;

        Ok(state_dir)
    }
}
```

**Tests:**
- Create state atomically
- Handle partial creation (cleanup temp dir on error)
- No orphaned temp directories

#### 5.3 Concurrency Tests

**File:** `tests/integration/concurrency_test.rs`

```rust
#[test]
fn test_concurrent_create() {
    let handles: Vec<_> = (0..10)
        .map(|i| {
            thread::spawn(move || {
                create_worktree(&format!("feature-{}", i))
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter()
        .map(|h| h.join().unwrap())
        .collect();

    // All should succeed
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 10);

    // All should have state directories
    assert_eq!(list_worktrees().unwrap().len(), 10);
}

#[test]
fn test_lock_timeout() {
    // Thread 1: Hold lock for 10 seconds
    let handle = thread::spawn(|| {
        let _lock = StateLock::acquire(".wt-state/.lock", Duration::from_secs(10)).unwrap();
        thread::sleep(Duration::from_secs(10));
    });

    // Thread 2: Try to acquire with 1 second timeout
    thread::sleep(Duration::from_millis(100));  // Let thread 1 acquire first
    let result = StateLock::acquire(".wt-state/.lock", Duration::from_secs(1));

    // Should timeout
    assert!(matches!(result, Err(LockError::Timeout)));

    handle.join().unwrap();
}
```

---

### Phase 6: Testing & Documentation (Week 5-6)

#### 6.1 Integration Tests

**File:** `tests/integration/lifecycle_test.rs`

```rust
#[test]
fn test_full_lifecycle() {
    let temp = TempRepo::new();

    // Create worktree
    let result = create_worktree("feature-x", None);
    assert!(result.is_ok());

    // List worktrees
    let worktrees = list_worktrees().unwrap();
    assert_eq!(worktrees.len(), 1);
    assert_eq!(worktrees[0].name, "feature-x");

    // Get info
    let info = get_worktree_info("feature-x").unwrap();
    assert_eq!(info.worktree.name, "feature-x");

    // Delete worktree
    let result = delete_worktree("feature-x", false);
    assert!(result.is_ok());

    // Verify deleted
    let worktrees = list_worktrees().unwrap();
    assert_eq!(worktrees.len(), 0);
}

#[test]
fn test_parent_child_tracking() {
    // Create parent
    create_worktree("feature-x", None).unwrap();

    // Switch to parent
    // (simulate being in feature-x worktree)
    std::env::set_var("GIT_DIR", "feature-x/.git");

    // Create child
    create_worktree("fix-bug", None).unwrap();

    // Verify parent tracking
    let child = get_worktree_info("fix-bug").unwrap();
    assert_eq!(child.worktree.parent, Some("feature-x".to_string()));

    // Verify parent knows about child
    let parent = get_worktree_info("feature-x").unwrap();
    assert_eq!(parent.children, vec!["fix-bug"]);
}
```

**File:** `tests/integration/scenarios_test.rs`

```rust
// Implement each scenario from spec.md as integration test

#[test]
fn scenario_multiple_features() {
    // Create 3 worktrees
    create_worktree("feature-auth", None).unwrap();
    create_worktree("feature-billing", None).unwrap();
    create_worktree("feature-dashboard", None).unwrap();

    // Verify all exist
    let worktrees = list_worktrees().unwrap();
    assert_eq!(worktrees.len(), 3);
}

#[test]
fn scenario_hotfix_during_feature() {
    // Deep in refactor
    create_worktree("refactor-db", Some("main")).unwrap();

    // Urgent bug! Create from main, not refactor-db
    create_worktree("hotfix-critical", Some("main")).unwrap();

    // Verify hotfix parent is main, not refactor-db
    let hotfix = get_worktree_info("hotfix-critical").unwrap();
    assert_eq!(hotfix.worktree.parent, None);  // Created from main
}

#[test]
fn scenario_nested_worktrees() {
    // Create parent
    create_worktree("feature-redesign", Some("main")).unwrap();

    // Create child (simulate being in feature-redesign)
    std::env::set_var("GIT_DIR", "feature-redesign/.git");
    create_worktree("fix-button-bug", None).unwrap();

    // Verify tree structure
    let tree = build_worktree_tree().unwrap();
    assert_eq!(tree.children.len(), 1);
    assert_eq!(tree.children[0].worktree.name, "feature-redesign");
    assert_eq!(tree.children[0].children.len(), 1);
    assert_eq!(tree.children[0].children[0].worktree.name, "fix-button-bug");
}
```

#### 6.2 Documentation

**Create:**
- `README.md` - Getting started, installation, basic usage
- `docs/commands.md` - Command reference
- `docs/config.md` - Configuration file format
- `docs/workflows.md` - Common workflows and examples

**README.md outline:**
```markdown
# hannahanna (hn)

Work on multiple git branches simultaneously with isolated environments.

## Installation

cargo install hannahanna

## Quick Start

# Create worktrees
hn create feature-x
hn create feature-y

# List worktrees
hn list

# Switch between worktrees
hn switch feature-x

# Delete worktree
hn delete feature-x

## Configuration

Create `.wt/config.yaml`:

```yaml
shared:
  symlinks:
    - node_modules
  compatibility_check:
    node_modules: "package-lock.json"

hooks:
  post_create: "npm install"
```

## See Also

- [Commands](docs/commands.md)
- [Configuration](docs/config.md)
- [Workflows](docs/workflows.md)
```

---

## Success Criteria

### MVP (v0.1.0) - ✅ COMPLETED

**Functionality:**
- ✅ All 6 core commands work correctly
- ✅ Git worktrees created and managed
- ✅ Parent/child tracking persists
- ✅ Symlinks created with compatibility checking
- ✅ Hooks execute successfully
- ✅ Concurrency-safe (file locking works)
- ✅ Docker integration (opt-in with --docker flag)
- ✅ Port allocation system
- ✅ Batch operations with `hn each`
- ✅ Graphite-style workflow with `hn return`

**Quality:**
- ✅ 80%+ test coverage (186 tests)
- ✅ All integration tests pass (184 passing, 2 ignored for Phase 4)
- ✅ No panics (graceful error handling)
- ✅ Clear error messages with suggestions

**Performance:**
- ✅ List 100 worktrees in < 100ms
- ✅ Create worktree (no hooks) in < 500ms
- ✅ Fuzzy search in < 10ms

**Documentation:**
- ✅ README with getting started
- ✅ Command reference
- ✅ Config documentation
- ✅ Example workflows

**Distribution:**
- ✅ Published to crates.io
- ✅ Binary builds for Linux/macOS
- ✅ Shell wrapper installation works

### Phase 2: Integration Operations - ✅ COMPLETED

**Implementation:**
- ✅ `hn integrate` command with fuzzy matching and merge options
- ✅ `hn sync` command with merge/rebase strategies
- ✅ Support for --no-ff, --squash, --strategy flags
- ✅ Autostash support for uncommitted changes
- ✅ 11 comprehensive integration tests

**Features:**
- ✅ Merge branches with fuzzy name matching
- ✅ Keep branches synchronized with upstream
- ✅ Validate conflicting options (--squash + --no-ff)
- ✅ Handle uncommitted changes gracefully

### Phase 3: Multi-VCS Foundation - ✅ COMPLETED

**Architecture:**
- ✅ VCS abstraction layer via `VcsBackend` trait
- ✅ Auto-detection of VCS type (Jujutsu → Git → Mercurial)
- ✅ Git backend refactored to implement trait
- ✅ Full Mercurial backend with registry tracking
- ✅ Full Jujutsu backend with native workspace commands

**Backends:**
- ✅ Git: Using libgit2 via existing implementation
- ✅ Mercurial: Using `hg share` with JSON registry
- ✅ Jujutsu: Using native `jj workspace` commands

**Testing:**
- ✅ 23 comprehensive Multi-VCS tests (all passing)
- ✅ VcsBackend trait fully exercised
- ✅ Factory functions tested
- ✅ Mercurial backend end-to-end
- ✅ Jujutsu backend end-to-end
- ✅ VCS detection and parsing

### Phase 4: Multi-VCS CLI Integration - ✅ COMPLETED

**Implementation:**
- ✅ Global `--vcs` flag added to main CLI
- ✅ All 11 commands accept optional VCS type parameter
- ✅ Backend initialization with auto-detection or explicit selection
- ✅ Integration across all command modules (add, list, remove, switch, info, integrate, sync, return, each, prune, cleanup)

**Production Status:**
- ✅ Git backend: Fully supported, production-ready
- ✅ Mercurial backend: Fully supported, production-ready
- ✅ Jujutsu backend: Fully supported, production-ready

**Testing:**
- ✅ All 186 tests passing
- ✅ Multi-VCS integration verified end-to-end
- ✅ Cross-VCS compatibility validated

---

## What's Next: Future Development

### v0.2: Monorepo & Advanced Features (Future)
- ⏸️ Sparse checkout for large monorepos
- ⏸️ Config hierarchy (multi-level config merging)
- ⏸️ Advanced hooks with conditions
- ⏸️ Performance optimizations for 100+ worktrees

See `vision.md` for full long-term roadmap.

---

## Development Workflow

### Daily Workflow
1. Pick a task from phase
2. Write test first (TDD)
3. Implement feature
4. Run tests: `cargo test`
5. Run clippy: `cargo clippy`
6. Commit with clear message

### Before Merging
- All tests pass
- No clippy warnings
- Code formatted: `cargo fmt`
- Documentation updated

### Testing Strategy
- Unit tests: Individual functions
- Integration tests: End-to-end workflows
- Concurrency tests: Stress test with threads
- Manual testing: Real repos, real workflows

---

## Key Principles

### Keep It Simple
- Don't add features until proven necessary
- Default to isolated resources (safer)
- Use git CLI when libgit2 is complex
- One config file is enough

### Ship Fast, Iterate
- MVP in 6 weeks, not 6 months
- Get feedback from real users
- Add complexity only when needed
- Defer edge cases

### Safety First
- Validate all inputs
- Lock shared state
- Atomic operations
- Graceful errors
- No data loss

### Test Everything
- Unit tests for logic
- Integration tests for workflows
- Concurrency tests for race conditions
- Don't ship without tests

---

## Questions & Decisions

### Open Questions
1. Should we use libgit2 or shell out to git CLI?
   - **Decision:** libgit2 for core operations (faster, safer)
   - Shell out for complex operations if needed

2. How do we handle worktree deletion with uncommitted changes?
   - **Decision:** Warn and refuse (unless --force)
   - Print uncommitted file list

3. Should hooks run in background or foreground?
   - **Decision:** Foreground (simpler, user sees output)
   - Can add background option in v0.2

4. How do we detect "current worktree"?
   - **Decision:** Check if cwd is inside a worktree path
   - Fall back to git rev-parse --show-toplevel

### Resolved Decisions
- ✅ Git-only for MVP (defer Hg/Jj)
- ✅ No Docker in MVP (add in v0.2)
- ✅ One config file (defer hierarchy)
- ✅ Linux/macOS only (no Windows native)
- ✅ File locking for concurrency
- ✅ Isolated-by-default for dependencies

---

## Timeline Estimate

**Optimistic:** 4 weeks
**Realistic:** 6 weeks
**Pessimistic:** 8 weeks

**Critical path:**
- Week 1-2: Git integration + worktree model
- Week 2-3: Environment setup + config
- Week 3-4: Delete/switch/info + hooks
- Week 4-5: Concurrency safety
- Week 5-6: Testing + documentation + polish

**Parallel work:**
- Documentation can be written alongside implementation
- Tests should be written before/during implementation
- Shell wrapper can be developed anytime

---

**Ready to build!** 🚀

Start with Phase 0: Set up the Cargo project and CLI framework.
