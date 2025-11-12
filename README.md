# hannahanna (hn)

A Git worktree manager that enables true parallel development with isolated environments.

## What is hannahanna?

`hannahanna` (command: `hn`) is a tool that makes it easy to work on multiple Git branches simultaneously. Each worktree gets its own complete, isolated development environment while intelligently sharing resources when safe.

**Why hannahanna?**
- Work on multiple features in parallel without stashing or context switching
- Create isolated environments for code reviews, experiments, or hotfixes
- Automatically share dependencies like `node_modules` when compatible
- Track parent/child relationships between worktrees for nested workflows
- Execute hooks on worktree creation/deletion for automated setup

## Installation

```bash
cargo install hannahanna
```

Or build from source:

```bash
git clone https://github.com/yourusername/hannahanna
cd hannahanna
cargo build --release
sudo cp target/release/hn /usr/local/bin/
```

### Quick Setup (Recommended)

After installation, run the automated setup:

```bash
hn setup
```

This command will:
- Install shell completions for your shell (bash/zsh/fish)
- Provide instructions for shell integration setup
- Create example templates (`.hn-templates/`)
- Validate your environment (git, docker)

The setup command auto-detects your shell, or you can specify:

```bash
hn setup --shell bash   # or zsh, fish
```

### Manual Shell Integration

Alternatively, for the `hn switch` command to work (changes directory), add this to your `~/.bashrc` or `~/.zshrc`:

```bash
eval "$(hn init-shell)"
```

## Quick Start

```bash
# Create worktrees for multiple features
hn add feature-auth
hn add feature-billing
hn add feature-dashboard

# List all worktrees
hn list

# Switch between worktrees
hn switch feature-auth

# Show detailed info
hn info feature-auth

# Remove when done
hn remove feature-auth
```

## Commands

### `hn add <name> [options]`

Create a new worktree.

```bash
# Create from current branch
hn add feature-x

# Create from specific branch
hn add hotfix-123 --from main

# Create without new branch (checkout existing)
hn add review-pr --no-branch
```

**Options:**
- `--from <branch>` - Base branch (default: current branch)
- `--no-branch` - Checkout existing branch instead of creating new one
- `--sparse <path>` - Sparse checkout paths (repeatable, v0.2+)
- `--no-hooks` - Skip hook execution (for untrusted repositories)

**Sparse Checkout (v0.2+):**
```bash
# Monorepo: only checkout specific paths
hn add feature-api --sparse services/api/ --sparse libs/utils/

# Configure default sparse paths in .hannahanna.yml
sparse:
  enabled: true
  paths:
    - services/api/
    - libs/shared/
```

### `hn list [options]`

List all worktrees.

```bash
# Simple list
hn list

# Tree view showing parent/child relationships
hn list --tree
```

**Output shows:**
- Worktree name
- Branch name
- Current commit (short hash)
- `*` marker for current worktree

### `hn switch <name>`

Switch to a worktree (requires shell integration).

```bash
hn switch feature-x
```

Supports fuzzy matching:
```bash
hn switch feat     # Matches "feature-x" if unique
```

### `hn return [options]`

Return to parent worktree with optional merge (requires shell integration).

```bash
# Switch back to parent worktree
hn return

# Merge current branch into parent before returning
hn return --merge

# Merge, return, and delete current worktree
hn return --merge --delete

# Force merge commit (no fast-forward)
hn return --merge --no-ff
```

**Perfect for nested workflows:**
```bash
hn add feature-payment
hn add fix-validation-bug    # Child of feature-payment
# ... fix bug, commit ...
hn return --merge             # Merge into feature-payment
# ... continue feature work
```

**Options:**
- `--merge` - Merge current branch into parent before returning
- `--delete` - Delete current worktree after merging (requires `--merge`)
- `--no-ff` - Force merge commit (no fast-forward)

### `hn each <command> [options]`

Execute a command in all worktrees.

```bash
# Run tests in all worktrees
hn each cargo test

# Run command in parallel
hn each --parallel npm install

# Only run on feature worktrees
hn each --filter="^feature" git status

# Stop on first failure
hn each --stop-on-error cargo check

# Only run on worktrees with Docker running
hn each --docker-running docker-compose ps
```

