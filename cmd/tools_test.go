package cmd

import (
	"os"
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
