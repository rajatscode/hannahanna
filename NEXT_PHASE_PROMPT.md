# hannahanna - Phase 2+ Implementation Prompt

## Current Status: v0.1.0 Complete ✅

The MVP is **complete and ready to ship** with 132 passing tests, full documentation, and all core features working. See `CHANGELOG.md` and `RELEASE_NOTES_v0.1.0.md` for details.

Branch: `claude/complete-hn-mvp-features-011CV2Q6ghwkTpLMc9tbvEjb` (ready to merge)

---

## Your Mission: Implement Remaining spec.md Features

The complete feature specification in `spec/spec.md` defines features beyond the MVP. Your task is to implement these remaining features to move toward v1.0.

### What's Already Built (v0.1.0)

**Core Commands:**
- ✅ `hn add/list/remove/switch/return/info/prune` - All working
- ✅ `hn each` - Batch operations with parallel execution
- ✅ `hn config` - Configuration management
- ✅ `hn docker` - Full Docker lifecycle (ps/start/stop/restart/logs/prune)
- ✅ `hn ports` - Port allocation management
- ✅ `hn init-shell` - Shell integration

**Infrastructure:**
- ✅ Git worktree management via git2-rs
- ✅ Parent/child relationship tracking
- ✅ Fuzzy matching
- ✅ Shared resources (symlinks with compatibility checking)
- ✅ Hooks system (post_create, pre_remove, post_switch)
- ✅ State management with file locking
- ✅ Error handling with helpful suggestions
- ✅ Comprehensive test suite (132 tests)

### What to Build Next (Priority Order)

Refer to `spec/spec.md` for complete specifications. Build these in order:

---

#### **PHASE 2: Integration Operations** (spec.md section 2)

**1. `hn integrate <source> [--into=<target>]`** (lines 161-184)
- Merge source worktree/branch into target
- Support `--no-ff`, `--squash`, `--strategy` options
- Check for uncommitted changes
- Handle merge conflicts gracefully

**2. `hn sync [source-branch]`** (lines 211-227)
- Sync current worktree with another branch (typically main)
- Support merge or rebase strategy (`--strategy=<merge|rebase>`)
- Handle conflicts, stash if needed (`--autostash`)

**Key Implementation Notes:**
- Build on existing git operations in `src/vcs/git.rs`
- Follow patterns from `hn return` command (src/cli/return_cmd.rs)
- Add comprehensive tests in `tests/integration/`

---

#### **PHASE 3: Multi-VCS Support** (spec.md section 6)

**Goal:** Support Mercurial (hg) and Jujutsu (jj) in addition to Git

**Architecture (lines 542-580):**
```
src/vcs/
├── mod.rs           # VCS trait definition
├── git.rs           # Git implementation (already exists)
├── hg.rs            # Mercurial implementation (NEW)
└── jj.rs            # Jujutsu implementation (NEW)
```

**VCS Trait Requirements:**
```rust
pub trait VCS {
    fn create_worktree(&self, name: &str, branch: Option<&str>) -> Result<Worktree>;
    fn list_worktrees(&self) -> Result<Vec<Worktree>>;
    fn remove_worktree(&self, name: &str) -> Result<()>;
    fn get_worktree_status(&self, name: &str) -> Result<WorktreeStatus>;
    // ... etc
}
```

**Detection Logic:**
- Auto-detect VCS from repository (check for .git, .hg, .jj)
- Allow override via config or CLI flag
- Graceful error if VCS not supported

**Testing:**
- Create test repositories for each VCS
- Ensure feature parity across all VCS implementations
- Add integration tests for multi-VCS scenarios

---

#### **PHASE 4: Sparse Checkout** (spec.md section 7, lines 582-607)

**For Monorepos:**

**Config Example:**
```yaml
sparse:
  enabled: true
  paths:
    - services/api/
    - services/web/
    - libs/shared/
```

**Behavior:**
- Only checkout specified paths in worktrees
- Main repo remains full checkout
- Automatic sparse-checkout config on worktree create
- Support path expansion and gitignore-style patterns

**Implementation:**
- Add sparse config to `src/config.rs`
- Integrate with worktree creation in `src/vcs/git.rs`
- Test with large monorepo fixture

---

#### **PHASE 5: Configuration Hierarchy** (spec.md section 8, lines 643-661)

**Three-Level Config System:**

1. **User Level** (`~/.config/hannahanna/config.yml`)
   - Global defaults for all repositories

2. **Repository Level** (`.hannahanna.yml`)
   - Project-specific settings (already implemented)

3. **Worktree Level** (`worktrees/<name>/.hannahanna.yml`)
   - Per-worktree overrides

**Merge Strategy:**
- User config ← Repo config ← Worktree config (rightmost wins)
- Deep merge for nested structures
- Allow per-section override vs merge

**Implementation:**
- Refactor `src/config.rs` to support hierarchy
- Add `Config::load_with_hierarchy()` method
- Add `hn config show --all-sources` to display merge result
- Test config precedence thoroughly

---

#### **PHASE 6: Advanced Features**

**1. Advanced Hooks** (spec.md section 7, lines 608-624)
```yaml
hooks:
  post_create:
    - condition: "has_docker_compose"
      command: "docker-compose up -d"
    - condition: "is_feature_branch"
      command: "npm install"
```

**2. `hn docker exec`** (spec.md lines 454)
```bash
hn docker exec <name> <command>
# Execute arbitrary command in container
```