**Options:**
- `--parallel` - Execute commands concurrently across all worktrees
- `--stop-on-error` - Stop on first command failure (default: continue)
- `--filter=<pattern>` - Filter worktrees by name using regex
- `--docker-running` - Only run on worktrees with active Docker containers

**Perfect for:**
- Running tests across all features simultaneously
- Updating dependencies in all worktrees
- Checking git status or running git commands everywhere
- Batch operations on filtered subsets of worktrees

### `hn integrate <source> [options]`

Merge a source worktree or branch into a target worktree.

```bash
# Merge a worktree into current worktree
hn integrate feature-x

# Merge a worktree into a specific target
hn integrate feature-x --into main

# Merge a branch (not a worktree) into current worktree
hn integrate develop

# Force merge commit (no fast-forward)
hn integrate feature-x --no-ff

# Squash commits before merging
hn integrate feature-x --squash

# Use specific merge strategy
hn integrate feature-x --strategy=recursive
```

**Options:**
- `--into=<target>` - Target worktree name (defaults to current worktree)
- `--no-ff` - Force merge commit (no fast-forward)
- `--squash` - Squash commits before merging
- `--strategy=<strategy>` - Git merge strategy (e.g., 'recursive', 'ours', 'theirs')

**Examples:**
```bash
# Work on a feature, then merge it into main worktree
hn add feature-auth
cd ../feature-auth
# ... make changes, commit ...
hn integrate feature-auth --into main

# Integrate changes from main into current feature branch
hn integrate main
```

**Note:** Target worktree must have no uncommitted changes. Supports fuzzy matching for worktree names.

### `hn sync [source-branch] [options]`

Sync current worktree with another branch (typically main).

```bash
# Sync with main branch (default)
hn sync

# Sync with specific branch
hn sync develop

# Use rebase instead of merge
hn sync --strategy=rebase

# Automatically stash/unstash uncommitted changes
hn sync --autostash

# Merge without auto-committing
hn sync --no-commit
```

**Options:**
- `source-branch` - Branch to sync with (defaults to 'main')
- `--strategy=<merge|rebase>` - Sync strategy (defaults to 'merge')
- `--autostash` - Automatically stash uncommitted changes before sync
- `--no-commit` - Don't automatically commit after merge

**Examples:**
```bash
# Keep feature branch up to date with main
hn add feature-dashboard
cd ../feature-dashboard
# ... work on feature ...
hn sync  # Pull latest from main

# Use rebase for cleaner history
hn sync --strategy=rebase

# Sync with uncommitted changes
hn sync --autostash
```

**Perfect for:**
- Keeping feature branches up to date with main
- Pulling in latest changes from develop branch
- Resolving conflicts with upstream changes

### `hn info [name]`

Show detailed information about a worktree.

```bash
# Info for current worktree
hn info

# Info for specific worktree
hn info feature-x
```

**Displays:**
- Path, branch, commit
- Parent/child worktrees
- Git status summary
- Shared resources (symlinks/copies)

### `hn remove <name> [options]`

Remove a worktree.

```bash
# Remove worktree
hn remove feature-x

# Force removal (ignore uncommitted changes)
hn remove feature-x --force
```

**Options:**
- `--force` / `-f` - Force removal even if there are uncommitted changes
- `--no-hooks` - Skip hook execution (for untrusted repositories)

**Safety checks:**
- Warns about uncommitted changes (unless `--force`)
- Runs `pre_remove` hook if configured (unless `--no-hooks`)
- Cleans up state directories

### `hn prune`

Clean up orphaned state directories from deleted worktrees.

```bash
hn prune
```

### `hn setup [options]` (v0.4)

Automate hannahanna installation and shell integration.

```bash
# Auto-detect shell and run setup
hn setup

# Specify shell explicitly
hn setup --shell bash
hn setup --shell zsh
hn setup --shell fish
```

**What it does:**
1. **Shell Completions**: Installs completions to the appropriate location for your shell
2. **Shell Integration**: Provides instructions for adding the cd wrapper to your shell config
3. **Example Templates**: Creates `.hn-templates/` with microservice and frontend examples
4. **Environment Validation**: Checks for git and docker installation

**After running setup:**
1. Reload your shell: `source ~/.bashrc` (or `~/.zshrc`)
2. Try tab completion: `hn <TAB>`
3. Explore templates: `ls .hn-templates/`

