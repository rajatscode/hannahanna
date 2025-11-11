#!/bin/bash
# Install git hooks for hannahanna development
# This script should be run after cloning the repository

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GIT_HOOKS_DIR="$SCRIPT_DIR/../.git/hooks"

echo "Installing git hooks..."

# Install pre-commit hook
if [ -f "$SCRIPT_DIR/git-hooks/pre-commit" ]; then
    cp "$SCRIPT_DIR/git-hooks/pre-commit" "$GIT_HOOKS_DIR/pre-commit"
    chmod +x "$GIT_HOOKS_DIR/pre-commit"
    echo "✓ Installed pre-commit hook (rustfmt + clippy)"
else
    echo "⚠ pre-commit hook not found"
fi

# Install pre-push hook
if [ -f "$SCRIPT_DIR/git-hooks/pre-push" ]; then
    cp "$SCRIPT_DIR/git-hooks/pre-push" "$GIT_HOOKS_DIR/pre-push"
    chmod +x "$GIT_HOOKS_DIR/pre-push"
    echo "✓ Installed pre-push hook (tests)"
else
    echo "⚠ pre-push hook not found"
fi

echo ""
echo "✅ Git hooks installed successfully!"
echo ""
echo "What runs when:"
echo "  • On commit:  cargo fmt + cargo clippy (fast, ~3s)"
echo "  • On push:    cargo test (thorough, ~15-20s)"
echo ""
echo "This ensures code quality without slowing down your workflow."
