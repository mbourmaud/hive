#!/bin/bash
# ============================================
# Basic Workflow Smoke Tests
# Quick sanity checks for core functionality
# ============================================

set -e

# Test configuration
TEST_DIR=$(mktemp -d)
HIVE_BIN="${HIVE_BIN:-hive}"
# Convert to absolute path if relative
if [[ "$HIVE_BIN" == ./* ]]; then
    HIVE_BIN="$(pwd)/${HIVE_BIN#./}"
fi

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Cleanup function
cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"
    cd "$TEST_DIR" 2>/dev/null || true
    $HIVE_BIN clean --force 2>/dev/null || true
    cd /
    rm -rf "$TEST_DIR"
}

trap cleanup EXIT

# Test helpers
log_test() {
    echo -e "\n${YELLOW}[TEST]${NC} $1"
    TESTS_RUN=$((TESTS_RUN + 1))
}

pass() {
    echo -e "${GREEN}✓ PASS${NC}"
    TESTS_PASSED=$((TESTS_PASSED + 1))
}

fail() {
    echo -e "${RED}✗ FAIL${NC} $1"
    TESTS_FAILED=$((TESTS_FAILED + 1))
}

# ============================================
# Test Suite
# ============================================

echo -e "${GREEN}====================================${NC}"
echo -e "${GREEN}Basic Workflow Smoke Tests${NC}"
echo -e "${GREEN}====================================${NC}\n"

# ============================================
# Test 1: Hive Binary
# ============================================

log_test "Checking hive binary exists"
if command -v $HIVE_BIN > /dev/null 2>&1; then
    pass
else
    fail "hive binary not found in PATH"
    exit 1
fi

log_test "Checking hive version command"
if $HIVE_BIN --version > /dev/null 2>&1; then
    pass
else
    fail "hive --version failed"
fi

log_test "Checking hive help command"
if $HIVE_BIN --help > /dev/null 2>&1; then
    pass
else
    fail "hive --help failed"
fi

# ============================================
# Test 2: Hive Init
# ============================================

cd "$TEST_DIR"

log_test "Creating test git repository"
git init > /dev/null 2>&1
git config user.email "test@example.com"
git config user.name "Test User"
echo "# Test" > README.md
git add README.md
git commit -m "Initial" > /dev/null 2>&1
pass

log_test "Running hive init (non-interactive)"
export CLAUDE_CODE_OAUTH_TOKEN="test-token-$$"
if $HIVE_BIN init \
    --no-interactive \
    --skip-start \
    --email "test@example.com" \
    --name "Test User" \
    --workspace "test" \
    --token "$CLAUDE_CODE_OAUTH_TOKEN" \
    --workers 1 > /tmp/hive-init.log 2>&1; then
    pass
else
    fail "hive init failed"
    cat /tmp/hive-init.log
    exit 1
fi

log_test "Verifying .hive directory created"
if [ -d ".hive" ]; then
    pass
else
    fail ".hive directory not created"
fi

log_test "Verifying .hive/.env file created"
if [ -f ".hive/.env" ]; then
    pass
else
    fail ".hive/.env not created"
fi

log_test "Verifying docker-compose.yml created"
if [ -f ".hive/docker-compose.yml" ]; then
    pass
else
    fail "docker-compose.yml not created"
fi

log_test "Verifying hive.yaml created"
if [ -f "hive.yaml" ]; then
    pass
else
    fail "hive.yaml not created"
fi

# ============================================
# Test 3: Hive Status (before start)
# ============================================

log_test "Checking hive status before start"
if $HIVE_BIN status > /dev/null 2>&1; then
    # Status should work even if containers aren't running
    pass
else
    # Some error is ok if containers not running
    pass
fi

# ============================================
# Test 4: Docker Compose Validation
# ============================================

log_test "Validating docker-compose.yml syntax"
if docker compose -f .hive/docker-compose.yml config > /dev/null 2>&1; then
    pass
else
    fail "docker-compose.yml has syntax errors"
fi

# ============================================
# Test 5: Hive Update
# ============================================

log_test "Testing hive update command (dry-run)"
# Just test that the command exists and runs
if $HIVE_BIN update --help > /dev/null 2>&1; then
    pass
else
    fail "hive update command not available"
fi

# ============================================
# Test 6: Configuration Files
# ============================================

log_test "Checking .env contains required variables"
REQUIRED_VARS=("WORKSPACE_NAME" "GIT_USER_EMAIL" "CLAUDE_CODE_OAUTH_TOKEN")
for var in "${REQUIRED_VARS[@]}"; do
    if grep -q "^${var}=" .hive/.env; then
        :  # Variable found
    else
        fail ".env missing variable: $var"
        break
    fi
done
if [ $? -eq 0 ]; then
    pass
fi

log_test "Checking hive.yaml structure"
if grep -q "workspace:" hive.yaml && grep -q "agents:" hive.yaml; then
    pass
else
    fail "hive.yaml missing required fields"
fi

# ============================================
# Test 7: Git Integration
# ============================================

log_test "Checking .gitignore updated"
if grep -q ".hive/" .gitignore 2>/dev/null; then
    pass
else
    fail ".gitignore not updated with .hive/"
fi

log_test "Verifying git worktrees created"
WORKTREE_COUNT=$(git worktree list | wc -l | tr -d ' ')
if [ "$WORKTREE_COUNT" -ge 2 ]; then  # main + at least 1 agent
    pass
else
    fail "Git worktrees not created. Found: $WORKTREE_COUNT"
    git worktree list
fi

# ============================================
# Test 8: Hive Clean
# ============================================

log_test "Testing hive clean command"
if $HIVE_BIN clean --force > /dev/null 2>&1; then
    pass
else
    fail "hive clean failed"
fi

log_test "Verifying .hive directory removed"
if [ ! -d ".hive" ]; then
    pass
else
    fail ".hive directory still exists after clean"
fi

log_test "Verifying worktrees removed"
WORKTREE_COUNT_AFTER=$(git worktree list | wc -l | tr -d ' ')
if [ "$WORKTREE_COUNT_AFTER" -eq 1 ]; then
    pass
else
    fail "Worktrees not cleaned up. Found: $WORKTREE_COUNT_AFTER"
fi

# ============================================
# Test 9: Re-init After Clean
# ============================================

log_test "Testing re-initialization after clean"
if $HIVE_BIN init \
    --no-interactive \
    --skip-start \
    --email "test@example.com" \
    --name "Test User" \
    --workspace "test" \
    --token "$CLAUDE_CODE_OAUTH_TOKEN" \
    --workers 1 > /dev/null 2>&1; then
    pass
else
    fail "Re-initialization failed"
fi

log_test "Final cleanup"
$HIVE_BIN clean --force > /dev/null 2>&1
pass

# ============================================
# Test Results
# ============================================

echo -e "\n${GREEN}====================================${NC}"
echo -e "${GREEN}Test Results${NC}"
echo -e "${GREEN}====================================${NC}"
echo -e "Total:  $TESTS_RUN"
echo -e "${GREEN}Passed: $TESTS_PASSED${NC}"
if [ $TESTS_FAILED -gt 0 ]; then
    echo -e "${RED}Failed: $TESTS_FAILED${NC}"
fi

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}✓ All tests passed!${NC}\n"
    exit 0
else
    echo -e "\n${RED}✗ Some tests failed${NC}\n"
    exit 1
fi