**Options:**
- `--shell <bash|zsh|fish>` - Specify shell type (auto-detects if omitted)

This is the recommended way to set up hannahanna after installation.

### `hn completions <shell>` (v0.4)

Generate shell completions for auto-completion (manual alternative to `hn setup`).

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
# Fish auto-loads completions
```

After setup, enjoy tab completion:
```bash
hn <TAB>        # Shows all commands
hn add <TAB>    # Shows options
hn switch <TAB> # Shows worktree names
```

### `hn config <subcommand>`

Manage configuration files.

```bash
# Create a new configuration file with template
hn config init

# Validate configuration syntax
hn config validate

# Show current configuration
hn config show

# Edit configuration in $EDITOR
hn config edit
```

**Subcommands:**
- `init` - Create `.hannahanna.yml` with comprehensive template
- `validate` - Check configuration syntax and show summary
- `show` - Display current configuration as YAML
- `edit` - Open config in `$EDITOR` and validate after saving

### `hn docker <subcommand>`

Manage Docker containers for worktrees (requires Docker configuration).

```bash
# Show container status for all worktrees
hn docker ps

# Start containers for a worktree
hn docker start feature-x

# Stop containers for a worktree
hn docker stop feature-x

# Restart containers for a worktree
hn docker restart feature-x

# View logs from containers
hn docker logs feature-x
hn docker logs feature-x web  # Specific service

# Clean up orphaned containers
hn docker prune
```

**Subcommands:**
- `ps` - Show container status for all worktrees
- `start <name>` - Start Docker containers for a worktree
- `stop <name>` - Stop Docker containers for a worktree
- `restart <name>` - Restart Docker containers for a worktree
- `logs <name> [service]` - View logs (optionally for specific service)
- `prune` - Remove containers for deleted worktrees

**Features:**
- Automatic port allocation to avoid conflicts
- Isolated Docker Compose projects per worktree
- Health check monitoring
- Works with both `docker compose` and legacy `docker-compose`

### `hn ports <subcommand>`

Manage Docker port allocations.

```bash
# List all port allocations
hn ports list

# Show ports for a specific worktree
hn ports show feature-x

# Release ports for a worktree
hn ports release feature-x
```

**Subcommands:**
- `list` - Show all port allocations across worktrees
- `show <name>` - Show port allocations for a specific worktree
- `release <name>` - Manually release port allocations
- `reassign <name>` - Reassign ports to resolve conflicts (v0.3+)

### `hn state <subcommand>` (v0.3)

Manage worktree state directories.

```bash
# List all state directories with sizes
hn state list

# Check disk usage for specific worktree
hn state size feature-x

# Clean orphaned state directories
hn state clean
```

**Subcommands:**
- `list` - View all state dirs with sizes
- `size [name]` - Check disk usage for a specific worktree or all worktrees
- `clean` - Remove orphaned state directories
- `cache stats` - View registry cache statistics (v0.4)
- `cache clear` - Clear registry cache (v0.4)

### Command Aliases (v0.3)

Define custom command aliases in `.hannahanna.yml`:

```yaml
aliases:
  sw: switch
  ls: list
  lt: list --tree
  s: sw  # Chained aliases supported
```

**Features:**
- Cycle detection prevents infinite loops
- Chained aliases supported (s → sw → switch)
- Aliases work with arguments: `hn sw feature-x`
- Cannot override built-in commands

**Usage:**
```bash
# Define in .hannahanna.yml
aliases:
  sw: switch
  lt: list --tree

# Use them
hn sw feature-1
hn lt
```

## Configuration

Create a `.hannahanna.yml` file in your repository root:

```yaml
# Shared resources (symlinked from main repo)
shared_resources:
  - source: node_modules
    target: node_modules
    compatibility: package-lock.json
  - source: vendor
    target: vendor
    compatibility: composer.lock

# File copying (for templates/config files)
shared:
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
    echo "Cleaning up..."
    make cleanup
```

### Shared Resources

**Symlinks with Compatibility Checking:**

```yaml
shared_resources:
  - source: node_modules
    target: node_modules
    compatibility: package-lock.json
