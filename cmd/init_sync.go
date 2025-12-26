package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// syncHiveYAML copies hive.yaml to .hive/hive.yaml
func syncHiveYAML() error {
	src := "hive.yaml"
	dst := filepath.Join(".hive", "hive.yaml")

	// Read source file
	data, err := os.ReadFile(src)
	if err != nil {
		return fmt.Errorf("failed to read %s: %w", src, err)
	}

	// Write to destination
	if err := os.WriteFile(dst, data, 0644); err != nil {
		return fmt.Errorf("failed to write %s: %w", dst, err)
	}

	return nil
}

// syncHostMCPs copies ~/.claude/settings.json to .hive/host-mcps.json
// This allows containers to access host MCPs without individual file mounts
func syncHostMCPs() error {
	home, err := os.UserHomeDir()
	if err != nil {
		return fmt.Errorf("failed to get home directory: %w", err)
	}

	src := filepath.Join(home, ".claude", "settings.json")
	dst := filepath.Join(".hive", "host-mcps.json")

	// Read source file (may not exist, that's OK)
	data, err := os.ReadFile(src)
	if err != nil {
		if os.IsNotExist(err) {
			// No host settings, create empty JSON
			data = []byte("{}")
		} else {
			return fmt.Errorf("failed to read %s: %w", src, err)
		}
	}

	// Write to destination
	if err := os.WriteFile(dst, data, 0644); err != nil {
		return fmt.Errorf("failed to write %s: %w", dst, err)
	}

	return nil
}

// syncProjectCLAUDEmd copies CLAUDE.md to .hive/CLAUDE.md
// This allows containers to access project guidelines
func syncProjectCLAUDEmd() error {
	src := "CLAUDE.md"
	dst := filepath.Join(".hive", "CLAUDE.md")

	// Read source file (may not exist, that's OK)
	data, err := os.ReadFile(src)
	if err != nil {
		if os.IsNotExist(err) {
			// No CLAUDE.md in project, skip
			return nil
		}
		return fmt.Errorf("failed to read %s: %w", src, err)
	}

	// Write to destination
	if err := os.WriteFile(dst, data, 0644); err != nil {
		return fmt.Errorf("failed to write %s: %w", dst, err)
	}

	return nil
}

// updateGitignore adds hive-specific entries to .gitignore
func updateGitignore() error {
	entries := []string{
		"",
		"# Hive (multi-agent Claude)",
		".hive/",
	}

	gitignorePath := ".gitignore"
	var content string

	// Read existing .gitignore if it exists
	if data, err := os.ReadFile(gitignorePath); err == nil {
		content = string(data)
	}

	// Check if hive entries already exist
	if strings.Contains(content, ".hive/") {
		return nil // Already configured
	}

	// Append hive entries
	content += strings.Join(entries, "\n") + "\n"

	return os.WriteFile(gitignorePath, []byte(content), 0644)
}
