#!/bin/bash
# ============================================
# Git and Worktree Integration Tests
# Critical tests for Hive multi-agent system
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
    echo -e "${YELLOW}Cleaning up test directory...${NC}"
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

assert_equals() {
    local expected="$1"
    local actual="$2"
    local message="${3:-}"

    if [ "$expected" = "$actual" ]; then
        pass
    else
        fail "Expected '$expected', got '$actual'. $message"
    fi
}

assert_file_exists() {
    if [ -f "$1" ]; then
        pass
    else
        fail "File not found: $1"
    fi
}

assert_dir_exists() {
    if [ -d "$1" ]; then
        pass
    else
        fail "Directory not found: $1"
    fi
}

assert_command_success() {
    if eval "$1" > /dev/null 2>&1; then
        pass
    else
        fail "Command failed: $1"
    fi
}

# ============================================
# Test Suite
# ============================================

echo -e "${GREEN}====================================${NC}"
echo -e "${GREEN}Git and Worktree Integration Tests${NC}"
echo -e "${GREEN}====================================${NC}\n"

# Setup test repository
cd "$TEST_DIR"

log_test "Creating test git repository"
git init
git config user.email "test@example.com"
git config user.name "Test User"
echo "# Test Project" > README.md
git add README.md
git commit -m "Initial commit"
assert_command_success "git log --oneline | grep -q 'Initial commit'"

# Create some test files to simulate a real project
log_test "Creating realistic project structure"
mkdir -p src tests docs
echo "console.log('Hello');" > src/index.js
echo '{"name": "test-project"}' > package.json
git add src package.json
git commit -m "Add project files"
assert_file_exists "src/index.js"

# ============================================
# Test 1: Hive Initialization
# ============================================

log_test "Initializing Hive with git repository"
export CLAUDE_CODE_OAUTH_TOKEN="test-token-$(date +%s)"
export GIT_USER_EMAIL="test@example.com"
export GIT_USER_NAME="Test User"
export WORKSPACE_NAME="test-project"

if $HIVE_BIN init \
    --no-interactive \
    --skip-start \
    --email "$GIT_USER_EMAIL" \
    --name "$GIT_USER_NAME" \
    --workspace "$WORKSPACE_NAME" \
    --token "$CLAUDE_CODE_OAUTH_TOKEN" \
    --workers 2 2>&1 | tee /tmp/hive-init.log; then
    pass
else
    fail "hive init failed"
    cat /tmp/hive-init.log
fi

# ============================================
# Test 2: Worktree Creation
# ============================================

log_test "Verifying worktree for queen was created"
assert_dir_exists ".hive/workspaces/queen"

log_test "Verifying worktree for drone-1 was created"
assert_dir_exists ".hive/workspaces/drone-1"

log_test "Verifying worktree for drone-2 was created"
assert_dir_exists ".hive/workspaces/drone-2"

log_test "Checking git worktree list"
WORKTREE_COUNT=$(git worktree list | wc -l | tr -d ' ')
if [ "$WORKTREE_COUNT" -ge 4 ]; then  # main + queen + 2 workers
    pass
else
    fail "Expected at least 4 worktrees (main + 3 agents), found $WORKTREE_COUNT"
    git worktree list
fi

# ============================================
# Test 3: Worktree Isolation
# ============================================

log_test "Verifying each worktree has project files"
assert_file_exists ".hive/workspaces/queen/$WORKSPACE_NAME/src/index.js"
assert_file_exists ".hive/workspaces/drone-1/$WORKSPACE_NAME/src/index.js"
assert_file_exists ".hive/workspaces/drone-2/$WORKSPACE_NAME/src/index.js"

log_test "Verifying worktrees can make independent changes"
# Make change in queen workspace
echo "// Queen's change" >> ".hive/workspaces/queen/$WORKSPACE_NAME/src/index.js"
QUEEN_CONTENT=$(cat ".hive/workspaces/queen/$WORKSPACE_NAME/src/index.js")

# Make different change in drone-1 workspace
echo "// Drone-1's change" >> ".hive/workspaces/drone-1/$WORKSPACE_NAME/src/index.js"
DRONE1_CONTENT=$(cat ".hive/workspaces/drone-1/$WORKSPACE_NAME/src/index.js")

# Verify they're different
if [ "$QUEEN_CONTENT" != "$DRONE1_CONTENT" ]; then
    pass
else
    fail "Worktrees should have independent working directories"
fi

# Verify main workspace is unchanged
MAIN_CONTENT=$(cat "src/index.js")
if ! echo "$MAIN_CONTENT" | grep -q "Queen's change"; then
    pass
else
    fail "Main workspace should be isolated from worktree changes"
fi

# ============================================
# Test 4: Git Operations in Worktrees
# ============================================

log_test "Verifying git status works in queen worktree"
cd ".hive/workspaces/queen/$WORKSPACE_NAME"
if git status > /dev/null 2>&1; then
    pass