```

When `compatibility` is specified, hannahanna compares the lockfile between the main repo and worktree:
- **Compatible** (identical lockfiles) → Create symlink to save disk space
- **Incompatible** (different lockfiles) → Skip symlink, worktree gets isolated copy

**File Copying:**

```yaml
shared:
  copy:
    - .env.template -> .env
    - config/database.yml.example -> config/database.yml
```

Files are copied once during worktree creation, perfect for:
- Environment config files (`.env`)
- Local configuration templates
- Files that should exist but not be symlinked

### Hooks

Execute commands at specific lifecycle events:

```yaml
hooks:
  post_create: "npm install && npm run setup"
  pre_remove: "npm run cleanup"
  timeout_seconds: 300  # 5 minutes (default)
```

**⚠️ SECURITY WARNING:** Hooks execute arbitrary shell commands from your `.hannahanna.yml` configuration file. Only use hannahanna in repositories you trust. Never clone and run `hn add` in untrusted repositories without first reviewing the `.hannahanna.yml` file for malicious hooks.

**Security Feature:** Use the `--no-hooks` flag to disable hook execution when working with untrusted repositories:
```bash
hn add feature-x --no-hooks    # Skip post_create hook
hn remove feature-x --no-hooks # Skip pre_remove hook
```

**Available hooks:**
- `pre_create` - Runs before worktree creation (v0.3+)
- `post_create` - Runs after worktree creation
- `post_switch` - Runs after switching to a worktree (v0.3+)
- `pre_remove` - Runs before worktree deletion
- `post_remove` - Runs after worktree deletion (v0.3+)
- `pre_integrate` - Runs before merging (v0.3+)
- `post_integrate` - Runs after merging (v0.3+)

All hooks support conditional execution via branch patterns (v0.3+)

**Environment variables available in hooks:**
- `$WT_NAME` - Worktree name
- `$WT_PATH` - Worktree path
- `$WT_BRANCH` - Branch name
- `$WT_PARENT` - Parent worktree (if any)
- `$WT_STATE_DIR` - State directory path

**Hook Timeout:**
Hooks automatically timeout after 5 minutes (300 seconds) by default to prevent hanging processes. You can customize this in your config:

```yaml
hooks:
  timeout_seconds: 600  # 10 minutes
```

If a hook times out, the operation will fail with a clear error message. Use `--no-hooks` to skip hooks entirely.

## Use Cases

### Multiple Features in Parallel

```bash
hn add feature-auth
hn add feature-billing
hn add feature-dashboard

# Work on auth
hn switch feature-auth
# Make changes, commit

# Quick switch to billing
hn switch feature-billing
# Make changes, commit

# All worktrees running simultaneously with isolated node_modules (if incompatible)
```

### Hotfix During Feature Work

```bash
# Deep in feature work
hn add feature-redesign

# Urgent bug reported!
hn add hotfix-critical --from main

# Fix bug in hotfix worktree
hn switch hotfix-critical
# Fix, test, commit

# Merge to main
git checkout main
git merge hotfix-critical

# Clean up
hn remove hotfix-critical

# Back to feature work
hn switch feature-redesign
```

### Nested Worktrees

```bash
# Working on big feature
hn add feature-payment

# Discover bug while implementing
hn switch feature-payment
hn add fix-validation-bug  # Automatically tracks feature-payment as parent

# Fix bug
hn switch fix-validation-bug
# Fix, commit

# Merge back to parent
hn switch feature-payment
git merge fix-validation-bug

# View hierarchy
hn list --tree
# main
# └── feature-payment
#     └── fix-validation-bug
```

### Code Review

```bash
# Review a pull request
git fetch origin pull/123/head:pr-123
hn add review-pr-123 --no-branch

# Test locally
hn switch review-pr-123
npm start
# Browse to localhost:3000

# Done reviewing
hn remove review-pr-123
```

## Advanced Features

### Helpful Error Messages

hannahanna provides context-aware error messages with actionable suggestions:

```bash
$ hn add feature-x
Error: Worktree 'feature-x' already exists

Suggestions:
  • Remove existing: hn remove feature-x
  • Use different name: hn add feature-x-v2
  • Switch to existing: hn switch feature-x
