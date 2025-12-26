package cmd

import (
	"os"
	"strings"
	"testing"

	"github.com/mbourmaud/hive/internal/config"
)

// TestLoadToolsRegistry tests loading the tools registry
func TestLoadToolsRegistry(t *testing.T) {
	// Reset registry before test
	toolsRegistry = nil

	err := loadToolsRegistry()
	if err != nil {
		t.Fatalf("loadToolsRegistry() error = %v", err)
	}

	if toolsRegistry == nil {
		t.Fatal("loadToolsRegistry() did not set toolsRegistry")
	}

	// Check for expected tools in embedded registry
	expectedTools := []string{"glab", "psql", "kubectl", "terraform"}
	for _, tool := range expectedTools {
		if _, ok := toolsRegistry.Tools[tool]; !ok {
			t.Errorf("loadToolsRegistry() missing tool %q", tool)
		}
	}
}

// TestLoadToolsRegistry_Cached tests that registry is cached
func TestLoadToolsRegistry_Cached(t *testing.T) {
	// Reset and load
	toolsRegistry = nil
	loadToolsRegistry()

	// Modify to verify caching
	originalVersion := toolsRegistry.Version
	toolsRegistry.Version = "modified"

	// Load again - should not overwrite
	loadToolsRegistry()

	if toolsRegistry.Version != "modified" {
		t.Error("loadToolsRegistry() should use cached registry")
	}

	// Reset for other tests
	toolsRegistry.Version = originalVersion
}

// TestLoadToolsRegistry_FromFile tests loading from a JSON file
func TestLoadToolsRegistry_FromFile(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Reset registry
	toolsRegistry = nil

	// Create a custom registry file
	registryJSON := `{
		"version": "2.0.0",
		"tools": {
			"custom-tool": {
				"name": "Custom Tool",
				"description": "A custom tool",
				"category": "custom"
			}
		},
		"categories": {
			"custom": "Custom Tools"
		}
	}`
	os.WriteFile("tools-registry.json", []byte(registryJSON), 0644)

	err := loadToolsRegistry()
	if err != nil {
		t.Fatalf("loadToolsRegistry() error = %v", err)
	}

	if toolsRegistry.Version != "2.0.0" {
		t.Errorf("loadToolsRegistry() version = %q, want %q", toolsRegistry.Version, "2.0.0")
	}

	if _, ok := toolsRegistry.Tools["custom-tool"]; !ok {
		t.Error("loadToolsRegistry() missing custom-tool")
	}

	// Reset for other tests
	toolsRegistry = nil
}

// TestAddToolToConfig tests adding a tool to hive.yaml
func TestAddToolToConfig(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Create initial hive.yaml
	cfg := config.Default()
	cfg.Save("hive.yaml")

	// Add a tool
	err := addToolToConfig("kubectl")
	if err != nil {
		t.Fatalf("addToolToConfig() error = %v", err)
	}

	// Verify tool was added
	cfg, _ = config.Load("hive.yaml")
	found := false
	for _, tool := range cfg.Tools {
		if tool == "kubectl" {
			found = true
			break
		}
	}
	if !found {
		t.Error("addToolToConfig() did not add tool to config")
	}
}

// TestAddToolToConfig_Duplicate tests adding a duplicate tool
func TestAddToolToConfig_Duplicate(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Create hive.yaml with existing tool
	cfg := config.Default()
	cfg.Tools = []string{"kubectl"}
	cfg.Save("hive.yaml")

	// Try to add duplicate
	err := addToolToConfig("kubectl")
	if err == nil {
		t.Error("addToolToConfig() should error on duplicate")
	}
}

// TestAddToolToConfig_NoConfig tests adding when no hive.yaml exists
func TestAddToolToConfig_NoConfig(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Add tool without existing config
	err := addToolToConfig("kubectl")
	if err != nil {
		t.Fatalf("addToolToConfig() error = %v", err)
	}

	// Verify config was created
	cfg, err := config.Load("hive.yaml")
	if err != nil {
		t.Fatalf("Failed to load created config: %v", err)
	}

	found := false
	for _, tool := range cfg.Tools {
		if tool == "kubectl" {
			found = true
			break
		}
	}
	if !found {
		t.Error("addToolToConfig() did not create config with tool")
	}
}

// TestRemoveToolFromConfig tests removing a tool from hive.yaml
func TestRemoveToolFromConfig(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Create hive.yaml with tools
	cfg := config.Default()
	cfg.Tools = []string{"kubectl", "helm", "terraform"}
	cfg.Save("hive.yaml")

	// Remove a tool
	err := removeToolFromConfig("helm")
	if err != nil {
		t.Fatalf("removeToolFromConfig() error = %v", err)
	}

	// Verify tool was removed
	cfg, _ = config.Load("hive.yaml")
	for _, tool := range cfg.Tools {
		if tool == "helm" {
			t.Error("removeToolFromConfig() did not remove tool")
		}
	}

	// Verify other tools still exist
	if len(cfg.Tools) != 2 {
		t.Errorf("removeToolFromConfig() expected 2 tools, got %d", len(cfg.Tools))
	}
}

// TestRemoveToolFromConfig_NotFound tests removing a non-existent tool
func TestRemoveToolFromConfig_NotFound(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Create hive.yaml
	cfg := config.Default()
	cfg.Tools = []string{"kubectl"}
	cfg.Save("hive.yaml")

	// Try to remove non-existent tool
	err := removeToolFromConfig("non-existent")
	if err == nil {
		t.Error("removeToolFromConfig() should error on non-existent tool")
	}
}

