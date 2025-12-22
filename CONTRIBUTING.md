# Contributing to Hive

Thank you for your interest in contributing to Hive! This document provides guidelines and instructions for contributing.

## Code of Conduct

- Be respectful and inclusive
- Welcome newcomers and help them get started
- Focus on constructive feedback
- Assume good intentions

## Ways to Contribute

### 1. Report Bugs

Found a bug? Please create an issue with:
- Clear, descriptive title
- Steps to reproduce
- Expected vs actual behavior
- Your environment (OS, Docker version, Hive version)
- Relevant logs or screenshots

**Example:**
```markdown
**Bug:** `hive connect queen` fails with "connection refused"

**Steps to reproduce:**
1. Run `hive start 2`
2. Wait for containers to start
3. Run `hive connect queen`

**Expected:** Opens Claude Code session
**Actual:** Error: "connection refused"

**Environment:**
- OS: macOS 14.1
- Docker: 24.0.6
- Hive: v0.2.0

**Logs:**
```
[error] Failed to connect to container hive-queen-1
```
```

### 2. Suggest Features

Have an idea? Create an issue with:
- Clear description of the feature
- Use case / motivation
- Proposed implementation (optional)
- Impact on existing functionality

### 3. Improve Documentation

Documentation improvements are always welcome:
- Fix typos or unclear explanations
- Add examples
- Improve troubleshooting guides
- Translate documentation

### 4. Submit Code

Ready to code? Follow the workflow below.

## Development Workflow

### 1. Fork and Clone

```bash
# Fork the repo on GitHub
gh repo fork mbourmaud/hive --clone

cd hive
```

### 2. Create a Branch

```bash
git checkout -b feature/your-feature-name

# Or for bugs:
git checkout -b fix/bug-description
```

Branch naming:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation
- `refactor/` - Code refactoring
- `test/` - Test improvements

### 3. Make Changes

#### Code Style

**Go:**
```bash
# Format code
go fmt ./...

# Lint
golangci-lint run

# Vet
go vet ./...
```

**Shell scripts:**
```bash
# Use shellcheck
shellcheck scripts/*.sh
```

**Docker:**
- Use official base images
- Minimize layers
- Don't install unnecessary packages
- Pin versions for production images

#### Commit Messages

Follow conventional commits:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `refactor`: Code refactoring
- `test`: Tests
- `chore`: Maintenance

**Examples:**
```bash
feat(cli): add hive logs command

Add new command to view agent logs without connecting.
Supports filtering by agent ID and tail mode.

Closes #42

---

fix(docker): increase shared memory size

Docker containers were failing with "out of memory" when
running Playwright tests. Increase shm_size to 2gb.

Fixes #38

---

docs(examples): add Python ML example

Add comprehensive example for ML projects with parallel
model training workflow.
```

### 4. Test Your Changes

#### Unit Tests

```bash
# Go tests
go test ./...

# With coverage
go test ./... -cover
```

#### Integration Tests

```bash
# Test full workflow
./scripts/test-integration.sh

# Manual test
hive start 2
hive status
hive connect queen
# ... test functionality
hive stop
```

#### Test Checklist

- [ ] Code builds without errors
- [ ] All tests pass
- [ ] No linter warnings
- [ ] Tested on your platform
- [ ] Documentation updated if needed

### 5. Submit Pull Request

```bash
# Push your branch
git push origin feature/your-feature-name

# Create PR
gh pr create \
  --title "feat: add new feature" \
  --body "Description of changes"
```

**PR Description Template:**

```markdown
## What does this PR do?

Brief description of the changes.

## Why?

Motivation and context.

## How?

High-level overview of the implementation.

## Testing

- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Manually tested on [OS]

## Screenshots (if applicable)

[Add screenshots or GIFs]

## Checklist

- [ ] Code follows project style guidelines
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] Commit messages follow conventional commits
- [ ] No breaking changes (or documented)

Closes #[issue number]
```

## Project Structure

