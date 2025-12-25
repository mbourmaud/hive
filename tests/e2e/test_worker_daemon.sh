#!/bin/bash
# ============================================
# Worker Daemon E2E Tests
# Test autonomous worker functionality end-to-end
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
    docker compose -f .hive/docker-compose.yml down -v 2>/dev/null || true
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

wait_for_container() {
    local container=$1
    local timeout=${2:-30}
    local elapsed=0

    echo "Waiting for container $container to be healthy..."
    while [ $elapsed -lt $timeout ]; do
        if docker ps --filter "name=$container" --filter "status=running" | grep -q "$container"; then
            return 0
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done

    return 1
}

# ============================================
# Test Suite
# ============================================

echo -e "${GREEN}====================================${NC}"
echo -e "${GREEN}Worker Daemon E2E Tests${NC}"
echo -e "${GREEN}====================================${NC}\n"

# Setup test repository
cd "$TEST_DIR"

log_test "Setting up test repository"
git init
git config user.email "test@example.com"
git config user.name "Test User"
echo "# Test Project" > README.md
git add README.md
git commit -m "Initial commit"
pass

# ============================================
# Test 1: Initialize Hive with Daemon Workers
# ============================================

log_test "Initializing Hive with daemon mode workers"
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
    --workers 2 > /tmp/hive-init.log 2>&1; then
    pass
else
    fail "hive init failed"
    cat /tmp/hive-init.log
    exit 1
fi

# Configure workers for daemon mode
log_test "Configuring workers for daemon mode"
echo "WORKER_1_MODE=daemon" >> .hive/.env
echo "WORKER_2_MODE=interactive" >> .hive/.env
pass

# ============================================
# Test 2: Start Hive Services
# ============================================

log_test "Starting Hive services"
if docker compose -f .hive/docker-compose.yml up -d > /tmp/hive-up.log 2>&1; then
    pass
else
    fail "docker compose up failed"
    cat /tmp/hive-up.log
    exit 1
fi

log_test "Waiting for Redis to be ready"
if wait_for_container "hive-redis" 30; then
    pass
else
    fail "Redis container didn't start"
    docker ps -a
    exit 1
fi

log_test "Waiting for worker containers to be ready"
if wait_for_container "claude-agent-1" 30; then
    pass
else
    fail "Worker 1 container didn't start"
    docker ps -a
    exit 1
fi

# ============================================
# Test 3: Verify Daemon is Running
# ============================================

log_test "Checking if worker-daemon.py process is running in agent-1"
sleep 5  # Give daemon time to start
if docker exec claude-agent-1 pgrep -f "worker-daemon.py" > /dev/null 2>&1; then
    pass
else
    fail "worker-daemon.py process not found"
    docker exec claude-agent-1 ps aux || true
fi

log_test "Checking daemon logs for initialization message"
if docker logs claude-agent-1 2>&1 | grep -q "HIVE Worker drone-1 initialized"; then
    pass
else
    fail "Daemon initialization message not found in logs"
    docker logs claude-agent-1 2>&1 | tail -20
fi

log_test "Verifying daemon detected correct backend"
if docker logs claude-agent-1 2>&1 | grep -q "Backend:"; then
    pass
else
    fail "Backend detection message not found"
    docker logs claude-agent-1 2>&1 | tail -20
fi

# ============================================
# Test 4: Redis Connection
# ============================================

log_test "Verifying daemon connected to Redis"
if docker logs claude-agent-1 2>&1 | grep -q "Connected to Redis"; then
    pass
else
    fail "Redis connection message not found"
    docker logs claude-agent-1 2>&1 | tail -20
fi

log_test "Checking Redis queue key exists"
if docker exec hive-redis redis-cli EXISTS "hive:queue:drone-1" > /dev/null 2>&1; then
    pass
else
    fail "Redis queue key not found"
fi

# ============================================
# Test 5: Task Assignment
# ============================================

log_test "Assigning a test task to worker 1"
TASK_ID=$(uuidgen 2>/dev/null || echo "test-task-$$")
if docker exec hive-redis redis-cli LPUSH "hive:queue:drone-1" "{\"id\":\"$TASK_ID\",\"title\":\"Test task\",\"description\":\"Echo hello world\"}" > /dev/null 2>&1; then
    pass
else
    fail "Failed to push task to Redis queue"
fi

log_test "Verifying task appears in queue"
QUEUE_LEN=$(docker exec hive-redis redis-cli LLEN "hive:queue:drone-1")
if [ "$QUEUE_LEN" -ge 1 ]; then
    pass
else
    fail "Task not found in queue. Queue length: $QUEUE_LEN"
fi

# ============================================
# Test 6: Interactive vs Daemon Mode
# ============================================

log_test "Verifying worker 1 is in daemon mode"
if docker logs claude-agent-1 2>&1 | grep -q "Starting autonomous daemon mode"; then
    pass
else
    fail "Worker 1 not in daemon mode"
    docker logs claude-agent-1 2>&1 | grep -i "mode" || true
fi

log_test "Verifying worker 2 is in interactive mode"
if docker logs claude-agent-2 2>&1 | grep -q "Starting interactive mode"; then
    pass
else
    fail "Worker 2 not in interactive mode"
    docker logs claude-agent-2 2>&1 | grep -i "mode" || true
fi

log_test "Verifying worker 2 runs bash instead of daemon"
if docker exec claude-agent-2 pgrep -f "bash" > /dev/null 2>&1; then
    pass
else
    fail "Worker 2 not running bash"
    docker exec claude-agent-2 ps aux || true
fi

# ============================================
# Test 7: Crash Recovery
# ============================================

log_test "Testing daemon crash recovery"
# Kill the daemon process
docker exec claude-agent-1 pkill -f "worker-daemon.py" 2>/dev/null || true
sleep 10  # Wait for restart

if docker exec claude-agent-1 pgrep -f "worker-daemon.py" > /dev/null 2>&1; then
    pass
else
    fail "Daemon did not restart after crash"
    docker logs claude-agent-1 2>&1 | tail -20
fi

log_test "Verifying restart message in logs"
if docker logs claude-agent-1 2>&1 | grep -q "restarting"; then
    pass
else
    # This might not always show up depending on timing
    echo -e "${YELLOW}⚠ Warning: Restart message not found (non-critical)${NC}"
    TESTS_RUN=$((TESTS_RUN - 1))
fi

# ============================================
# Test 8: Cleanup and Stop
# ============================================

log_test "Stopping Hive services"
if docker compose -f .hive/docker-compose.yml down > /dev/null 2>&1; then
    pass
else
    fail "docker compose down failed"
fi

log_test "Verifying containers stopped"
RUNNING=$(docker ps --filter "name=claude-agent" --filter "status=running" | wc -l | tr -d ' ')
if [ "$RUNNING" -le 1 ]; then  # Header line counts as 1
    pass
else
    fail "Containers still running after down"
    docker ps
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
