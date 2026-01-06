package cmd

import (
	"encoding/json"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// detectGitConfig retrieves git configuration from the current repository
func detectGitConfig() (email, name, repoURL, workspaceName string) {
	// git config user.email
	if out, err := exec.Command("git", "config", "user.email").Output(); err == nil {
		email = strings.TrimSpace(string(out))
	}

	// git config user.name
	if out, err := exec.Command("git", "config", "user.name").Output(); err == nil {
		name = strings.TrimSpace(string(out))
	}

	// git remote get-url origin
	if out, err := exec.Command("git", "remote", "get-url", "origin").Output(); err == nil {
		repoURL = strings.TrimSpace(string(out))
	}

	// Workspace name from current directory
	if cwd, err := os.Getwd(); err == nil {
		workspaceName = filepath.Base(cwd)
	}

	return
}

// fileExists checks if a file exists
func fileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// detectProjectType detects the project type based on config files
func detectProjectType(dir string) string {
	check := func(name string) bool {
		return fileExists(filepath.Join(dir, name))
	}

	if check("package.json") {
		return "node"
	}
	if check("go.mod") {
		return "go"
	}
	if check("pyproject.toml") || check("requirements.txt") {
		return "python"
	}
	if check("Cargo.toml") {
		return "rust"
	}
	return "unknown"
}

// detectClaudeToken attempts to find Claude OAuth token from existing config
func detectClaudeToken() string {
	home, err := os.UserHomeDir()
	if err != nil {
		return ""
	}

	// Try reading from ~/.claude/settings.json
	settingsPath := filepath.Join(home, ".claude", "settings.json")
	if data, err := os.ReadFile(settingsPath); err == nil {
		var settings map[string]interface{}
		if err := json.Unmarshal(data, &settings); err == nil {
			if oauth, ok := settings["oauthAccount"].(map[string]interface{}); ok {
				if token, ok := oauth["accessToken"].(string); ok && token != "" {
					return token
				}
			}
		}
	}

	// Try environment variable
	if token := os.Getenv("CLAUDE_CODE_OAUTH_TOKEN"); token != "" {
		return token
	}

	return ""
}

// detectAnthropicApiKey attempts to find Anthropic API key from environment
func detectAnthropicApiKey() string {
	return os.Getenv("ANTHROPIC_API_KEY")
}

// detectNodeVersion attempts to detect Node.js version from package.json or .nvmrc
func detectNodeVersion() string {
	// Try package.json first
	if data, err := os.ReadFile("package.json"); err == nil {
		var pkg struct {
			Engines struct {
				Node string `json:"node"`
			} `json:"engines"`
		}
		if err := json.Unmarshal(data, &pkg); err == nil && pkg.Engines.Node != "" {
			// Extract major version number
			// Examples: ">=24.0.0" → "24", "^20.5.0" → "20", "24" → "24"
			nodeVersion := pkg.Engines.Node
			// Remove common prefixes
			nodeVersion = strings.TrimPrefix(nodeVersion, ">=")
			nodeVersion = strings.TrimPrefix(nodeVersion, "^")
			nodeVersion = strings.TrimPrefix(nodeVersion, "~")
			nodeVersion = strings.TrimPrefix(nodeVersion, ">")
			nodeVersion = strings.TrimPrefix(nodeVersion, "<")
			nodeVersion = strings.TrimSpace(nodeVersion)

			// Extract first number sequence (major version)
			parts := strings.Split(nodeVersion, ".")
			if len(parts) > 0 {
				// Remove any non-numeric characters from major version
				major := parts[0]
				var digits strings.Builder
				for _, ch := range major {
					if ch >= '0' && ch <= '9' {
						digits.WriteRune(ch)
					}
				}
				if digits.Len() > 0 {
					return digits.String()
				}
			}
		}
	}

	// Try .nvmrc as fallback
	if data, err := os.ReadFile(".nvmrc"); err == nil {
		version := strings.TrimSpace(string(data))
		// Remove 'v' prefix if present (e.g., "v24.0.0" → "24.0.0")
		version = strings.TrimPrefix(version, "v")
		// Extract major version
		parts := strings.Split(version, ".")
		if len(parts) > 0 {
			var digits strings.Builder
			for _, ch := range parts[0] {
				if ch >= '0' && ch <= '9' {
					digits.WriteRune(ch)
				}
			}
			if digits.Len() > 0 {
				return digits.String()
			}
		}
	}

	return "" // Not found, will use default (22)
}