else
    fail "git status failed in queen worktree"
fi

log_test "Verifying git commit works in queen worktree"
git add src/index.js
if git commit -m "Queen's commit" > /dev/null 2>&1; then
    pass
else
    fail "git commit failed in queen worktree"
fi

log_test "Verifying commit is on current branch"
COMMIT_COUNT=$(git log --oneline | grep -c "Queen's commit" || true)
if [ "$COMMIT_COUNT" -eq 1 ]; then
    pass
else
    fail "Commit not found in worktree git log"
fi

cd "$TEST_DIR"

log_test "Verifying queen's commit is visible in main repo"
MAIN_COMMIT_COUNT=$(git log --all --oneline | grep -c "Queen's commit" || true)
if [ "$MAIN_COMMIT_COUNT" -eq 1 ]; then
    pass
else
    fail "Queen's commit not visible in main repository"
    git log --all --oneline
fi

# ============================================
# Test 5: Multiple Agents on Same Branch
# ============================================

log_test "Verifying multiple agents can work on same branch"
# All worktrees should be on main branch (detached)
QUEEN_BRANCH=$(cd ".hive/workspaces/queen/$WORKSPACE_NAME" && git rev-parse --abbrev-ref HEAD)
DRONE1_BRANCH=$(cd ".hive/workspaces/drone-1/$WORKSPACE_NAME" && git rev-parse --abbrev-ref HEAD)
DRONE2_BRANCH=$(cd ".hive/workspaces/drone-2/$WORKSPACE_NAME" && git rev-parse --abbrev-ref HEAD)

# Worktrees are detached, so HEAD will be returned
# Verify they're all on the same commit
QUEEN_COMMIT=$(cd ".hive/workspaces/queen/$WORKSPACE_NAME" && git rev-parse HEAD)
DRONE1_COMMIT=$(cd ".hive/workspaces/drone-1/$WORKSPACE_NAME" && git rev-parse HEAD)
DRONE2_COMMIT=$(cd ".hive/workspaces/drone-2/$WORKSPACE_NAME" && git rev-parse HEAD)

if [ "$QUEEN_COMMIT" = "$DRONE1_COMMIT" ]; then
    pass
else
    fail "Queen and Drone-1 should start from same commit. Queen: $QUEEN_COMMIT, Drone-1: $DRONE1_COMMIT"
fi

# ============================================
# Test 6: Worktree Cleanup
# ============================================

log_test "Testing worktree cleanup (hive clean)"
cd "$TEST_DIR"

# Store worktree count before clean
WORKTREES_BEFORE=$(git worktree list | wc -l | tr -d ' ')

if $HIVE_BIN clean --force 2>&1 | tee /tmp/hive-clean.log; then
    pass
else
    fail "hive clean failed"
    cat /tmp/hive-clean.log
fi

log_test "Verifying worktrees were removed"
# After clean, should only have main worktree
WORKTREES_AFTER=$(git worktree list | wc -l | tr -d ' ')
if [ "$WORKTREES_AFTER" -eq 1 ]; then
    pass
else
    fail "Expected 1 worktree after clean, found $WORKTREES_AFTER"
    git worktree list
fi

log_test "Verifying .hive directory was removed"
if [ ! -d ".hive" ]; then
    pass
else
    fail ".hive directory still exists after clean"
fi

# ============================================
# Test 7: Re-initialization
# ============================================

log_test "Testing re-initialization after clean"
if $HIVE_BIN init \
    --no-interactive \
    --email "$GIT_USER_EMAIL" \
    --name "$GIT_USER_NAME" \
    --workspace "$WORKSPACE_NAME" \
    --token "$CLAUDE_CODE_OAUTH_TOKEN" \
    --workers 2 > /dev/null 2>&1; then
    pass
else
    fail "Re-initialization failed after clean"
fi

log_test "Verifying worktrees recreated correctly"
NEW_WORKTREE_COUNT=$(git worktree list | wc -l | tr -d ' ')
if [ "$NEW_WORKTREE_COUNT" -eq "$WORKTREES_BEFORE" ]; then
    pass
else
    fail "Expected $WORKTREES_BEFORE worktrees, found $NEW_WORKTREE_COUNT"
fi

# ============================================
# Test 8: Git Worktree Pruning
# ============================================

log_test "Testing orphaned worktree cleanup"
# Manually remove a worktree directory to create orphan
rm -rf ".hive/workspaces/drone-2"

# Prune should happen during next init
git worktree prune

log_test "Verifying git worktree prune removes orphaned entries"
PRUNED_COUNT=$(git worktree list | grep -c "drone-2" || true)
if [ "$PRUNED_COUNT" -eq 0 ]; then
    pass
else
    fail "Orphaned worktree not pruned"
    git worktree list
fi

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
