# Template Management

Hannahanna v0.5+ provides a powerful template system for creating reusable worktree configurations. Templates allow you to standardize worktree setup, share configurations across teams, and quickly bootstrap new development environments.

## Overview

Templates are stored in the `.hn-templates/` directory at the repository root. Each template is a directory containing:

- **`.hannahanna.yml`** - Configuration file (hooks, Docker, etc.)
- **`README.md`** - Template documentation
- **`files/`** - Optional directory with files to copy to new worktrees

## Quick Start

### Creating a Template

```bash
# Create a basic template
hn templates create my-template

# Create with Docker enabled
hn templates create api-dev --docker

# Create with description
hn templates create frontend-dev --description "Frontend development environment with hot reload"

# Create from current worktree configuration
hn templates create from-current --from-current
```

### Using a Template

```bash
# Create worktree from template
hn add my-feature --template my-template
```

### Listing Templates

```bash
# List all templates (formatted)
hn templates list

# List as JSON
hn templates list --json
```

### Viewing Template Details

```bash
# Show template configuration
hn templates show my-template
```

## Template Structure

### Directory Layout

```
.hn-templates/
└── my-template/
    ├── .hannahanna.yml    # Worktree configuration
    ├── README.md          # Template documentation
    └── files/             # Files to copy to worktrees
        ├── .env.example
        ├── config/
        │   └── dev.yml
        └── scripts/
            └── setup.sh
```

### Configuration File (`.hannahanna.yml`)

The template's `.hannahanna.yml` defines the worktree configuration:

```yaml
# Hooks
hooks:
  post_create: |
    echo "Setting up development environment..."
    npm install
    cp .env.example .env
    chmod +x scripts/setup.sh
    ./scripts/setup.sh

# Docker configuration
docker:
  enabled: true
  services:
    - app
    - db
  ports:
    app: auto
    db: 5432
```

### File Copying with Variable Substitution

Files in the `files/` directory are copied to new worktrees with variable substitution:

**Template file** (`files/.env.example`):
```bash
# Environment variables for ${HNHN_NAME}
WORKTREE_NAME=${HNHN_NAME}
WORKTREE_PATH=${HNHN_PATH}
WORKTREE_BRANCH=${HNHN_BRANCH}
API_PORT=3000
DB_HOST=localhost
```

**Result in worktree** (when `hn add my-feature --template my-template`):
```bash
# Environment variables for my-feature
WORKTREE_NAME=my-feature
WORKTREE_PATH=/path/to/repo/../my-feature
WORKTREE_BRANCH=my-feature
API_PORT=3000
DB_HOST=localhost
```

### Available Variables

- `${HNHN_NAME}` - Worktree name
- `${HNHN_PATH}` - Absolute path to worktree
- `${HNHN_BRANCH}` - Git branch name

Variables are substituted in **text files only**. Binary files are copied as-is.

## Common Template Patterns

### 1. Frontend Development Template

```yaml
# .hn-templates/frontend/.hannahanna.yml
hooks:
  post_create: |
    npm install
    npm run dev &
    echo "Frontend dev server started on port ${HNHN_DOCKER_PORT_APP:-3000}"

docker:
  enabled: true
  services:
    - app
  ports:
    app: auto
  port_range:
    start: 3000
    end: 3100
```

**Files:**
```
frontend/
└── files/
    ├── .env.example
    ├── .eslintrc.json
    └── vite.config.ts
```

### 2. Backend API Template

```yaml
# .hn-templates/api/.hannahanna.yml
hooks:
  post_create: |
    python -m venv venv
    source venv/bin/activate
    pip install -r requirements.txt
    cp .env.example .env
    flask db upgrade
    echo "API ready at http://localhost:${HNHN_DOCKER_PORT_APP}"

docker:
  enabled: true
  services:
    - app
    - db
    - redis
  ports:
    app: auto
    db: 5432
    redis: 6379
```

### 3. Microservice Template

```yaml
# .hn-templates/microservice/.hannahanna.yml
hooks:
  post_create: |
    # Service-specific setup
    SERVICE_NAME=${HNHN_NAME}
    docker-compose -f docker-compose.${SERVICE_NAME}.yml up -d

  pre_remove: |
    SERVICE_NAME=${HNHN_NAME}
    docker-compose -f docker-compose.${SERVICE_NAME}.yml down

docker:
  enabled: true
  services:
    - app
    - db
  ports:
    app: auto
    db: auto
```

### 4. Testing Environment Template

```yaml
# .hn-templates/testing/.hannahanna.yml
hooks:
  post_create: |
    # Install test dependencies
    npm install --include=dev

    # Setup test database
    createdb test_db_${HNHN_NAME}

    # Run initial test suite
    npm test

  pre_remove: |
    # Cleanup test database
    dropdb test_db_${HNHN_NAME}

docker:
  enabled: false
```

## Advanced Usage

### Creating Templates from Existing Worktrees

Save your current worktree configuration as a template:

```bash
cd my-feature
hn templates create production-ready --from-current
```

This captures:
- Current `.hannahanna.yml` configuration
- All hooks
- Docker settings
- Files you've marked for template inclusion

### Template Validation

Templates are validated when created:

```bash
$ hn templates create invalid/name
Error: Invalid template name 'invalid/name'.
Template names cannot contain path separators or start with '.'
```

Valid template names:
- ✓ `frontend-dev`
- ✓ `api_v2`
- ✓ `microservice-template`
- ✗ `invalid/name` (contains `/`)
- ✗ `.hidden` (starts with `.`)

