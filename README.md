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

### Shell Integration

For the `hn switch` command to work (changes directory), add this to your `~/.bashrc` or `~/.zshrc`:

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

**Safety checks:**
- Warns about uncommitted changes (unless `--force`)
- Runs `pre_remove` hook if configured
- Cleans up state directories

### `hn prune`

Clean up orphaned state directories from deleted worktrees.

```bash
hn prune
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
```

**⚠️ SECURITY WARNING:** Hooks execute arbitrary shell commands from your `.hannahanna.yml` configuration file. Only use hannahanna in repositories you trust. Never clone and run `hn add` in untrusted repositories without first reviewing the `.hannahanna.yml` file for malicious hooks.

**Available hooks:**
- `post_create` - Runs after worktree creation
- `pre_remove` - Runs before worktree deletion

**Environment variables available in hooks:**
- `$WT_NAME` - Worktree name
- `$WT_PATH` - Worktree path
- `$WT_BRANCH` - Branch name
- `$WT_PARENT` - Parent worktree (if any)
- `$WT_STATE_DIR` - State directory path

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

Each worktree gets a state directory (`.wt-state/<name>/`) for:
- Worktree-specific metadata
- Logs and temporary files
- Future: isolated databases, caches

State directories are automatically cleaned up when you remove a worktree. Orphaned states can be cleaned with:

```bash
hn prune
```

## Project Structure

When you use hannahanna, your repository layout looks like:

```
my-project/              # Main repository
├── .hannahanna.yml      # Configuration
├── .wt-state/           # State directories (gitignored)
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

**Current Version:** 0.1+ (MVP + Enhancements)

**Implemented:**
- ✅ Git worktree management (add, list, remove, switch, info, prune)
- ✅ Parent/child tracking with nested workflow support
- ✅ `return` command for merging back to parent
- ✅ Fuzzy name matching
- ✅ Shared resource symlinks with compatibility checking
- ✅ File copying for templates
- ✅ Lifecycle hooks (post_create, pre_remove)
- ✅ State management with file locking
- ✅ **Docker integration** (ahead of schedule!)
  - Port allocation system
  - Container lifecycle management
  - Docker Compose override generation
- ✅ **Config management commands** (init/validate/show/edit)
- ✅ **Helpful error messages with actionable suggestions**

**Test Coverage:** 193 tests passing, ~80% coverage

**Planned for v0.2:**
- Advanced hook conditions
- Multi-VCS support (Mercurial, Jujutsu)
- Sparse checkout for monorepos

**See:** [`spec/plan.md`](spec/plan.md) and [`spec/spec.md`](spec/spec.md) for detailed roadmap

## Contributing

Contributions welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details

## Name Origin

**Hannahanna** (Ḫannaḫanna) is the Hittite mother goddess, associated with creation and nurturing - fitting for a tool that creates and manages development environments.