```

Error suggestions cover common scenarios:
- Worktree already exists → Remove, rename, or switch options
- Worktree not found → List all, check spelling, create new
- Uncommitted changes → Commit, stash, or force remove
- No parent → Explains parent tracking, suggests alternatives
- Port conflicts → Port management commands
- Docker issues → Installation and permission fixes

### Fuzzy Matching

Most commands support fuzzy name matching:

```bash
hn switch feat      # → feature-auth (if unique match)
hn remove billing   # → feature-billing (if unique match)
hn info dash        # → feature-dashboard (if unique match)
```

Matching rules:
1. Exact match (case-sensitive) - highest priority
2. Exact match (case-insensitive)
3. Substring match (case-insensitive)
4. Error if ambiguous (multiple matches)

### Parent/Child Tracking

When you create a worktree from within another worktree, it's automatically tracked as a child:

```bash
hn add parent-feature
hn switch parent-feature
hn add child-feature  # Automatically becomes child of parent-feature

hn list --tree
# Shows parent/child relationships
```

### State Management

Each worktree gets a state directory (`.hn-state/<name>/`) for:
- Worktree-specific metadata
- Logs and temporary files
- Future: isolated databases, caches

State directories are automatically cleaned up when you remove a worktree. Orphaned states can be cleaned with:

```bash
hn prune
```

### Graphite Compatibility

**hannahanna works seamlessly with Graphite!**

[Graphite](https://graphite.dev/) is a tool for managing stacked diffs and PRs. Since Graphite is built on top of standard Git operations, hannahanna's worktree management integrates perfectly with Graphite workflows.

**Using hannahanna with Graphite:**

```bash
# Create separate worktrees for different stacks
hn add stack-auth
cd ../stack-auth
gt stack create auth-v2

# Work on your stack in isolation
# Each worktree can have its own Graphite stack

# Switch between different stacks easily
hn switch stack-billing
gt stack checkout billing-v1

# Sync stacks with main
hn sync main
gt stack sync
```

**Benefits of using hannahanna with Graphite:**
- **Isolated stacks**: Each worktree can have its own Graphite stack without interference
- **Parallel development**: Work on multiple stacks simultaneously
- **Easy context switching**: Switch between stacks without stashing or committing
- **Resource isolation**: Different dependency versions for different stacks

**Example workflow:**
```bash
# Stack 1: Authentication overhaul
hn add auth-overhaul
cd ../auth-overhaul
gt stack create
gt create "Add JWT support"
gt create "Implement refresh tokens"
gt create "Add rate limiting"

# Stack 2: Billing features (in parallel)
hn add billing-features
cd ../billing-features
gt stack create
gt create "Add subscription tiers"
gt create "Implement payment webhooks"

# Work on both stacks independently
hn switch auth-overhaul
# Make changes to JWT...
gt submit

hn switch billing-features
# Make changes to billing...
gt submit
```

Since Graphite uses standard Git branches and commits under the hood, all hannahanna commands (`integrate`, `sync`, `return`) work perfectly with Graphite-managed branches.

## Project Structure

When you use hannahanna, your repository layout looks like:

```
my-project/              # Main repository
├── .hannahanna.yml      # Configuration
├── .hn-state/           # State directories (gitignored)
│   ├── feature-x/
│   └── feature-y/
├── node_modules/        # Shared when compatible
├── src/
└── package.json

../feature-x/            # Worktree 1 (sibling directory)
├── node_modules -> ../my-project/node_modules  # Symlink
├── .env                 # Copied from .env.template
└── src/

