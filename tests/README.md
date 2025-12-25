# Hive Test Suite

Comprehensive test suite to ensure reliability and prevent regressions.

## Test Categories

### 1. Smoke Tests (`make test-smoke`)
Quick sanity checks for core functionality.

**What it tests:**
- Basic workflow: `hive init` → `hive status` → `hive clean`
- Configuration file creation (`.env`, `hive.yaml`, `docker-compose.yml`)
- Git integration and `.gitignore` updates
- Git worktree creation and cleanup
- Re-initialization after clean

**Duration:** < 30 seconds
**Runs on:** All OS (Ubuntu, macOS)

```bash
make test-smoke
```

### 2. Docker Integration Tests (`make test-docker`)
Validates Docker image build and container contents.

**What it tests:**
- Docker image builds successfully
- All required files copied to container:
  - `entrypoint.sh`, `start-worker.sh`
  - `worker-daemon.py`, `backends.py`, `tools.py`
  - `.claude.json.template`
- File permissions (executable files)
- Python module imports (`backends`, `tools`)
- Required Python packages (`redis`, `anthropic`)
- Node.js tools (`node`, `pnpm`, `claude`)
- Git tools (`git`, `gh`, `glab`)
- Directory structure
- HIVE CLI tools in PATH
- User permissions

**Duration:** ~2-3 minutes
**Runs on:** Linux (requires Docker)

```bash
make test-docker
```

### 3. Git/Worktree Tests (`make test-git`) ⚠️ CRITICAL
Tests Git worktree functionality - **the most critical feature for multi-agent isolation**.

**What it tests:**
- ✅ Worktree creation for all agents (queen + workers)
- ✅ Worktree isolation (agents can make independent changes)
- ✅ Git operations in worktrees (commit, status, log)
- ✅ Multiple agents working on same branch simultaneously
- ✅ Worktree cleanup (`hive clean` removes all worktrees)
- ✅ Orphaned worktree pruning
- ✅ Re-initialization creates fresh worktrees
- ✅ Commits from worktrees visible in main repo

**Duration:** ~1-2 minutes
**Runs on:** All OS (Ubuntu, macOS)

```bash
make test-git
```

**Why critical?**
- Git worktrees enable agents to work simultaneously without conflicts
- If worktrees break, multi-agent collaboration fails completely
- Worktree bugs cause data loss or merge conflicts

### 4. E2E Worker Daemon Tests (`make test-e2e`)
End-to-end tests for autonomous worker functionality.

**What it tests:**
- Initialize Hive with daemon mode workers
- Start Docker containers
- Verify daemon process running
- Backend auto-detection (API/CLI/Bedrock)
- Redis connection
- Task queue assignment
- Interactive vs daemon mode differences
- Worker crash recovery (auto-restart)
- Container cleanup

**Duration:** ~3-5 minutes
**Runs on:** Linux (requires Docker + Redis)

```bash
make test-e2e
```

## Running Tests

### Run Individual Test Suites

```bash
# Smoke tests (quick sanity checks)
make test-smoke

# Docker integration tests
make test-docker

# Git/worktree tests (CRITICAL)
make test-git

# E2E worker daemon tests
make test-e2e
```

### Run All Tests

```bash
# Run everything (Go unit + all integration/E2E tests)
make test-all
```

### Run Specific Test Categories

```bash
# Go unit tests only
make test

# Integration tests (Docker + Git)
make test-integration
```

## CI/CD Integration

Tests run automatically on every push and pull request via GitHub Actions.

### CI Jobs

1. **unit-tests** - Go unit tests (Ubuntu + macOS)
2. **smoke-tests** - Smoke tests (Ubuntu + macOS)
3. **git-tests** - Git/worktree tests (Ubuntu + macOS)
4. **docker-tests** - Docker integration (Ubuntu only)
5. **e2e-tests** - E2E worker daemon (Ubuntu only)
6. **test-summary** - Aggregates results

### Viewing CI Results

```bash
# Check latest CI run
gh run list --limit 1

# View specific run details
gh run view <run-id>
```

## Test Structure

```
tests/
├── README.md                           # This file
├── smoke/
│   └── test_basic_workflow.sh         # Basic workflow tests
├── integration/
│   ├── test_docker.sh                 # Docker integration tests
│   └── test_git_worktrees.sh          # Git/worktree tests (CRITICAL)
└── e2e/
    └── test_worker_daemon.sh          # E2E worker daemon tests
```

## Writing New Tests

### Test Script Template

```bash
#!/bin/bash
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

# Cleanup
cleanup() {
    # Clean up test resources
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

# Your tests here
log_test "Testing something"
if [ condition ]; then
    pass
else
    fail "Error message"
fi

# Report results
if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}✓ All tests passed!${NC}\n"
    exit 0
else
    echo -e "\n${RED}✗ Some tests failed${NC}\n"
    exit 1
fi
```

### Adding to Makefile

```makefile
test-mynew:
	@echo "Running my new tests..."
	@bash tests/category/test_mynew.sh
```

### Adding to CI

Edit `.github/workflows/test.yml`:

```yaml
mynew-tests:
  name: My New Tests
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Run my tests
      run: make test-mynew
```

## Debugging Failed Tests

### View test output

```bash
# Run with verbose output
bash -x tests/smoke/test_basic_workflow.sh
```

### Common Issues

**Test fails with "hive not found":**
```bash
# Make sure binary is built
make build
export HIVE_BIN=./hive
```

**Docker tests fail:**
```bash
# Ensure Docker is running
docker ps

# Check Docker Buildx
docker buildx version
```

**Git tests fail:**
```bash
# Configure git
git config --global user.email "test@example.com"
git config --global user.name "Test User"
```

## Test Coverage

- **40+ test cases** across all suites
- **Critical paths**: Git worktrees, Docker builds, worker daemon
- **Multi-OS**: Ubuntu + macOS
- **Full stack**: Unit → Integration → E2E

## Performance

| Test Suite | Duration | Runs On |
|------------|----------|---------|
| Smoke | ~30s | All OS |
| Docker | ~2-3min | Linux |
| Git/Worktree | ~1-2min | All OS |
| E2E | ~3-5min | Linux |
| **Total** | **~7-11min** | CI/CD |

## Contributing

When adding new features:

1. **Write tests first** (TDD)
2. **Update existing tests** if behavior changes
3. **Run `make test-all`** before committing
4. **Document test coverage** in PR description

## Questions?

- Check test script comments for detailed documentation
- Run tests with `-x` flag for verbose output
- Open an issue if tests fail unexpectedly