**3. `hn ports reassign`** (spec.md line 485)
```bash
hn ports reassign <name>
# Allocate new ports for a worktree
```

**4. Team Coordination** (spec.md section 7, lines 625-641)
- Shared state for teams
- Worktree locking (prevent concurrent work)
- Status broadcasting
- Conflict detection

---

## Getting Started

### 1. **Read the Specs**
```bash
# Complete feature specification
cat spec/spec.md

# Long-term vision and architecture
cat spec/vision.md

# Original MVP plan (for context)
cat spec/plan.md
```

### 2. **Understand Current Implementation**
```bash
# Browse existing code
ls -R src/

# Run tests to ensure everything works
cargo test

# Check documentation
cat README.md
cat CHANGELOG.md
```

### 3. **Key Files to Study**

**Command Patterns:**
- `src/cli/add.rs` - Complex command with hooks
- `src/cli/return_cmd.rs` - Git merge operations
- `src/cli/each.rs` - Parallel execution
- `src/cli/docker.rs` - Subcommand structure

**Core Logic:**
- `src/vcs/git.rs` - VCS operations (extend for multi-VCS)
- `src/config.rs` - Configuration (extend for hierarchy)
- `src/state.rs` - State management
- `src/env/symlinks.rs` - Environment setup

**Testing:**
- `tests/common/mod.rs` - Test utilities
- `tests/worktree_lifecycle.rs` - Full lifecycle tests
- `tests/integration/` - Integration tests

### 4. **Development Workflow**

```bash
# Install git hooks
./scripts/install-git-hooks.sh

# Create feature branch
git checkout -b claude/implement-integrate-sync

# Run tests frequently
cargo test

# Check code quality
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings

# Build release
cargo build --release
```

### 5. **Testing Guidelines**

- Write tests FIRST (TDD approach)
- Aim for 85%+ coverage
- Use `tests/common/mod.rs` TestRepo helper
- Add integration tests for complex workflows
- Test error cases thoroughly

### 6. **Documentation Requirements**

For each new feature:
- [ ] Update `README.md` with command usage
- [ ] Add examples to documentation
- [ ] Update `CHANGELOG.md` with changes
- [ ] Add inline code documentation
- [ ] Update error message suggestions if needed

---

## Recommended Implementation Order

### Phase 2.1: `hn integrate` (1-2 days)
1. Study `src/cli/return_cmd.rs` (similar merge logic)
2. Add `src/cli/integrate.rs`
3. Wire up in `src/main.rs`
4. Add tests in `tests/integration/integrate_workflow.rs`
5. Update README

### Phase 2.2: `hn sync` (1 day)
1. Build on integrate infrastructure
2. Add rebase support
3. Add autostash logic
4. Test conflict scenarios

### Phase 3: Multi-VCS (3-5 days)
1. Design VCS trait (`src/vcs/mod.rs`)
2. Refactor existing Git code to implement trait
3. Add Mercurial support (`src/vcs/hg.rs`)
4. Add Jujutsu support (`src/vcs/jj.rs`)
5. Add VCS detection logic
6. Update all commands to use trait
7. Comprehensive testing

### Phase 4: Sparse Checkout (2-3 days)
1. Extend config structure
2. Add sparse-checkout logic to Git backend
3. Update worktree creation flow
4. Test with monorepo fixtures

### Phase 5: Config Hierarchy (2-3 days)
1. Refactor config loading
2. Implement merge logic
3. Add user config location
4. Update all config consumers
5. Add `--all-sources` flag

### Phase 6: Advanced Features (1-2 weeks)
- Implement incrementally based on priority
- Each feature can be a separate PR/branch

---

## Architecture Principles

1. **Follow Existing Patterns**
   - Commands in `src/cli/<name>.rs`
   - Core logic in dedicated modules
   - Comprehensive error handling
   - Tests parallel to implementation

2. **Maintain Quality**
   - 85%+ test coverage
   - Zero clippy warnings
   - Formatted with rustfmt
   - Clear error messages

3. **Security First**
   - No command injection (use Command::new, not shell)
   - Input validation
   - Path traversal prevention
   - Safe file operations

4. **Performance Targets**
   - Commands complete in < 500ms (except long operations)
   - Efficient file I/O
   - Parallel execution where appropriate

---

## Questions to Clarify Before Starting

1. **Multi-VCS Priority**: Which VCS to support first? (Suggest: Mercurial, then Jujutsu)
2. **Config Hierarchy**: Should worktree-level config be automatic or opt-in?
3. **Sparse Checkout**: Only Git or all VCS?
4. **Team Features**: What's the priority for coordination features?

---

## Success Criteria

For each phase:
- [ ] Feature implemented per spec.md
- [ ] Tests written and passing (85%+ coverage)
- [ ] Documentation updated
- [ ] No clippy warnings
- [ ] Code reviewed (if applicable)
- [ ] CHANGELOG updated

---

## Resources

- **Spec**: `spec/spec.md` (authoritative feature list)
- **Vision**: `spec/vision.md` (architecture and design)
- **Plan**: `spec/plan.md` (MVP scope, for context)
- **Code**: `src/` (current implementation)
- **Tests**: `tests/` (test patterns)

---

## Final Notes

The codebase is well-structured, thoroughly tested, and ready for extension. All the hard infrastructure work (state management, hooks, Docker, testing framework) is done. You're building on a solid foundation.

Focus on one phase at a time, write tests first, and maintain the quality bar. The architecture supports all these features naturally - no major refactoring needed.

**Good luck! 🚀**