../feature-y/            # Worktree 2 (sibling directory)
├── node_modules/        # Isolated (incompatible lockfile)
├── .env                 # Copied from .env.template
└── src/
```

## Development Status

**Current Version:** v0.4.0

**Implemented:**
- ✅ Git worktree management (add, list, remove, switch, info, prune)
- ✅ Parent/child tracking with nested workflow support
- ✅ `return` command for merging back to parent
- ✅ `integrate` command for merging worktrees/branches
- ✅ `sync` command for keeping branches up to date
- ✅ `each` command for batch operations
- ✅ Fuzzy name matching
- ✅ Shared resource symlinks with compatibility checking
- ✅ File copying for templates
- ✅ **Extended lifecycle hooks (7 total)** - v0.3
  - pre_create, post_create, post_switch
  - pre_remove, post_remove
  - pre_integrate, post_integrate
  - Conditional execution via branch patterns
- ✅ State management with file locking
- ✅ **State commands** (list/size/clean) - v0.3
- ✅ Docker integration
  - Port allocation system
  - **Port reassign command** - v0.3
  - Container lifecycle management (exec/restart/prune)
  - Docker Compose override generation
- ✅ Config management commands (init/validate/show/edit)
- ✅ **Command aliases with cycle detection** - v0.3
- ✅ Helpful error messages with actionable suggestions
- ✅ **Graphite compatibility** - Works seamlessly with Graphite stacks

**Test Coverage:** 273 tests passing (87 lib + 186 integration), zero warnings

**Multi-VCS Support (v0.3 Complete):**
- ✅ VCS abstraction layer with trait-based design
- ✅ Auto-detection (Jujutsu → Git → Mercurial priority)
- ✅ **Full Mercurial backend** (`hg share` workspaces)
- ✅ **Sparse checkout for Mercurial** - v0.3
- ✅ Full Jujutsu backend (`jj workspace` support)
- ✅ Clear error messages for unsupported VCS operations

**v0.2 Features (Complete):**
- ✅ Sparse checkout for Git and Jujutsu (monorepo support)
- ✅ Configuration hierarchy (user/repo/worktree)

**v0.3 Features (Complete):**
- ✅ Command aliases with cycle detection
- ✅ Extended hooks (7 types with conditional execution)
- ✅ State management commands
- ✅ Port reassign command
- ✅ Docker enhancements (exec/restart/prune)

**v0.4 Features (Complete):**
- ✅ **Registry caching system** - 50%+ faster worktree listings
  - Intelligent TTL-based caching (30s default)
  - Auto-invalidation on add/remove operations
  - Cache management: `hn state cache stats/clear`
- ✅ **Performance benchmark suite** - Criterion-based benchmarks
  - Established performance targets for key operations
  - Run with `cargo bench`
  - Documentation in `BENCHMARKS.md`
- ✅ **Shell completions** - Auto-completion for bash/zsh/fish
  - Generate with `hn completions <shell>`
  - Full command and option completion
- ✅ **Enhanced `hn info` output** - Rich, actionable information
  - Status with emojis (✓/⚠)
  - Age, disk usage, VCS type
  - Parent/children relationships
  - Docker memory & CPU stats
  - Suggested actions section
- ✅ Mercurial sparse checkout

**See:** [`spec/plan.md`](spec/plan.md) and [`spec/spec.md`](spec/spec.md) for detailed roadmap

## Troubleshooting

### Shell Integration Not Working

If `hn switch` or `hn return` don't change your directory, check these common issues:

#### 1. Shell Integration Not Loaded

**Symptom:** `hn switch` outputs a path but doesn't change directory

**Diagnosis:**
```bash
# Check if shell wrapper is defined
type hn
```

**Expected output:**
```
hn is a function
hn ()
{
    # ... function code ...
}
```

**If it shows `hn is /path/to/hn`:** Shell integration is not loaded.

**Fix:**
```bash
# Add to ~/.bashrc or ~/.zshrc
eval "$(hn init-shell)"

# Then reload your shell
source ~/.bashrc  # or source ~/.zshrc
```

#### 2. Wrong Shell Configuration File

**Symptom:** Works in new terminals but not current one

**Common Issues:**
- Added to `~/.bash_profile` instead of `~/.bashrc` (Linux)
- Added to `~/.zshrc` but running bash
- Added to `~/.bashrc` but running zsh

**Fix:**
```bash
# Check your shell
echo $SHELL

# Add eval "$(hn init-shell)" to the correct file:
# - Bash on Linux: ~/.bashrc
# - Bash on macOS: ~/.bash_profile or ~/.bashrc
# - Zsh: ~/.zshrc
# - Fish: ~/.config/fish/config.fish (not yet supported)

# Then reload
exec $SHELL -l
```

#### 3. Shell Integration Conflicts

**Symptom:** Shell wrapper seems loaded but doesn't work correctly

**Possible Causes:**
- Multiple `eval "$(hn init-shell)"` lines in shell config
- Conflicting aliases or functions named `hn`
- PATH issues causing wrong `hn` binary to be called

**Diagnosis:**
```bash
# Check for duplicates
grep -n "hn init-shell" ~/.bashrc ~/.zshrc ~/.bash_profile 2>/dev/null

