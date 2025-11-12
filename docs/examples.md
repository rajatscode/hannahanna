# Hannahanna Real-World Examples

This guide demonstrates common workflows and scenarios using hannahanna (`hn`).

## Table of Contents

1. [Multi-Feature Development](#1-multi-feature-development)
2. [Hotfix While In Feature Branch](#2-hotfix-while-in-feature-branch)
3. [Monorepo with Sparse Checkout](#3-monorepo-with-sparse-checkout)
4. [Docker + Port Conflicts](#4-docker--port-conflicts)
5. [Team Collaboration with Hooks](#5-team-collaboration-with-hooks)
6. [Reviewing Pull Requests](#6-reviewing-pull-requests)
7. [Experimental Features](#7-experimental-features)

---

## 1. Multi-Feature Development

**Scenario**: You're working on a large feature but need to start another feature in parallel.

### Setup

```bash
# Start from main branch
cd my-project

# Create first feature
hn add feature-auth --from main
cd ../feature-auth
# Work on authentication...
```

### Start Second Feature

```bash
# No need to commit or stash! Just create another worktree
hn add feature-payments --from main

# Switch to it
hn switch feature-payments

# Or use absolute navigation
cd ~/projects/my-project-worktrees/feature-payments
```

### Check Status Across All Worktrees

```bash
# See all worktrees with tree view
hn list --tree

# Run tests in all worktrees
hn each "npm test"

# Run tests in parallel (faster!)
hn each --parallel "npm test"
```

### Integrate When Ready

```bash
# Feature auth is done, merge it
hn integrate feature-auth main

# Continue working on payments
hn switch feature-payments
```

### Cleanup

```bash
# Remove completed feature
hn remove feature-auth

# Clean orphaned state
hn state clean
```

---

## 2. Hotfix While In Feature Branch

**Scenario**: Critical bug in production while you're mid-feature with uncommitted changes.

### Current State

```bash
# You're in feature-redesign with uncommitted work
pwd  # ~/projects/my-app-worktrees/feature-redesign
git status  # 15 files modified, tests broken
```

### Create Hotfix Worktree

```bash
# Create hotfix worktree from main
hn add hotfix-security --from main

# Your feature branch remains untouched!
# No stashing needed

# Switch to hotfix
hn switch hotfix-security
```

### Fix and Deploy

```bash
# Make the fix
vim src/auth.rs
git add src/auth.rs
git commit -m "fix: patch security vulnerability"

# Push and deploy
git push
# ... deploy to production ...

# Integrate back to main
hn integrate hotfix-security main
```

### Return to Feature Work

```bash
# Go back to your feature
hn switch feature-redesign

# Your uncommitted changes are exactly as you left them!
git status  # Still 15 files modified

# Sync the hotfix into your feature branch
hn sync main
```

### Cleanup

```bash
hn remove hotfix-security
```

---

## 3. Monorepo with Sparse Checkout

**Scenario**: Large monorepo, you only need specific packages.

### Configuration

Create `.hannahanna.yml`:

```yaml
# Default sparse checkout for all worktrees
sparse:
  enabled: true
  paths:
    - /packages/shared
    - /packages/api
    - /tools
    - /package.json
    - /tsconfig.json

# Docker port ranges for monorepo services
docker:
  enabled: true
  ports:
    base:
      api: 3000
      db: 5432
      redis: 6379
```

### Create Worktree with Sparse Checkout

```bash
# Uses config defaults
hn add feature-api-endpoints

# Override for specific worktree
hn add feature-frontend --sparse /packages/shared /packages/web /packages/ui
```

### Verify Sparse Checkout

```bash
hn switch feature-api-endpoints
ls packages/
# Only shows: shared/ api/

# If you need more paths later
cd ../feature-api-endpoints
git sparse-checkout add /packages/auth
```

---

## 4. Docker + Port Conflicts

**Scenario**: Running multiple worktrees with Docker services simultaneously.

### Configuration

```yaml
# .hannahanna.yml
docker:
  enabled: true
  auto_start: true
  compose_file: docker-compose.yml
  ports:
    base:
      app: 3000
      postgres: 5432
      redis: 6379
    increment: 10
```

### Usage

```bash
# Create first feature
hn add feature-a
# Ports allocated:
#   app: 3000
#   postgres: 5432
#   redis: 6379

# Create second feature
hn add feature-b
# Ports automatically incremented:
#   app: 3010
#   postgres: 5442
#   redis: 6389

# Check port allocations
hn ports list
# feature-a  app: 3000, postgres: 5432, redis: 6379
# feature-b  app: 3010, postgres: 5442, redis: 6389
```

### Managing Docker

```bash
# View container status for all worktrees
hn each --docker-running "docker ps"

# Restart containers
hn docker restart feature-a

# Execute command in container
hn docker exec feature-a postgres "psql -U myuser"

# View logs
hn docker logs feature-a app

# Cleanup unused containers
hn docker prune
```

### Port Conflicts

```bash
# Manually reassign ports if needed
hn ports reassign feature-a app 4000

# Or release and reallocate
hn remove feature-a
hn add feature-a  # Gets fresh ports
```

---

## 5. Team Collaboration with Hooks

**Scenario**: Enforce team standards and automate setup.

### Team Configuration

Create `.hannahanna.yml` in your repo:

```yaml
hooks:
  # Ensure branch naming convention
  pre_create: |
    if ! echo "$WT_BRANCH" | grep -qE '^(feature|bugfix|hotfix)/'; then
      echo "❌ Branch must start with feature/, bugfix/, or hotfix/"
      exit 1
    fi

  # Automatic setup
  post_create: |
    echo "📦 Installing dependencies..."
    npm install

    echo "🗄️  Setting up database..."
    npm run db:migrate

    echo "✅ Worktree ready!"

  # Conditional hooks for feature branches
  post_create_conditions:
    - condition: "branch.startsWith('feature/')"
      command: |
        echo "🚀 Feature branch detected"
        npm run generate:types

    - condition: "branch.contains('api')"
      command: |
        echo "🔌 API feature detected"
        docker-compose up -d api-deps

  # Safety checks before integration
  pre_integrate: |
    npm test
    npm run lint
    if [ $? -ne 0 ]; then
      echo "❌ Tests or linting failed. Fix before integrating."
      exit 1
    fi

  # Notification after integration
  post_integrate: |
    echo "✅ Integration complete!"
    # Optional: Notify team via Slack/Discord
    # curl -X POST $SLACK_WEBHOOK -d "..."

# Docker configuration
docker:
  enabled: true
  auto_start: true
```

### Usage

```bash
# Create feature - validates name and auto-installs
hn add feature/new-api-endpoint
# ✓ Branch name valid
# 📦 Installing dependencies...
# 🗄️  Setting up database...
# 🚀 Feature branch detected
# 🔌 API feature detected
# ✅ Worktree ready!

# Try invalid name
hn add my-feature
# ❌ Branch must start with feature/, bugfix/, or hotfix/
# Error: Hook failed

# Integrate with safety checks
hn integrate feature/new-api-endpoint main
# Running tests...
# Running linter...
# ✓ All checks passed
# ✅ Integration complete!
```

---

## 6. Reviewing Pull Requests

**Scenario**: Quickly checkout and test pull requests.

### Review Workflow

```bash
# Create worktree for PR review
hn add review-pr-123 --from origin/pull/123/head --no-branch

# Or if it's a branch
hn add review-user-feature --from origin/user/feature --no-branch

# Check it out
hn switch review-pr-123
hn info review-pr-123
# Shows: branch, commit, age, disk usage, etc.

# Run tests
npm test
npm run build

# Make notes or small fixes
vim notes.md
# (these changes stay isolated)

# When done reviewing
hn switch main
hn remove review-pr-123
```

### Bulk Review

```bash
# Create worktrees for multiple PRs
hn add review-pr-123 --from origin/pull/123/head --no-branch
hn add review-pr-124 --from origin/pull/124/head --no-branch
hn add review-pr-125 --from origin/pull/125/head --no-branch

# Run tests in all review worktrees
hn each --filter "review-pr-" "npm test"

# Check results
hn list --tree

# Cleanup all review worktrees
hn each --filter "review-pr-" "true" && \
  hn remove review-pr-123 && \
  hn remove review-pr-124 && \
  hn remove review-pr-125
```

---

## 7. Experimental Features

**Scenario**: Test risky changes without affecting your main work.

### Create Experimental Branch

```bash
# Create experiment from current feature
hn add experiment-refactor --from feature-auth

# Make aggressive changes
cd ../experiment-refactor
rm -rf src/old-code
# Rewrite everything...
```

### Compare Approaches

```bash
# Keep both approaches running
hn docker start feature-auth       # Original on port 3000
hn docker start experiment-refactor # Experiment on port 3010

# Test both
curl localhost:3000/api/login
curl localhost:3010/api/login

# Compare performance
hn each --filter "feature-auth|experiment-refactor" "npm run benchmark"
```

### Decide Which to Keep

```bash
# Experiment is better, integrate it
hn integrate experiment-refactor main
hn remove feature-auth

# Or original was better
hn remove experiment-refactor
hn integrate feature-auth main
```

### Quick Experiments

```bash
# Try something quickly
hn add test-idea --from main
cd ../test-idea
# Hack away...

# Nope, didn't work
cd ../main
hn remove test-idea --force  # Force removes even with uncommitted changes
```

---

## Advanced Tips

### Automatic Cleanup

```bash
# Find old worktrees
hn list | grep "30 days ago"

# Clean orphaned state
hn state clean

# Remove old branches
hn each "git branch --merged | grep -v main | xargs git branch -d"
```

### Performance

```bash
# Check cache status
hn state cache stats

# Warm up cache
hn list

# Clear cache if stale
hn state cache clear
```

### Scripting

```bash
# Create worktree with error handling
if hn add feature-x --from main; then
  cd ../feature-x
  npm install
else
  echo "Failed to create worktree"
  exit 1
fi

# Loop through worktrees
for worktree in $(hn list | awk '{print $1}'); do
  echo "Processing $worktree"
  hn switch $worktree
  git pull
done
```

### Shell Integration

```bash
# Add to ~/.bashrc or ~/.zshrc
eval "$(hn init-shell)"

# Now you have:
# - Automatic directory switching
# - Prompt integration showing current worktree

# Generate completions
hn completions bash > ~/.local/share/bash-completion/completions/hn
source ~/.local/share/bash-completion/completions/hn
```

---

## Troubleshooting

### Problem: Worktree won't remove

```bash
# Check what's preventing removal
hn info worktree-name

# Force remove
hn remove worktree-name --force

# Clean up any leftover state
hn state clean
```

### Problem: Docker ports conflicting

```bash
# List current allocations
hn ports list

# Reassign specific port
hn ports reassign worktree-name service 9000

# Or remove and recreate for fresh ports
hn remove worktree-name
hn add worktree-name
```

### Problem: Hooks failing

```bash
# Skip hooks temporarily
hn add worktree-name --no-hooks

# Check hook configuration
hn config show

# Validate config
hn config validate
```

### Problem: Slow list command

```bash
# Check if cache is working
hn state cache stats

# Clear and rebuild cache
hn state cache clear
hn list  # Rebuilds cache
```

---

## Next Steps

- Read [MIGRATING.md](MIGRATING.md) for v0.4 upgrade info
- See [README.md](../README.md) for full command reference
- Check [BENCHMARKS.md](../BENCHMARKS.md) for performance tuning
- Review `.hannahanna.yml` [configuration docs](../README.md#configuration)
