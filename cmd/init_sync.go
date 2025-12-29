package cmd

import (
	"encoding/json"
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

// syncHostMCPs extracts MCPs from the current project in ~/.claude.json
// and copies them to .hive/host-mcps.json for container access
func syncHostMCPs() error {
	home, err := os.UserHomeDir()
	if err != nil {
		return fmt.Errorf("failed to get home directory: %w", err)
	}

	// Get current project path
	cwd, err := os.Getwd()
	if err != nil {
		return fmt.Errorf("failed to get current directory: %w", err)
	}

	src := filepath.Join(home, ".claude.json")
	dst := filepath.Join(".hive", "host-mcps.json")

	// Read ~/.claude.json (contains project-specific MCPs)
	data, err := os.ReadFile(src)
	if err != nil {
		if os.IsNotExist(err) {
			// No ~/.claude.json, create empty JSON
			return os.WriteFile(dst, []byte("{}"), 0644)
		}
		return fmt.Errorf("failed to read %s: %w", src, err)
	}

	// Parse JSON and extract MCPs for current project
	var claudeJSON map[string]interface{}
	if err := json.Unmarshal(data, &claudeJSON); err != nil {
		// Invalid JSON, create empty
		return os.WriteFile(dst, []byte("{}"), 0644)
	}

	// Look for project MCPs in projects[cwd].mcpServers
	projectMCPs := make(map[string]interface{})

	if projects, ok := claudeJSON["projects"].(map[string]interface{}); ok {
		// Try exact path match first
		if projectConfig, ok := projects[cwd].(map[string]interface{}); ok {
			if mcps, ok := projectConfig["mcpServers"].(map[string]interface{}); ok {
				projectMCPs = mcps
			}
		}

		// Also try /workspace path (for already-running containers)
		if projectConfig, ok := projects["/workspace"].(map[string]interface{}); ok {
			if mcps, ok := projectConfig["mcpServers"].(map[string]interface{}); ok {
				// Merge with existing (project path takes precedence)
				for name, config := range mcps {
					if _, exists := projectMCPs[name]; !exists {
						projectMCPs[name] = config
					}
				}
			}
		}
	}

	// Write project MCPs to destination
	result := map[string]interface{}{
		"mcpServers": projectMCPs,
	}

	output, err := json.MarshalIndent(result, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal MCPs: %w", err)
	}

	if err := os.WriteFile(dst, output, 0644); err != nil {
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
		"# Hive (multi-agent Claude Code)",
		".hive/",
		"hive.yaml",
	}

	gitignorePath := ".gitignore"
	var content string

	// Read existing .gitignore if it exists
	if data, err := os.ReadFile(gitignorePath); err == nil {
		content = string(data)
	}

	// Check if hive entries already exist
	if strings.Contains(content, ".hive/") && strings.Contains(content, "hive.yaml") {
		return nil // Already configured
	}

	// Append hive entries
	content += strings.Join(entries, "\n") + "\n"

	return os.WriteFile(gitignorePath, []byte(content), 0644)
}