### Template Inheritance

While Hannahanna doesn't support direct template inheritance, you can:

1. **Copy and modify** existing templates
2. **Use hooks** to source shared scripts
3. **Symlink common files** in the `files/` directory

Example:
```bash
# Create base template
hn templates create base-dev

# Copy to create specialized version
cp -r .hn-templates/base-dev .hn-templates/frontend-dev

# Customize frontend-dev
vim .hn-templates/frontend-dev/.hannahanna.yml
```

### Permission Preservation

**Unix/Linux/macOS:** File permissions are preserved when copying from templates.

**Template file:**
```bash
#!/bin/bash
# scripts/setup.sh (chmod +x)
echo "Running setup..."
```

**Result:** `setup.sh` remains executable in the worktree.

## Template Management Commands

### List Templates

```bash
# Formatted output
$ hn templates list
Template Name        Description                          Files
─────────────────────────────────────────────────────────────
frontend-dev        Frontend development environment      5 files
api-backend         API backend with PostgreSQL           3 files
testing             Testing environment                   2 files

# JSON output
$ hn templates list --json
[
  {
    "name": "frontend-dev",
    "description": "Frontend development environment",
    "docker_enabled": true,
    "file_count": 5,
    "has_hooks": true
  }
]
```

### Show Template

```bash
$ hn templates show frontend-dev
Template: frontend-dev
Description: Frontend development environment
Created: 2025-01-15
Docker: Enabled (2 services: app, db)

Configuration:
  hooks:
    post_create: |
      npm install
      npm run dev &

  docker:
    enabled: true
    services: [app, db]
    ports:
      app: auto

Files (5):
  .env.example
  .eslintrc.json
  vite.config.ts
  package.json
  README.md
```

### Delete Template

Templates are just directories - delete them manually:

```bash
rm -rf .hn-templates/old-template
```

## Best Practices

### 1. Document Your Templates

Always include a descriptive `README.md`:

```markdown
# Frontend Dev Template

## What This Template Provides

- Node.js development environment
- Hot reload with Vite
- ESLint configuration
- Environment variables setup

## Usage

hn add my-feature --template frontend-dev

## Post-Creation Steps

1. Review `.env` file
2. Install dependencies: `npm install`
3. Start dev server: `npm run dev`
```

### 2. Use `.env.example` Files

Never commit secrets. Use `.env.example` in templates:

```bash
# .env.example
DATABASE_URL=postgresql://localhost/dev_${HNHN_NAME}
API_KEY=your-api-key-here
SECRET_KEY=generate-this-yourself
```

Hook copies and prompts:
```yaml
hooks:
  post_create: |
    cp .env.example .env
    echo "⚠️  Remember to set your API_KEY in .env"
```

### 3. Keep Templates Minimal

Only include files that vary between worktrees. Share common configs via:
- Symlinks
- Git submodules
- Shared scripts in repo root

### 4. Version Control Your Templates

Commit `.hn-templates/` to Git:

```bash
git add .hn-templates/
git commit -m "Add frontend development template"
```

### 5. Test Templates Before Sharing

```bash
# Create test worktree
hn add test-template --template my-new-template

# Verify setup works
cd test-template
# ... test your setup ...

# Clean up
hn remove test-template --force
```

## Troubleshooting

### Template Not Found

```
Error: Template 'my-template' not found
```

**Solution:** List available templates with `hn templates list`

### Variable Substitution Not Working

Variables only work in **text files**. Binary files are copied as-is.

**Check file type:**
```bash
file myfile.txt  # Should show "text"
```

### Files Not Copying

Ensure the `files/` directory exists:

```bash
mkdir -p .hn-templates/my-template/files
```

### Permission Issues

On Unix systems, ensure template files have correct permissions:

```bash
chmod +x .hn-templates/my-template/files/scripts/*.sh
```

## Examples

### Complete Frontend Template

```bash
# Create template structure
hn templates create react-app --docker

# Add configuration
cat > .hn-templates/react-app/.hannahanna.yml <<'EOF'
hooks:
  post_create: |
    npm install
    cp .env.example .env
    echo "React app ready! Run: npm start"

docker:
  enabled: true
  services:
    - app
  ports:
    app: auto
  port_range:
    start: 3000
    end: 3100
EOF

# Add template files
mkdir -p .hn-templates/react-app/files
cat > .hn-templates/react-app/files/.env.example <<'EOF'
REACT_APP_NAME=${HNHN_NAME}
REACT_APP_API_URL=http://localhost:8080
PORT=${HNHN_DOCKER_PORT_APP}
EOF

# Create worktree from template
hn add my-react-feature --template react-app
```

## Migration Guide

### From Manual Setup to Templates

**Before (manual):**
```bash
hn add feature-x
cd feature-x
npm install
cp ../config/.env.example .env
# ... 10 more manual steps ...
```

**After (template):**
```bash
hn add feature-x --template frontend-dev
# Done! Everything configured automatically.
```

### Converting Existing Configs to Templates

1. Identify your standard setup steps
2. Create template with those steps in hooks
3. Move shared files to template `files/` directory
4. Test template with new worktree
5. Document template usage

## See Also

- [Hooks Documentation](./hooks.md) - Hook execution details
- [Docker Integration](./docker.md) - Docker configuration
- [Workspace Management](./workspaces.md) - Saving worktree sets
- [Examples](./examples.md) - More template examples
