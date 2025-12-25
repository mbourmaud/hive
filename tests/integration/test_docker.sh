#!/bin/bash
# ============================================
# Docker Integration Tests
# Verify all required files are copied to containers
# ============================================

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

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

echo -e "${GREEN}====================================${NC}"
echo -e "${GREEN}Docker Integration Tests${NC}"
echo -e "${GREEN}====================================${NC}\n"

# ============================================
# Test 1: Build Docker Image
# ============================================

log_test "Building Docker image from Dockerfile.node"
if docker build -f docker/Dockerfile.node -t hive-test:latest . > /tmp/docker-build.log 2>&1; then
    pass
else
    fail "Docker build failed"
    tail -50 /tmp/docker-build.log
    exit 1
fi

# ============================================
# Test 2: Required Files in Container
# ============================================

REQUIRED_FILES=(
    "/home/agent/entrypoint.sh"
    "/home/agent/start-worker.sh"
    "/home/agent/worker-daemon.py"
    "/home/agent/backends.py"
    "/home/agent/tools.py"
    "/home/agent/.claude.json.template"
)

for file in "${REQUIRED_FILES[@]}"; do
    log_test "Checking if $file exists in container"
    if docker run --rm hive-test:latest test -f "$file"; then
        pass
    else
        fail "File not found in container: $file"
    fi
done

# ============================================
# Test 3: File Permissions
# ============================================

EXECUTABLE_FILES=(
    "/home/agent/entrypoint.sh"
    "/home/agent/start-worker.sh"
    "/home/agent/worker-daemon.py"
)

for file in "${EXECUTABLE_FILES[@]}"; do
    log_test "Checking if $file is executable"
    if docker run --rm hive-test:latest test -x "$file"; then
        pass
    else
        fail "File not executable: $file"
    fi
done

# ============================================
# Test 4: Python Module Imports
# ============================================

log_test "Verifying backends.py can be imported"
if docker run --rm hive-test:latest python3 -c "import sys; sys.path.insert(0, '/home/agent'); import backends" 2>&1; then
    pass
else
    fail "backends.py cannot be imported"
fi

log_test "Verifying tools.py can be imported"
if docker run --rm hive-test:latest python3 -c "import sys; sys.path.insert(0, '/home/agent'); import tools" 2>&1; then
    pass
else
    fail "tools.py cannot be imported"
fi

# ============================================
# Test 5: Required Python Packages
# ============================================

REQUIRED_PACKAGES=(
    "redis"
    "anthropic"
)

for package in "${REQUIRED_PACKAGES[@]}"; do
    log_test "Checking if Python package '$package' is installed"
    if docker run --rm hive-test:latest python3 -c "import $package" 2>&1; then
        pass
    else
        fail "Python package not installed: $package"
    fi
done

# ============================================
# Test 6: Node.js and Tools
# ============================================

log_test "Verifying Node.js is installed"
if docker run --rm hive-test:latest node --version > /dev/null 2>&1; then
    pass
else
    fail "Node.js not installed"
fi

log_test "Verifying pnpm is installed"
if docker run --rm hive-test:latest pnpm --version > /dev/null 2>&1; then
    pass
else
    fail "pnpm not installed"
fi

log_test "Verifying claude CLI is installed"
if docker run --rm hive-test:latest claude --version > /dev/null 2>&1; then
    pass
else
    fail "claude CLI not installed"
fi

# ============================================
# Test 7: Git Configuration
# ============================================

log_test "Verifying git is installed"
if docker run --rm hive-test:latest git --version > /dev/null 2>&1; then
    pass
else
    fail "git not installed"
fi

log_test "Verifying gh (GitHub CLI) is installed"
if docker run --rm hive-test:latest gh --version > /dev/null 2>&1; then
    pass
else
    fail "gh CLI not installed"
fi

# ============================================
# Test 8: Directory Structure
# ============================================

REQUIRED_DIRS=(
    "/home/agent/.claude"
    "/home/agent/.local/share/pnpm"
    "/home/agent/node_modules_cache"
    "/scripts/redis"
)

for dir in "${REQUIRED_DIRS[@]}"; do
    log_test "Checking if directory $dir exists"
    if docker run --rm hive-test:latest test -d "$dir"; then
        pass
    else
        fail "Directory not found: $dir"
    fi
done

# ============================================
# Test 9: HIVE CLI Tools
# ============================================

HIVE_TOOLS=(
    "hive-assign"
    "hive-status"
    "my-tasks"
    "take-task"
    "task-done"
    "task-failed"
)

for tool in "${HIVE_TOOLS[@]}"; do
    log_test "Checking if HIVE tool '$tool' is in PATH"
    if docker run --rm hive-test:latest which "$tool" > /dev/null 2>&1; then
        pass
    else
        fail "HIVE tool not found in PATH: $tool"
    fi
done

# ============================================
# Test 10: User and Permissions
# ============================================

log_test "Verifying container runs as 'agent' user"
CURRENT_USER=$(docker run --rm hive-test:latest whoami)
if [ "$CURRENT_USER" = "agent" ]; then
    pass
else
    fail "Container running as '$CURRENT_USER' instead of 'agent'"
fi

log_test "Verifying agent user has write access to home directory"
if docker run --rm hive-test:latest touch /home/agent/test-write && docker run --rm hive-test:latest rm /home/agent/test-write; then
    pass
else
    fail "agent user cannot write to /home/agent"
fi

# ============================================
# Cleanup
# ============================================

log_test "Cleaning up test image"
if docker rmi hive-test:latest > /dev/null 2>&1; then
    pass
else
    fail "Failed to remove test image"
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