```
hive/
├── cmd/               # CLI commands (Go)
│   ├── init.go
│   ├── start.go
│   ├── status.go
│   └── ...
├── docker/            # Dockerfiles
│   ├── Dockerfile.minimal
│   ├── Dockerfile.node
│   └── ...
├── docs/              # Documentation
│   ├── architecture.md
│   ├── mcp-setup.md
│   └── ...
├── examples/          # Example projects
│   ├── nodejs-monorepo/
│   ├── golang-api/
│   └── ...
├── scripts/           # Shell scripts
│   ├── redis/
│   └── ...
├── templates/         # Claude instructions
│   ├── CLAUDE-QUEEN.md
│   └── CLAUDE-WORKER.md
├── main.go            # Entry point
└── docker-compose.yml # Container orchestration
```

## Adding a New Feature

### Example: Add `hive logs` Command

1. **Create command file:**
```go
// cmd/logs.go
package cmd

import (
    "github.com/spf13/cobra"
)

var logsCmd = &cobra.Command{
    Use:   "logs [agent-id]",
    Short: "View agent logs",
    RunE:  runLogs,
}

func init() {
    rootCmd.AddCommand(logsCmd)
    logsCmd.Flags().BoolP("follow", "f", false, "Follow log output")
    logsCmd.Flags().IntP("tail", "n", 50, "Number of lines to show")
}

func runLogs(cmd *cobra.Command, args []string) error {
    // Implementation
    return nil
}
```

2. **Add tests:**
```go
// cmd/logs_test.go
package cmd

import "testing"

func TestLogsCommand(t *testing.T) {
    // Test implementation
}
```

3. **Update documentation:**
```markdown
<!-- README.md -->
### HIVE CLI

```bash
hive logs [agent-id]    # View agent logs
hive logs queen -f      # Follow Queen's logs
```
```

4. **Test:**
```bash
go build
./hive logs queen
```

5. **Create PR**

## Docker Images

When modifying Dockerfiles:

### Testing Locally

```bash
# Build image
docker build -f docker/Dockerfile.node -t hive-test .

# Test image
docker run --rm -it hive-test bash
```

### Size Optimization

```bash
# Check image size
docker images | grep hive

# Analyze layers
docker history hive-test

# Minimize size:
# - Use multi-stage builds
# - Remove build dependencies
# - Use .dockerignore
# - Combine RUN commands
```

### Multi-Platform Support

Test on multiple platforms if possible:
- macOS (Intel and Apple Silicon)
- Linux (Ubuntu, Debian)
- Windows (WSL2)

## Documentation

### Writing Good Documentation

- **Clear and concise**: Avoid jargon, explain complex concepts
- **Examples**: Show don't tell, provide code examples
- **Up to date**: Update docs when changing functionality
- **Searchable**: Use clear headings and keywords

### Documentation Checklist

- [ ] Correct grammar and spelling
- [ ] Code examples are tested
- [ ] Links are valid
- [ ] Screenshots are clear and current
- [ ] Follows existing documentation style

## Release Process

(Maintainers only)

1. **Update version:**
```bash
# Update in main.go
version = "0.3.0"
```

2. **Update CHANGELOG:**
```markdown
## [0.3.0] - 2024-01-15

### Added
- New feature X
- New feature Y

### Fixed
- Bug fix A
- Bug fix B

### Changed
- Changed behavior of Z
```

3. **Create tag:**
```bash
git tag v0.3.0
git push origin v0.3.0
```

4. **GitHub Actions will:**
- Build binaries for all platforms
- Create GitHub release
- Generate release notes
- Upload checksums

5. **Update Homebrew tap:**
```bash
# Update Formula/hive.rb with new version and checksums
cd ../homebrew-tap
# ... update formula
git commit -m "chore: update hive to v0.3.0"
git push
```

## Getting Help

- **Questions:** Open a [discussion](https://github.com/mbourmaud/hive/discussions)
- **Bugs:** Open an [issue](https://github.com/mbourmaud/hive/issues)
- **Chat:** Join our community (coming soon)

## Recognition

Contributors are recognized in:
- GitHub contributors page
- Release notes
- README (for significant contributions)

Thank you for contributing to Hive! 🐝
