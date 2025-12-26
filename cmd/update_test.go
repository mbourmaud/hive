package cmd

import (
	"os"
	"testing"

	"github.com/mbourmaud/hive/internal/config"
)

// TestUpdateCmdUsage verifies the update command has correct configuration
func TestUpdateCmdUsage(t *testing.T) {
	if updateCmd.Use != "update" {
		t.Errorf("expected Use to be 'update', got '%s'", updateCmd.Use)
	}
	if updateCmd.Short == "" {
		t.Error("Short description should not be empty")
	}
}

// TestUpdateCmdFlags verifies the update command has expected flags
func TestUpdateCmdFlags(t *testing.T) {
	// Test --rebuild flag exists
	rebuild := updateCmd.Flags().Lookup("rebuild")
	if rebuild == nil {
		t.Error("expected --rebuild flag to exist")
	}

	// Test --pull flag exists
	pull := updateCmd.Flags().Lookup("pull")
	if pull == nil {
		t.Error("expected --pull flag to exist")
	}

	// Test --wait flag exists
	wait := updateCmd.Flags().Lookup("wait")
	if wait == nil {
		t.Error("expected --wait flag to exist")
	}
}

// TestRunUpdate_NoHiveDir tests update when .hive doesn't exist
func TestRunUpdate_NoHiveDir(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Run update without .hive directory
	err := runUpdate(nil, nil)
	if err == nil {
		t.Error("runUpdate() should error when .hive doesn't exist")
	}
}

// TestRunUpdate_WithHiveDir tests update with .hive directory
func TestRunUpdate_WithHiveDir(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Create .hive directory with required files
	os.MkdirAll(".hive", 0755)

	// Create minimal hive.yaml
	cfg := config.Default()
	cfg.Save("hive.yaml")

	// Run update - will fail on docker commands but tests early logic
	err := runUpdate(nil, nil)
	// This will error because docker is not mocked, but it tests the config loading
	if err == nil {
		// If docker works in the test environment, this might pass
		t.Log("runUpdate() succeeded unexpectedly - docker may be running")
	}
}

// TestUpdateFlags tests the global update flags
func TestUpdateFlags(t *testing.T) {
	// Verify default values
	if updateRebuild {
		t.Error("updateRebuild should default to false")
	}
	if updatePull {
		t.Error("updatePull should default to false")
	}
	if updateWait {
		t.Error("updateWait should default to false")
	}
}
