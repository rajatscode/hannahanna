#!/bin/bash
# Install git hooks for hannahanna development
# This script copies the pre-commit hook to .git/hooks/

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "Installing git hooks for hannahanna..."

# Check if we're in a git repository
if [ ! -d "$REPO_ROOT/.git" ]; then
    echo "❌ Error: Not in a git repository"
    echo "   Please run this script from the hannahanna repository"
    exit 1
fi

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Install pre-commit hook
echo "→ Installing pre-commit hook..."
cp "$SCRIPT_DIR/git-hooks/pre-commit" "$HOOKS_DIR/pre-commit"
chmod +x "$HOOKS_DIR/pre-commit"
echo "✓ Pre-commit hook installed"

echo ""
echo "✅ Git hooks installed successfully!"
echo ""
echo "The pre-commit hook will now run automatically before each commit to:"
echo "  • Check code formatting with rustfmt"
echo "  • Run clippy lints with -D warnings"
echo ""
echo "To skip the hook for a specific commit (not recommended):"
echo "  git commit --no-verify"
