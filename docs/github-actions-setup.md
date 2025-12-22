# GitHub Actions Setup

GitHub Actions require the `workflow` scope to be added via git push. There are two ways to add the workflows:

## Option 1: Via GitHub UI (Recommended)

1. Go to https://github.com/mbourmaud/hive
2. Click "Add file" → "Create new file"
3. Create `.github/workflows/test.yml`
4. Copy content from local `.github/workflows/test.yml`
5. Commit directly to main

Repeat for `.github/workflows/release.yml`

## Option 2: Update Git Token

1. Generate new GitHub token with `workflow` scope:
   ```bash
   gh auth refresh -h github.com -s workflow
   ```

2. Push workflows:
   ```bash
   git push origin main
   ```

## Workflows Included

### test.yml
- Runs on every PR and push
- Tests on Ubuntu + macOS
- Validates build

### release.yml
- Triggers on version tags (v*)
- Builds binaries for all platforms
- Creates GitHub release
- Auto-generates changelog

## Testing Workflows

Once added, workflows are in local `.github/workflows/`:

```bash
# Create a test tag
git tag v0.1.0-test
git push origin v0.1.0-test

# Check workflow runs
gh run list
gh run view <run-id>
```

## First Release

After workflows are set up:

```bash
# Tag version
git tag v0.2.0
git push origin v0.2.0

# Check release
gh release view v0.2.0

# Download binary
gh release download v0.2.0
```
