# hannahanna v0.1.0 Release Notes

**Release Date:** January 11, 2025

We're excited to announce the first stable release of **hannahanna** (command: `hn`) - a powerful Git worktree manager that enables true parallel development with isolated environments!

## 🚀 What is hannahanna?

hannahanna makes it easy to work on multiple Git branches simultaneously. Each worktree gets its own complete, isolated development environment while intelligently sharing resources when safe.

**Perfect for:**
- 🔀 Working on multiple features in parallel without stashing
- 🔍 Creating isolated environments for code reviews
- 🧪 Running experiments without affecting your main work
- 🚨 Quick context switching for hotfixes
- 🏗️ Managing nested workflows with parent/child relationships

## ✨ Highlights

### 🎯 Core Workflow
```bash
# Create worktrees for multiple features
hn add feature-auth
hn add feature-billing
hn add feature-dashboard

# List all worktrees with tree view
hn list --tree

# Switch between worktrees (requires shell integration)
hn switch feature-auth

# Return to parent with merge
hn return --merge --delete
```

### ⚡ Batch Operations
```bash
# Run tests across all worktrees in parallel
hn each --parallel cargo test

# Run command only on feature branches
hn each --filter="^feature" git status

# Stop on first failure
hn each --stop-on-error cargo check
```

### 🐳 Docker Integration
```bash
# Start containers for a worktree
hn docker start feature-x

# View status of all containers
hn docker ps

# View logs
hn docker logs feature-x web
```

### 🔄 Intelligent Resource Sharing
- Automatically shares `node_modules`, `vendor`, `.venv` when compatible
- Checks lockfiles for compatibility before symlinking
- Falls back to copying when needed
- Saves disk space and installation time

### 🪝 Powerful Hooks System
```yaml
hooks:
  post_create: |
    npm install
    echo "✓ Dependencies installed"

  pre_remove: |
    docker-compose down
    echo "✓ Containers cleaned up"
```

## 📊 By The Numbers

- **132 tests** - Comprehensive test coverage
- **12 commands** - Simple, focused interface
- **3 subsystems** - Config, Docker, Ports management
- **0 panics** - Graceful error handling throughout
- **85%+ coverage** - Well-tested codebase

## 🔧 Installation

### From source:
```bash
git clone https://github.com/rajatscode/hannahanna
cd hannahanna
cargo build --release
sudo cp target/release/hn /usr/local/bin/
```

### Shell Integration (Required for `hn switch`):
Add to your `~/.bashrc` or `~/.zshrc`:
```bash
eval "$(hn init-shell)"
```

### Git Hooks (Recommended for contributors):
```bash
./scripts/install-git-hooks.sh
```

## 📚 Documentation

- **README.md** - Full command reference and examples
- **spec/spec.md** - Complete feature specification
- **spec/vision.md** - Roadmap and design philosophy
- **CHANGELOG.md** - Detailed change history

## 🙏 Getting Started

1. **Install hannahanna** (see above)
2. **Add shell integration** to your shell rc file
3. **Create a config** (optional): `hn config init`
4. **Create your first worktree**: `hn add my-feature`
5. **Switch to it**: `hn switch my-feature`

## 🎯 Use Cases

### Parallel Feature Development
```bash
hn add feature-auth
hn add feature-billing
hn add feature-dashboard
hn each --parallel npm test  # Test all at once!
```

### Code Review Workflow
```bash
hn add review-pr-123 --from origin/pr-123 --no-branch
# Review code, test locally
hn remove review-pr-123  # Clean up when done
```

### Hotfix During Feature Work
```bash
# Working on big-refactor
hn add hotfix-critical --from main
# Fix bug, test, merge
hn return --merge --delete
# Back to big-refactor
```

### Nested Development
```bash
hn add feature-payment
hn add fix-validation --from feature-payment
# Fix bug in nested worktree
hn return --merge  # Merge fix into feature-payment
```

## 🚦 What's Next?

We're following a careful roadmap to v1.0:

- **v0.2** - Enhanced Docker features, performance monitoring
- **v0.3** - Advanced hooks with conditions, config hierarchy
- **v0.4** - Team coordination features

See [spec/vision.md](spec/vision.md) for details.

## 🤝 Contributing

We welcome contributions! The codebase is well-tested and documented:

1. Fork the repository
2. Install git hooks: `./scripts/install-git-hooks.sh`
3. Create a feature branch: `hn add my-feature`
4. Make your changes and ensure tests pass
5. Submit a pull request

All contributions should:
- Pass `cargo test` (132 tests)
- Pass `cargo clippy` with no warnings
- Follow `cargo fmt` formatting
- Include tests for new features

## 📝 License

MIT OR Apache-2.0

---

## 🐛 Known Issues

None at release! 🎉

If you encounter any issues, please report them at:
https://github.com/rajatscode/hannahanna/issues

---

**Thank you for trying hannahanna!** We hope it makes your Git workflow smoother and more productive. Happy coding! 🚀
