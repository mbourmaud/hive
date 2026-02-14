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

# ── Rust checks ─────────────────────────────────────────────────
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

# ── Web frontend checks ────────────────────────────────────────
if [ -d "web" ] && git diff --cached --name-only | grep -q "^web/src/"; then
    echo "→ Running Biome on staged web files..."
    STAGED_WEB=$(git diff --cached --name-only --diff-filter=ACM | grep "^web/src/.*\.\(ts\|tsx\|css\)$" || true)
    if [ -n "$STAGED_WEB" ]; then
        if ! (cd web && npx @biomejs/biome check --no-errors-on-unmatched $STAGED_WEB); then
            echo "❌ Biome check failed!"
            echo "   Run: cd web && npm run lint:fix"
            exit 1
        fi
        echo "✓ Biome OK"
    fi

    echo "→ Checking TypeScript..."
    if ! (cd web && npx tsc --noEmit); then
        echo "❌ TypeScript check failed!"
        exit 1
    fi
    echo "✓ TypeScript OK"
fi

echo "✅ All pre-commit checks passed!"
EOF

chmod +x "$HOOKS_DIR/pre-commit"

echo "✅ Git hooks installed successfully!"
echo ""
echo "The following hooks are now active:"
echo "  • pre-commit: Rust (fmt + clippy) + Web (biome + tsc)"
echo ""
echo "To bypass hooks (not recommended), use: git commit --no-verify"