// TestRemoveToolFromConfig_NoConfig tests removing when no hive.yaml exists
func TestRemoveToolFromConfig_NoConfig(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Try to remove without config
	err := removeToolFromConfig("kubectl")
	if err == nil {
		t.Error("removeToolFromConfig() should error when no config exists")
	}
}

// TestRunToolsReinstall tests the tools reinstall command
func TestRunToolsReinstall(t *testing.T) {
	// Mock docker command runner
	originalRunner := dockerCommandRunner
	defer func() { dockerCommandRunner = originalRunner }()

	// Track calls
	calls := []string{}
	dockerCommandRunner = func(args ...string) (string, error) {
		call := strings.Join(args, " ")
		calls = append(calls, call)

		// Simulate container not running
		if len(args) >= 4 && args[0] == "inspect" {
			return "false", nil
		}
		return "", nil
	}

	// Run the command
	err := runToolsReinstall(nil, nil)
	if err != nil {
		t.Fatalf("runToolsReinstall() error = %v", err)
	}

	// Verify docker inspect was called for containers
	hasInspect := false
	for _, call := range calls {
		if strings.Contains(call, "inspect") {
			hasInspect = true
			break
		}
	}
	if !hasInspect {
		t.Error("runToolsReinstall() should call docker inspect")
	}
}

// TestRunToolsReinstall_WithRunningContainers tests with running containers
func TestRunToolsReinstall_WithRunningContainers(t *testing.T) {
	originalRunner := dockerCommandRunner
	defer func() { dockerCommandRunner = originalRunner }()

	execCalls := []string{}
	dockerCommandRunner = func(args ...string) (string, error) {
		call := strings.Join(args, " ")

		if len(args) >= 4 && args[0] == "inspect" {
			// Only hive-queen is running
			if args[3] == "hive-queen" {
				return "true", nil
			}
			return "false", nil
		}

		if args[0] == "exec" {
			execCalls = append(execCalls, call)
			return "", nil
		}

		return "", nil
	}

	err := runToolsReinstall(nil, nil)
	if err != nil {
		t.Fatalf("runToolsReinstall() error = %v", err)
	}

	// Should have called exec on hive-queen
	if len(execCalls) == 0 {
		t.Error("runToolsReinstall() should call docker exec on running containers")
	}

	foundQueen := false
	for _, call := range execCalls {
		if strings.Contains(call, "hive-queen") {
			foundQueen = true
			break
		}
	}
	if !foundQueen {
		t.Error("runToolsReinstall() should clear cache for hive-queen")
	}
}

// TestRunToolsAdd tests the tools add command
func TestRunToolsAdd(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Reset registry
	toolsRegistry = nil

	// Run add command for a known tool
	err := runToolsAdd(nil, []string{"kubectl"})
	if err != nil {
		t.Fatalf("runToolsAdd() error = %v", err)
	}

	// Verify tool was added to config
	cfg, err := config.Load("hive.yaml")
	if err != nil {
		t.Fatalf("Failed to load config: %v", err)
	}

	found := false
	for _, tool := range cfg.Tools {
		if tool == "kubectl" {
			found = true
			break
		}
	}
	if !found {
		t.Error("runToolsAdd() did not add tool to config")
	}
}

// TestRunToolsAdd_UnknownTool tests adding an unknown tool
func TestRunToolsAdd_UnknownTool(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	toolsRegistry = nil

	// Add unknown tool (should still work with warning)
	err := runToolsAdd(nil, []string{"unknown-tool"})
	if err != nil {
		t.Fatalf("runToolsAdd() should not error for unknown tools: %v", err)
	}

	cfg, _ := config.Load("hive.yaml")
	found := false
	for _, tool := range cfg.Tools {
		if tool == "unknown-tool" {
			found = true
			break
		}
	}
	if !found {
		t.Error("runToolsAdd() should add unknown tool anyway")
	}
}

// TestRunToolsRemove tests the tools remove command
func TestRunToolsRemove(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Create config with a tool
	cfg := config.Default()
	cfg.Tools = []string{"kubectl", "helm"}
	cfg.Save("hive.yaml")

	// Remove one tool
	err := runToolsRemove(nil, []string{"helm"})
	if err != nil {
		t.Fatalf("runToolsRemove() error = %v", err)
	}

	// Verify tool was removed
	cfg, _ = config.Load("hive.yaml")
	for _, tool := range cfg.Tools {
		if tool == "helm" {
			t.Error("runToolsRemove() did not remove tool")
		}
	}
}

// TestRunToolsRemove_NotConfigured tests removing a non-configured tool
func TestRunToolsRemove_NotConfigured(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	cfg := config.Default()
	cfg.Tools = []string{"kubectl"}
	cfg.Save("hive.yaml")

	// Try to remove non-existent tool
	err := runToolsRemove(nil, []string{"not-configured"})
	if err == nil {
		t.Error("runToolsRemove() should error when tool not configured")
	}
}

// TestRunToolsList tests the tools list command
func TestRunToolsList(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Reset registry to use embedded
	toolsRegistry = nil

	// Create config with some tools
	cfg := config.Default()
	cfg.Tools = []string{"kubectl", "psql"}
	cfg.Save("hive.yaml")

	// Run list command (should not error)
	err := runToolsList(nil, nil)
	if err != nil {
		t.Fatalf("runToolsList() error = %v", err)
	}
}

// TestRunToolsList_EmptyConfig tests list with no configured tools
func TestRunToolsList_EmptyConfig(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	toolsRegistry = nil

	// No config file exists
	err := runToolsList(nil, nil)
	if err != nil {
		t.Fatalf("runToolsList() error = %v", err)
	}
}