# Check for conflicts
alias | grep hn
type -a hn
```

**Fix:**
```bash
# Remove duplicate eval lines, keep only one
# Remove conflicting aliases
unalias hn 2>/dev/null

# Reload shell
exec $SHELL -l
```

#### 4. Non-Interactive Shell

**Symptom:** Works in terminal but not in scripts

**Explanation:** Shell integration only works in interactive shells. Scripts should use `cd` with the output of `hn switch`:

```bash
# In scripts, don't rely on shell wrapper
cd "$(hn switch feature-x)"

# Or better, use full path operations
WORKTREE_PATH=$(hn switch feature-x)
cd "$WORKTREE_PATH"
./run-tests.sh
```

### Permission Errors

**Symptom:** `Permission denied` errors when creating worktrees

**Common Causes:**
- Repository is in read-only location
- Insufficient permissions for parent directory
- SELinux or filesystem restrictions

**Fix:**
```bash
# Check permissions
ls -la "$(git rev-parse --show-toplevel)"

# Ensure writable
chmod u+w path/to/repo

# Check filesystem
df -T "$(git rev-parse --show-toplevel)"
```

### Docker Port Conflicts

**Symptom:** Port allocation fails or containers won't start

**Diagnosis:**
```bash
# Check port allocations
hn ports list

# Check what's using a port
lsof -i :3000
netstat -tulpn | grep 3000
```

**Fix:**
```bash
# Release ports for a worktree
hn ports release worktree-name

# Or manually edit port registry
vi .hn-state/port-registry.json

# Change base port in config
vi .hannahanna.yml
# Set docker.ports.base to different range
```

### Hook Failures

**Symptom:** `hn add` fails with hook errors

**Diagnosis:**
```bash
# Validate config
hn config validate

# Check what hooks would run
hn config show
```

**Fix:**
```bash
# Skip hooks for untrusted repos
hn add feature-x --no-hooks

# Debug hook script
# Hooks run in worktree directory with these variables:
# - $WT_NAME, $WT_PATH, $WT_BRANCH, $WT_COMMIT, $WT_STATE_DIR

# Test hook manually
cd path/to/worktree
export WT_NAME=test
export WT_PATH=$PWD
export WT_BRANCH=$(git branch --show-current)
# ... then run hook commands
```

### Worktree in Inconsistent State

**Symptom:** Worktree shows in `hn list` but directory doesn't exist

**Fix:**
```bash
# Remove git worktree reference
git worktree prune

# Clean up hannahanna state
hn prune

# Remove manually if needed
git worktree remove --force worktree-name
rm -rf .hn-state/worktree-name
```

### Performance Issues

**Symptom:** `hn add` is slow, especially with large node_modules

**Solutions:**
```bash
# Use shared resources instead of copying
# In .hannahanna.yml:
shared_resources:
  - source: node_modules
    target: node_modules
    compatibility: package-lock.json

# Or use Docker isolation
docker:
  enabled: true
```

### Getting More Help

If you're still stuck:

1. **Check verbose output:** Most commands support `-v` or `--verbose` (planned feature)
2. **Review logs:** Check `.hn-state/` for any state files
3. **Validate git state:** Run `git worktree list` to see git's view
4. **File an issue:** [GitHub Issues](https://github.com/yourusername/hannahanna/issues)

**When reporting issues, include:**
- Output of `hn --version`
- Your shell: `echo $SHELL`
- Output of `type hn`
- Output of `git worktree list`
- Relevant error messages

## Contributing

Contributions welcome! Please follow these guidelines:

### Development Setup

1. **Fork and clone** the repository
2. **Install git hooks** to ensure code quality:
   ```bash
   ./scripts/install-git-hooks.sh
   ```
   This installs a pre-commit hook that runs `rustfmt` and `clippy` automatically.

3. **Make your changes**

4. **Run tests:**
   ```bash
   cargo test
   cargo fmt -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   ```

5. **Submit a pull request**

### Code Quality Standards

- All code must pass `rustfmt` formatting
- All code must pass `clippy` with `-D warnings` (no warnings allowed)
- All tests must pass
- New features should include tests

## License

MIT License - see [LICENSE](LICENSE) for details

## Name Origin

**Hannahanna** (Ḫannaḫanna) is the Hittite mother goddess, associated with creation and nurturing - fitting for a tool that creates and manages development environments.
