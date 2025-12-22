# Homebrew Tap Setup

How to create and maintain the Homebrew tap for Hive.

## Initial Setup

### 1. Create Tap Repository

```bash
# Create homebrew-tap repo on GitHub
gh repo create mbourmaud/homebrew-tap --public --description "Homebrew formulae for Hive"

# Clone it
git clone https://github.com/mbourmaud/homebrew-tap.git
cd homebrew-tap
```

### 2. Create Formula Directory

```bash
mkdir -p Formula
cd Formula
```

### 3. Create Formula

Create `Formula/hive.rb`:

```ruby
class Hive < Formula
  desc "Multi-Agent Claude System for parallel development"
  homepage "https://github.com/mbourmaud/hive"
  version "0.2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/mbourmaud/hive/releases/download/v0.2.0/hive-darwin-arm64.tar.gz"
      sha256 "<checksum-here>"
    else
      url "https://github.com/mbourmaud/hive/releases/download/v0.2.0/hive-darwin-amd64.tar.gz"
      sha256 "<checksum-here>"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/mbourmaud/hive/releases/download/v0.2.0/hive-linux-arm64.tar.gz"
      sha256 "<checksum-here>"
    else
      url "https://github.com/mbourmaud/hive/releases/download/v0.2.0/hive-linux-amd64.tar.gz"
      sha256 "<checksum-here>"
    end
  end

  depends_on "docker" => :recommended

  def install
    bin.install Dir["hive-*"].first => "hive"
  end

  test do
    assert_match "hive", shell_output("#{bin}/hive --help")
  end
end
```

### 4. Get Checksums

```bash
# Download all release assets
gh release download v0.2.0 -R mbourmaud/hive

# Get checksums (already provided in .sha256 files)
cat hive-darwin-arm64.tar.gz.sha256
cat hive-darwin-amd64.tar.gz.sha256
cat hive-linux-arm64.tar.gz.sha256
cat hive-linux-amd64.tar.gz.sha256

# Update formula with checksums
```

### 5. Test Formula

```bash
# Test installation
brew install --build-from-source Formula/hive.rb

# Test command
hive --help

# Uninstall
brew uninstall hive
```

### 6. Commit and Push

```bash
git add Formula/hive.rb
git commit -m "feat: add hive formula v0.2.0"
git push origin main
```

## Usage

Once published:

```bash
# Add tap
brew tap mbourmaud/tap

# Install
brew install hive

# Verify
hive --version

# Update
brew upgrade hive
```

## Updating Formula

### Manual Update (for each release)

```bash
cd homebrew-tap

# Update version and URLs in Formula/hive.rb
vim Formula/hive.rb

# Update checksums
gh release download v0.3.0 -R mbourmaud/hive
cat hive-darwin-arm64.tar.gz.sha256  # Copy to formula
cat hive-darwin-amd64.tar.gz.sha256  # etc.

# Test
brew reinstall --build-from-source Formula/hive.rb

# Commit
git add Formula/hive.rb
git commit -m "feat: update hive to v0.3.0"
git push
```

### Automated Update (GitHub Actions)

Add to `hive/.github/workflows/release.yml`:

```yaml
  update-homebrew:
    name: Update Homebrew Formula
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Checkout tap
        uses: actions/checkout@v4
        with:
          repository: mbourmaud/homebrew-tap
          token: ${{ secrets.TAP_GITHUB_TOKEN }}

      - name: Update formula
        run: |
          VERSION=${GITHUB_REF#refs/tags/v}

          # Download checksums
          gh release download v$VERSION -R mbourmaud/hive -p "*.sha256"

          # Update formula
          cat > Formula/hive.rb << 'EOF'
          class Hive < Formula
            desc "Multi-Agent Claude System for parallel development"
            homepage "https://github.com/mbourmaud/hive"
            version "$VERSION"

            on_macos do
              if Hardware::CPU.arm?
                url "https://github.com/mbourmaud/hive/releases/download/v$VERSION/hive-darwin-arm64.tar.gz"
                sha256 "$(cat hive-darwin-arm64.tar.gz.sha256 | cut -d' ' -f1)"
              else
                url "https://github.com/mbourmaud/hive/releases/download/v$VERSION/hive-darwin-amd64.tar.gz"
                sha256 "$(cat hive-darwin-amd64.tar.gz.sha256 | cut -d' ' -f1)"
              end
            end

            on_linux do
              if Hardware::CPU.arm?
                url "https://github.com/mbourmaud/hive/releases/download/v$VERSION/hive-linux-arm64.tar.gz"
                sha256 "$(cat hive-linux-arm64.tar.gz.sha256 | cut -d' ' -f1)"
              else
                url "https://github.com/mbourmaud/hive/releases/download/v$VERSION/hive-linux-amd64.tar.gz"
                sha256 "$(cat hive-linux-amd64.tar.gz.sha256 | cut -d' ' -f1)"
              end
            end

            depends_on "docker" => :recommended

            def install
              bin.install Dir["hive-*"].first => "hive"
            end

            test do
              assert_match "hive", shell_output("#{bin}/hive --help")
            end
          end
          EOF

          git config user.name "GitHub Actions"
          git config user.email "actions@github.com"
          git add Formula/hive.rb
          git commit -m "chore: update hive to v$VERSION"
          git push
```

## Troubleshooting

### Checksum Mismatch

```bash
# Regenerate checksums
shasum -a 256 hive-darwin-arm64.tar.gz

# Update in formula
```

### Formula Syntax Error

```bash
# Validate syntax
brew audit --strict Formula/hive.rb

# Fix errors and retry
```

### Installation Fails

```bash
# Check logs
brew install --verbose --debug hive

# Common issues:
# - Wrong binary name in `bin.install`
# - Missing dependencies
# - Incorrect URL
```

## Alternative: Submit to Homebrew Core

For wider reach, submit to official Homebrew:

```bash
# Fork homebrew-core
gh repo fork Homebrew/homebrew-core

# Create formula
cd homebrew-core
cp path/to/hive.rb Formula/hive.rb

# Test thoroughly
brew install --build-from-source Formula/hive.rb
brew test hive
brew audit --strict Formula/hive.rb

# Submit PR
gh pr create --repo Homebrew/homebrew-core \
  --title "hive 0.2.0 (new formula)" \
  --body "Multi-Agent Claude System for parallel development"
```

## Resources

- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Homebrew Acceptable Formulae](https://docs.brew.sh/Acceptable-Formulae)
- [Example Tap](https://github.com/Homebrew/homebrew-core/tree/master/Formula)
