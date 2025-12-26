package cmd

import (
	"os"
	"testing"

	"github.com/mbourmaud/hive/internal/config"
)

// TestConfigCmdUsage verifies the config command has correct configuration
func TestConfigCmdUsage(t *testing.T) {
	if configCmd.Use != "config" {
		t.Errorf("expected Use to be 'config', got '%s'", configCmd.Use)
	}
	if configCmd.Short == "" {
		t.Error("Short description should not be empty")
	}
}

// TestConfigShowCmd tests the config show subcommand
func TestConfigShowCmd(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Test without hive.yaml - should use defaults
	err := configShowCmd.RunE(nil, nil)
	if err != nil {
		t.Fatalf("configShowCmd.RunE() error = %v", err)
	}

	// Test with hive.yaml
	cfg := config.Default()
	cfg.Workspace.Name = "test-project"
	cfg.Save("hive.yaml")

	err = configShowCmd.RunE(nil, nil)
	if err != nil {
		t.Fatalf("configShowCmd.RunE() with hive.yaml error = %v", err)
	}
}

// TestConfigValidateCmd tests the config validate subcommand
func TestConfigValidateCmd(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Test without any files - should fail
	err := configValidateCmd.RunE(nil, nil)
	if err == nil {
		t.Error("configValidateCmd.RunE() should error when files are missing")
	}

	// Create required files
	cfg := config.Default()
	cfg.Save("hive.yaml")
	os.WriteFile(".env", []byte("TEST=value"), 0644)
	os.WriteFile("docker-compose.yml", []byte("version: '3'"), 0644)

	// Should pass now
	err = configValidateCmd.RunE(nil, nil)
	if err != nil {
		t.Errorf("configValidateCmd.RunE() error = %v", err)
	}
}

// TestConfigValidateCmd_InvalidHiveYAML tests with invalid hive.yaml
func TestConfigValidateCmd_InvalidHiveYAML(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Create invalid hive.yaml
	os.WriteFile("hive.yaml", []byte("invalid: {{{yaml"), 0644)
	os.WriteFile(".env", []byte("TEST=value"), 0644)
	os.WriteFile("docker-compose.yml", []byte("version: '3'"), 0644)

	err := configValidateCmd.RunE(nil, nil)
	if err == nil {
		t.Error("configValidateCmd.RunE() should error with invalid YAML")
	}
}

// TestConfigPathCmd tests the config path subcommand
func TestConfigPathCmd(t *testing.T) {
	// This command should not panic or error
	configPathCmd.Run(nil, nil)
}

// TestConfigPathCmd_Different Directory tests path in different directory
func TestConfigPathCmd_DifferentDirectory(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Should work in any directory
	configPathCmd.Run(nil, nil)
}

// TestConfigSubcommands verifies all subcommands exist
func TestConfigSubcommands(t *testing.T) {
	subcommands := []string{"show", "validate", "path"}

	for _, name := range subcommands {
		found := false
		for _, cmd := range configCmd.Commands() {
			if cmd.Use == name {
				found = true
				break
			}
		}
		if !found {
			t.Errorf("configCmd missing subcommand: %s", name)
		}
	}
}
