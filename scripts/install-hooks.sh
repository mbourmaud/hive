#!/bin/bash
# Install git hooks for Hive project
# Run this script after cloning the repository

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$PROJECT_ROOT/.git/hooks"

echo "Installing git hooks..."

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Install pre-commit hook
cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/bash
# Pre-commit hook for Hive project
# Runs formatting and linting checks before allowing commits

set -e

echo "Running pre-commit checks..."

# Check if this is a Rust project
if [ -f "Cargo.toml" ]; then
    echo "→ Checking Rust formatting..."
    if ! cargo fmt --all -- --check; then
        echo "❌ Formatting check failed!"
        echo "   Run: cargo fmt --all"
        exit 1
    fi
    echo "✓ Formatting OK"

    echo "→ Running Clippy..."
    if ! cargo clippy --all-targets --all-features -- -D warnings; then
        echo "❌ Clippy check failed!"
        echo "   Fix the warnings above"
        exit 1
    fi
    echo "✓ Clippy OK"
fi

echo "✅ All pre-commit checks passed!"
EOF

chmod +x "$HOOKS_DIR/pre-commit"

echo "✅ Git hooks installed successfully!"
echo ""
echo "The following hooks are now active:"
echo "  • pre-commit: Checks formatting and runs Clippy"
echo ""
echo "To bypass hooks (not recommended), use: git commit --no-verify"
