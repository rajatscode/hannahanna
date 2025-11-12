# Migration Guides

Hannahanna migration guides for major version upgrades.

## Table of Contents

- [v0.4 → v0.5 (BREAKING CHANGES)](#migrating-to-v05)
- [v0.3 → v0.4](#migrating-to-v04)

---

# Migrating to v0.5

**⚠️ BREAKING CHANGES**: v0.5 contains breaking changes to environment variable names.

## TL;DR - Quick Migration

**Breaking Change**: Environment variables renamed from `WT_*` to `HNHN_*`.

```bash
# Update hooks in .hannahanna.yml
sed -i 's/WT_NAME/HNHN_NAME/g' .hannahanna.yml
sed -i 's/WT_PATH/HNHN_PATH/g' .hannahanna.yml
sed -i 's/WT_BRANCH/HNHN_BRANCH/g' .hannahanna.yml
sed -i 's/WT_COMMIT/HNHN_COMMIT/g' .hannahanna.yml
sed -i 's/WT_STATE_DIR/HNHN_STATE_DIR/g' .hannahanna.yml
sed -i 's/WT_DOCKER_PORT_/HNHN_DOCKER_PORT_/g' .hannahanna.yml

# Update and verify
cargo install --git https://github.com/yourusername/hannahanna
hn --version  # Should show 0.5.0

# Test hooks still work
hn add test-migration
hn remove test-migration --force
```

---

## What's New in v0.5

### 1. 🏷️ Environment Variable Rename (BREAKING)

**What Changed**: All environment variables renamed for clarity and consistency.

| v0.4 (Old)              | v0.5 (New)              | Description                    |
|-------------------------|-------------------------|--------------------------------|
| `WT_NAME`               | `HNHN_NAME`             | Worktree name                  |
| `WT_PATH`               | `HNHN_PATH`             | Worktree absolute path         |
| `WT_BRANCH`             | `HNHN_BRANCH`           | Git branch name                |
| `WT_COMMIT`             | `HNHN_COMMIT`           | Current commit hash            |
| `WT_STATE_DIR`          | `HNHN_STATE_DIR`        | Worktree state directory       |
| `WT_DOCKER_PORT_<SVC>`  | `HNHN_DOCKER_PORT_<SVC>`| Docker port for service        |

**Why**: `HNHN` (Hannahanna) is more specific than `WT` (worktree), reducing conflicts with other tools.

**Impact**: Any hooks using environment variables must be updated.

**Migration**: Update your `.hannahanna.yml`:

**Before (v0.4)**:
```yaml
hooks:
  post_create: |
    echo "Created worktree: $WT_NAME"
    echo "Path: $WT_PATH"
    echo "Branch: $WT_BRANCH"
    docker-compose -p $WT_NAME up -d
```

**After (v0.5)**:
```yaml
hooks:
  post_create: |
    echo "Created worktree: $HNHN_NAME"
    echo "Path: $HNHN_PATH"
    echo "Branch: $HNHN_BRANCH"
    docker-compose -p $HNHN_NAME up -d
```

### 2. 📋 Template Management (NEW)

**What**: Create, manage, and use worktree templates.

```bash
# Create template
hn templates create frontend-dev --docker

# List templates
hn templates list

# Show template
hn templates show frontend-dev

# Use template
hn add my-feature --template frontend-dev
```

**Features**:
- Store templates in `.hn-templates/`
- Templates include config, hooks, and files
- Variable substitution in template files: `${HNHN_NAME}`, `${HNHN_PATH}`, `${HNHN_BRANCH}`
- Share templates across team via git

**Documentation**: See [templates.md](./templates.md)

### 3. 💾 Workspace Save/Restore (NEW)

**What**: Save and restore sets of worktrees as workspaces.

```bash
# Save current worktrees as workspace
hn workspace save my-workspace

# List saved workspaces
hn workspace list

# Restore workspace
hn workspace restore my-workspace

# Delete workspace
hn workspace delete my-workspace --force

# Export workspace
hn workspace export my-workspace
```

**Use Cases**:
- Switch between projects with different worktree sets
- Share team configurations
- Backup worktree state before experiments

### 4. 📊 Resource Tracking (NEW)

**What**: Track disk usage and resource consumption.

```bash
# Show stats for all worktrees
hn stats

# Show stats for specific worktree
hn stats my-feature

# Show only disk usage
hn stats --disk

# Include main repo in stats
hn stats --all
```

---

## Migration Checklist

### ✅ Step 1: Audit Hook Usage

Find all hooks using old environment variables:

```bash
# Search for old variable names
grep -r "WT_" .hannahanna.yml .hn-templates/

# Also check template files
grep -r "WT_" .hn-templates/*/files/
```

### ✅ Step 2: Update Configuration Files

**Main Config** (`.hannahanna.yml`):
```bash
# Automated update (backup first!)
cp .hannahanna.yml .hannahanna.yml.backup

# Replace all occurrences
sed -i 's/\$WT_NAME/\$HNHN_NAME/g' .hannahanna.yml
sed -i 's/\${WT_NAME}/\${HNHN_NAME}/g' .hannahanna.yml
sed -i 's/\$WT_PATH/\$HNHN_PATH/g' .hannahanna.yml
sed -i 's/\${WT_PATH}/\${HNHN_PATH}/g' .hannahanna.yml
sed -i 's/\$WT_BRANCH/\$HNHN_BRANCH/g' .hannahanna.yml
sed -i 's/\${WT_BRANCH}/\${HNHN_BRANCH}/g' .hannahanna.yml
sed -i 's/\$WT_COMMIT/\$HNHN_COMMIT/g' .hannahanna.yml
sed -i 's/\${WT_COMMIT}/\${HNHN_COMMIT}/g' .hannahanna.yml
sed -i 's/\$WT_STATE_DIR/\$HNHN_STATE_DIR/g' .hannahanna.yml
sed -i 's/\${WT_STATE_DIR}/\${HNHN_STATE_DIR}/g' .hannahanna.yml
sed -i 's/WT_DOCKER_PORT_/HNHN_DOCKER_PORT_/g' .hannahanna.yml

# Review changes
diff .hannahanna.yml.backup .hannahanna.yml
```

**Templates**:
```bash
# Update all templates
for template in .hn-templates/*/; do
  if [ -f "$template/.hannahanna.yml" ]; then
    echo "Updating $template"
    sed -i 's/\$WT_NAME/\$HNHN_NAME/g' "$template/.hannahanna.yml"
    sed -i 's/\${WT_NAME}/\${HNHN_NAME}/g' "$template/.hannahanna.yml"
    sed -i 's/\$WT_PATH/\$HNHN_PATH/g' "$template/.hannahanna.yml"
    sed -i 's/\${WT_PATH}/\${HNHN_PATH}/g' "$template/.hannahanna.yml"
    sed -i 's/\$WT_BRANCH/\$HNHN_BRANCH/g' "$template/.hannahanna.yml"
    sed -i 's/\${WT_BRANCH}/\${HNHN_BRANCH}/g' "$template/.hannahanna.yml"
    sed -i 's/WT_DOCKER_PORT_/HNHN_DOCKER_PORT_/g' "$template/.hannahanna.yml"
  fi

  # Update template files
  find "$template/files" -type f 2>/dev/null | while read file; do
    if file "$file" | grep -q text; then
      sed -i 's/\${WT_NAME}/\${HNHN_NAME}/g' "$file"
      sed -i 's/\${WT_PATH}/\${HNHN_PATH}/g' "$file"
      sed -i 's/\${WT_BRANCH}/\${HNHN_BRANCH}/g' "$file"
    fi
  done
done
```

### ✅ Step 3: Update Hannahanna

```bash
# Update installation
cargo install --force --git https://github.com/yourusername/hannahanna

# Verify version
hn --version
# hannahanna 0.5.0
```

### ✅ Step 4: Test Migration

Create a test worktree to verify hooks work:

```bash
# Create test worktree
hn add migration-test

# Check hooks executed correctly
# Review output for any errors

# Verify environment variables were set correctly
cd migration-test
# Check any files created by hooks

# Clean up
hn remove migration-test --force
```

### ✅ Step 5: Update Team Documentation

If you maintain team docs:

```bash
# Update any documentation referencing old variables
find docs/ wiki/ -name "*.md" -exec sed -i 's/WT_NAME/HNHN_NAME/g' {} \;
find docs/ wiki/ -name "*.md" -exec sed -i 's/WT_PATH/HNHN_PATH/g' {} \;
# etc.

# Notify team of breaking changes
```

---

## Common Migration Issues

### Issue 1: Hooks Failing After Upgrade

**Symptom**: Hooks that worked in v0.4 now fail or behave incorrectly.

**Cause**: Still using old `WT_*` variable names.

**Solution**:
```bash
# Check hook output
hn add test --verbose

# Look for empty variables or errors like:
# "WT_NAME: unbound variable"

# Fix: Update .hannahanna.yml as shown in Step 2
```

### Issue 2: Template Files Not Substituting

**Symptom**: Template files still contain `${WT_NAME}` after creation.

**Cause**: Template files not updated to use new variables.

**Solution**:
```bash
# Update template files
find .hn-templates/*/files -type f | while read file; do
  if file "$file" | grep -q text; then
    sed -i 's/\${WT_NAME}/\${HNHN_NAME}/g' "$file"
    sed -i 's/\${WT_PATH}/\${HNHN_PATH}/g' "$file"
    sed -i 's/\${WT_BRANCH}/\${HNHN_BRANCH}/g' "$file"
  fi
done
```

### Issue 3: Docker Environment Variables

**Symptom**: Docker containers can't access port numbers.

**Cause**: `WT_DOCKER_PORT_*` → `HNHN_DOCKER_PORT_*` rename.

**Solution**:
```bash
# Update docker-compose.yml or hooks
sed -i 's/WT_DOCKER_PORT_/HNHN_DOCKER_PORT_/g' .hannahanna.yml

# Also update any docker-compose files
sed -i 's/\${WT_DOCKER_PORT_/\${HNHN_DOCKER_PORT_/g' docker-compose*.yml
```

---

## New Features Guide

### Using Templates

**Create your first template**:
```bash
# Create template directory
hn templates create my-app

# Customize template config
cat > .hn-templates/my-app/.hannahanna.yml <<'EOF'
hooks:
  post_create: |
    echo "Setting up ${HNHN_NAME}..."
    npm install
    cp .env.example .env

docker:
  enabled: true
  services:
    - app
  ports:
    app: auto
EOF

# Add template files
mkdir -p .hn-templates/my-app/files
echo "NODE_ENV=development" > .hn-templates/my-app/files/.env.example
echo "APP_NAME=\${HNHN_NAME}" >> .hn-templates/my-app/files/.env.example

# Use template
hn add feature-x --template my-app
```

**Result**: New worktree with config applied and files copied with variable substitution.

### Using Workspaces

**Save current work**:
```bash
# Create several worktrees for a project
hn add frontend
hn add backend
hn add database-migration

# Save as workspace
hn workspace save web-app

# Later, restore all at once
hn workspace restore web-app
```

### Using Stats

**Monitor resource usage**:
```bash
# See which worktrees use most space
hn stats --all

# Check specific worktree
hn stats frontend

# Quick disk check
hn stats --disk
```

---

## Rollback (If Needed)

If you encounter critical issues:

```bash
# Rollback to v0.4
cargo install --force --git https://github.com/yourusername/hannahanna --tag v0.4.0

# Verify
hn --version  # Should show 0.4.0

# Restore backup config
cp .hannahanna.yml.backup .hannahanna.yml
```

**Note**: v0.5 doesn't modify existing worktrees, only adds new features. Safe to rollback.

---

## FAQ

### Q: Can I use both old and new variable names?

**A**: No. v0.5 only supports `HNHN_*` variables. Old `WT_*` names are removed.

### Q: Will my existing worktrees break?

**A**: No. Existing worktrees work fine. Only hooks using old variable names need updates.

### Q: How do I test before fully migrating?

**A**:
1. Create backup: `cp .hannahanna.yml .hannahanna.yml.backup`
2. Update config as shown above
3. Test with: `hn add migration-test`
4. If issues: `cp .hannahanna.yml.backup .hannahanna.yml`

### Q: Do templates work with the new variables?

**A**: Yes! Templates fully support `${HNHN_NAME}`, `${HNHN_PATH}`, `${HNHN_BRANCH}` substitution.

### Q: Can I migrate gradually?

**A**: No. Once you upgrade to v0.5, all hooks must use new variable names. Migrate all configs before upgrading.

---

## Getting Help

- 📖 **Templates Guide**: [templates.md](./templates.md)
- 🐛 **Report Issues**: [GitHub Issues](https://github.com/yourusername/hannahanna/issues)
- 💬 **Questions**: [GitHub Discussions](https://github.com/yourusername/hannahanna/discussions)

---

# Migrating to v0.4

This guide helps you upgrade from v0.3 to v0.4.

## TL;DR

**No breaking changes!** v0.4 is fully backward compatible with v0.3.

Just update and enjoy:
- ⚡ 50%+ faster worktree listings (caching)
- 📊 Performance benchmarks
- 🔧 Shell completions & automated setup
- 📋 Template system
- ✨ Enhanced `hn info` output

```bash
# Update
cargo install --git https://github.com/yourusername/hannahanna

# Verify
hn --version  # Should show 0.4.0

# Run setup (recommended)
hn setup

# Done!
```

---

## What's New in v0.4

### 1. ⚡ Performance: Registry Caching

**What**: Worktree listings are now cached for 30 seconds, dramatically improving `hn list` performance.

**Automatic**: No configuration needed. The cache works transparently.

**Benefits**:
- `hn list` is 50%+ faster on cache hits
- Especially noticeable with 10+ worktrees
- Cache auto-invalidates on add/remove operations

**New Commands**:
```bash
# View cache statistics
hn state cache stats

# Clear cache manually (rarely needed)
hn state cache clear
```

**Example**:
```bash
# First run - queries VCS (slower)
time hn list
# 0.08s

# Second run - uses cache (faster!)
time hn list
# 0.03s  (~60% faster!)
```

### 2. 📊 Performance Benchmarks

**What**: Comprehensive benchmark suite to measure and track performance.

**Usage**:
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench list_worktrees

# Save baseline
cargo bench -- --save-baseline v0.4.0

# Compare against baseline
cargo bench -- --baseline v0.4.0
```

**Targets**:
- List 100 worktrees: < 100ms
- Create worktree: < 500ms
- Fuzzy search 1000 items: < 10ms
- Port allocation (10 concurrent): < 2s
- Config load: < 50ms

**Documentation**: See [BENCHMARKS.md](../BENCHMARKS.md)

### 3. 🔧 Shell Completions & Automated Setup

**What**: Auto-completion for bash, zsh, and fish shells, plus automated setup.

**Quick Setup** (Recommended):
```bash
# One command to set up everything
hn setup

# Or specify shell
hn setup --shell bash
```

This installs completions, provides shell integration instructions, creates example templates, and validates your environment.

**Manual Setup** (Alternative):
```bash
# Bash
hn completions bash > ~/.local/share/bash-completion/completions/hn

# Zsh
hn completions zsh > ~/.zsh/completions/_hn

# Fish
hn completions fish > ~/.config/fish/completions/hn.fish
```

**Reload your shell**, then enjoy tab completion:
```bash
hn <TAB>        # Shows all commands
hn add <TAB>    # Completes options
hn switch <TAB> # Shows worktree names
```

### 4. ✨ Enhanced `hn info` Output

**What**: Much richer information display with colors, emojis, and actionable commands.

**New Fields**:
- ✓ Status emoji (✓ clean / ⚠ dirty)
- 📅 Age (time since creation)
- 💾 Disk usage
- 👪 Parent/children relationships with ages
- 🐳 Docker memory & CPU (if containers running)
- 🎯 Actions section with suggested commands

**Example**:
```bash
hn info feature-auth
```

**Old Output** (v0.3):
```
Worktree: feature-auth
Path: ../feature-auth
Branch: feature/authentication
Commit: abc1234

Status:
  Modified: 2 files

Docker:
  Ports:
    app: 3010
  Containers: Running
```

**New Output** (v0.4):
```
Worktree: feature-auth
============================================================
Path: ../feature-auth
Branch: feature/authentication
Commit: abc1234
Status: ⚠ 2 modified, 1 untracked
Age: 3 days ago (created 2024-11-09 14:32)
Disk: 245.67 MB

Parent: main
Children:
  - feature-oauth (created 2 days ago)
  - fix-token-refresh (created 5 hours ago)

Docker:
  Ports: app:3010, postgres:5442
  Containers: Running
    Memory: 156.2 MiB / 2 GiB
    CPU: 2.34%

Actions:
  → hn switch feature-auth
  → hn integrate feature-auth main
  → hn docker logs feature-auth
  → hn remove feature-auth
```

### 5. 📋 Template System

**What**: Pre-configured environment setups for different worktree types.

**Usage**:
```bash
# Create templates in your repo
mkdir -p .hn-templates/microservice
cat > .hn-templates/microservice/.hannahanna.yml <<EOF
docker:
  enabled: true
  ports:
    base:
      app: 3000
      db: 5432
hooks:
  post_create: |
    npm install
    npm run db:migrate
EOF

# Use template when creating worktrees
hn add my-service --template microservice
```

**Features**:
- Templates stored in `.hn-templates/<template-name>/`
- Each template has `.hannahanna.yml` with config overrides
- Applied to worktree's `.hannahanna.local.yml`
- Great for standardizing environments (microservices, frontends, etc.)

**Example templates are created by `hn setup`** in `.hn-templates/microservice/` and `.hn-templates/frontend/`.

---

## Migration Checklist

### ✅ Step 1: Update Installation

```bash
# Via cargo
cargo install --force --git https://github.com/yourusername/hannahanna

# Or download binary (if available)
# wget https://github.com/yourusername/hannahanna/releases/download/v0.4.0/hn
# chmod +x hn
# mv hn /usr/local/bin/
```

### ✅ Step 2: Verify Version

```bash
hn --version
# hannahanna 0.4.0
```

### ✅ Step 3: Set Up Shell Completions (Optional)

```bash
# Bash
hn completions bash > ~/.local/share/bash-completion/completions/hn
source ~/.bashrc

# Zsh
hn completions zsh > ~/.zsh/completions/_hn
# Add to ~/.zshrc: fpath=(~/.zsh/completions $fpath)
source ~/.zshrc

# Fish
hn completions fish > ~/.config/fish/completions/hn.fish
```

### ✅ Step 4: Test Cache (Optional)

```bash
# Check cache is working
hn list
hn state cache stats

# Should show:
# Status: Valid
# Age: 0.1s
# Worktrees: X
# Size: Y bytes
```

### ✅ Step 5: Run Benchmarks (Optional)

```bash
cargo bench

# Review results to establish your baseline
```

---

## Configuration Changes

### No Changes Required ✓

Your existing `.hannahanna.yml` files work as-is. No updates needed.

### Optional: Cache TTL Configuration

If you want to customize cache behavior, you can (though defaults work well):

```yaml
# .hannahanna.yml
# Note: This is NOT officially supported yet, but planned for future

# For now, cache is hardcoded to 30 seconds
# Future: configurable cache settings
```

---

## Command Changes

### New Commands

```bash
hn completions <shell>      # Generate shell completions
hn state cache stats        # View cache statistics
hn state cache clear        # Clear cache
```

### Enhanced Commands

```bash
hn info <name>              # Now shows much more detail
```

### Unchanged Commands

All existing commands work exactly as before:
- `hn add`
- `hn list`
- `hn switch`
- `hn remove`
- `hn integrate`
- `hn sync`
- `hn each`
- `hn docker *`
- `hn ports *`
- `hn state list/clean/size`
- `hn config *`
- etc.

---

## Performance Improvements

### Registry Caching

**Impact**: `hn list` is 50%+ faster on cache hits.

**What Changed**:
- First `hn list` call builds cache
- Subsequent calls use cache (30s TTL)
- Cache automatically invalidates on `hn add` / `hn remove`

**No Action Needed**: Works automatically.

**If You Notice Issues**:
```bash
# Clear cache manually
hn state cache clear

# Check if caching is working
hn state cache stats
```

### Parallel Execution

**Note**: Already existed in v0.3! Now documented better.

```bash
# Run command in all worktrees simultaneously
hn each --parallel "npm test"

# Faster than sequential for I/O-bound operations
```

---

## Troubleshooting

### Cache Not Working?

```bash
# Check cache status
hn state cache stats

# If shows "No cache found":
hn list  # Builds cache
hn state cache stats  # Should now show valid cache

# If cache seems stale:
hn state cache clear
hn list  # Rebuilds
```

### Completions Not Working?

**Bash**:
```bash
# Ensure bash-completion is installed
which bash-completion

# Ensure completions directory exists
ls ~/.local/share/bash-completion/completions/

# Reload shell
source ~/.bashrc
```

**Zsh**:
```bash
# Ensure completions directory in fpath
echo $fpath | grep .zsh/completions

# Add to ~/.zshrc if missing:
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit

# Reload
source ~/.zshrc
```

**Fish**:
```bash
# Check completions directory
ls ~/.config/fish/completions/

# Should see hn.fish
# Completions load automatically in fish
```

### Performance Issues?

```bash
# Run benchmarks to identify bottlenecks
cargo bench

# Compare against targets in BENCHMARKS.md

# If significantly slower:
# 1. Check cache is working (hn state cache stats)
# 2. Check disk I/O (slow filesystem?)
# 3. Check repo size (very large repos?)
# 4. Check worktree count (100+ worktrees?)

# Report performance issues with:
cargo bench > benchmark_results.txt
# Include in issue report
```

---

## Rollback (If Needed)

If you encounter issues, rolling back is safe:

```bash
# Reinstall v0.3
cargo install --force --git https://github.com/yourusername/hannahanna --tag v0.3.0

# Or use previous binary
cp ~/backups/hn-v0.3 /usr/local/bin/hn

# Verify
hn --version  # Should show 0.3.0
```

**Note**: v0.4 doesn't modify your worktrees or config. Only adds cache files in `.hn-state/.registry-cache`.

**To Clean Up**:
```bash
# Remove cache files (optional)
find . -name ".registry-cache" -delete
```

---

## FAQ

### Q: Do I need to rebuild my worktrees?

**A**: No! Existing worktrees work perfectly with v0.4.

### Q: Will caching cause stale data?

**A**: No. Cache auto-invalidates on add/remove. TTL is only 30s. Worst case: `hn state cache clear`.

### Q: Do benchmarks slow down normal usage?

**A**: No. Benchmarks are separate (`cargo bench`). Normal `hn` commands are unaffected.

### Q: Can I disable caching?

**A**: Not currently. Caching is transparent and has no downsides. Future versions may add a `--no-cache` flag if needed.

### Q: What if I find a bug?

**A**: Please report at: https://github.com/yourusername/hannahanna/issues

Include:
- `hn --version` output
- Steps to reproduce
- Expected vs actual behavior
- `hn state cache stats` output (if cache-related)

---

## What's Next?

Check out:
- [examples.md](examples.md) - Real-world workflows
- [BENCHMARKS.md](../BENCHMARKS.md) - Performance tuning
- [README.md](../README.md) - Full documentation
- [CHANGELOG.md](../CHANGELOG.md) - Detailed changes

---

## Feedback

We want to hear from you!

- 🐛 Bug reports: [GitHub Issues](https://github.com/yourusername/hannahanna/issues)
- 💡 Feature requests: [GitHub Discussions](https://github.com/yourusername/hannahanna/discussions)
- 📖 Doc improvements: [Pull Requests](https://github.com/yourusername/hannahanna/pulls)

Happy worktree-ing! 🚀
