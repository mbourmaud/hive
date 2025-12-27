package cmd

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestExposeCmdUsage verifies the expose command has correct configuration
func TestExposeCmdUsage(t *testing.T) {
	if exposeCmd.Use != "expose <agent>" {
		t.Errorf("expected Use to be 'expose <agent>', got '%s'", exposeCmd.Use)
	}
	if exposeCmd.Short == "" {
		t.Error("Short description should not be empty")
	}
	if exposeCmd.Long == "" {
		t.Error("Long description should not be empty")
	}
}

// TestExposeCmdRegistered verifies the expose command is registered with root
func TestExposeCmdRegistered(t *testing.T) {
	found := false
	for _, cmd := range rootCmd.Commands() {
		if cmd.Use == "expose <agent>" {
			found = true
			break
		}
	}

	if !found {
		t.Error("expose command should be registered with root command")
	}
}

// TestExposeCmdFlags verifies the expose command has correct flags
func TestExposeCmdFlags(t *testing.T) {
	portsFlag := exposeCmd.Flags().Lookup("ports")
	if portsFlag == nil {
		t.Error("expose command should have --ports flag")
	} else {
		if portsFlag.Shorthand != "p" {
			t.Errorf("expected ports flag shorthand to be 'p', got '%s'", portsFlag.Shorthand)
		}
	}

	resetFlag := exposeCmd.Flags().Lookup("reset")
	if resetFlag == nil {
		t.Error("expose command should have --reset flag")
	}
}

// TestDefaultPorts verifies default port mappings are configured
func TestDefaultPorts(t *testing.T) {
	expectedPorts := []string{"3000", "4000", "5000", "8080", "8081", "19000", "19001", "19002"}
	for _, port := range expectedPorts {
		if _, ok := defaultPorts[port]; !ok {
			t.Errorf("expected default port mapping for %s", port)
		}
	}

	// Verify host ports have prefix to avoid conflicts
	if defaultPorts["3000"] != "13000" {
		t.Errorf("expected port 3000 to map to 13000, got %s", defaultPorts["3000"])
	}
}

// TestGenerateExposeOverlay verifies overlay generation
func TestGenerateExposeOverlay(t *testing.T) {
	ports := map[string]string{
		"3000": "13000",
		"8080": "18080",
	}

	overlay := generateExposeOverlay("drone-1", ports)

	// Check overlay content
	if !strings.Contains(overlay, "services:") {
		t.Error("overlay should contain 'services:' section")
	}
	if !strings.Contains(overlay, "drone-1:") {
		t.Error("overlay should contain agent name 'drone-1:'")
	}
	if !strings.Contains(overlay, "REACT_NATIVE_PACKAGER_HOSTNAME") {
		t.Error("overlay should set REACT_NATIVE_PACKAGER_HOSTNAME")
	}
	if !strings.Contains(overlay, "EXPO_DEVTOOLS_LISTEN_ADDRESS") {
		t.Error("overlay should set EXPO_DEVTOOLS_LISTEN_ADDRESS")
	}
	if !strings.Contains(overlay, "ports:") {
		t.Error("overlay should contain 'ports:' section")
	}
}

// TestRemoveAgentFromOverlay verifies overlay removal
func TestRemoveAgentFromOverlay(t *testing.T) {
	// Create a temporary directory
	tempDir := t.TempDir()
	overlayPath := filepath.Join(tempDir, "docker-compose.expose.yml")

	// Create a test overlay file
	content := "test overlay content"
	if err := os.WriteFile(overlayPath, []byte(content), 0644); err != nil {
		t.Fatalf("failed to create test overlay: %v", err)
	}

	// Verify file exists
	if _, err := os.Stat(overlayPath); os.IsNotExist(err) {
		t.Fatal("test overlay file should exist")
	}

	// Remove overlay
	if err := removeAgentFromOverlay(overlayPath, "drone-1"); err != nil {
		t.Errorf("removeAgentFromOverlay failed: %v", err)
	}

	// Verify file is removed
	if _, err := os.Stat(overlayPath); !os.IsNotExist(err) {
		t.Error("overlay file should be removed")
	}
}

// TestRemoveAgentFromOverlayNonExistent verifies removal of non-existent overlay
func TestRemoveAgentFromOverlayNonExistent(t *testing.T) {
	tempDir := t.TempDir()
	overlayPath := filepath.Join(tempDir, "nonexistent.yml")

	// Should not error on non-existent file
	if err := removeAgentFromOverlay(overlayPath, "drone-1"); err != nil {
		t.Errorf("removeAgentFromOverlay should not error on non-existent file: %v", err)
	}
}

// TestRunExposeInvalidAgent verifies error on invalid agent name
func TestRunExposeInvalidAgent(t *testing.T) {
	// Create temp dir with .hive
	tempDir := t.TempDir()
	oldWd, _ := os.Getwd()
	defer os.Chdir(oldWd)
	os.Chdir(tempDir)
	os.Mkdir(".hive", 0755)

	// Reset flags
	exposePorts = ""
	exposeReset = false

	err := runExpose(exposeCmd, []string{"invalid-agent"})
	if err == nil {
		t.Error("expected error for invalid agent name")
	}
	if !strings.Contains(err.Error(), "invalid agent") {
		t.Errorf("expected 'invalid agent' error, got: %v", err)
	}
}

// TestRunExposeNoHiveDir verifies error when .hive doesn't exist
func TestRunExposeNoHiveDir(t *testing.T) {
	// Create temp dir without .hive
	tempDir := t.TempDir()
	oldWd, _ := os.Getwd()
	defer os.Chdir(oldWd)
	os.Chdir(tempDir)

	// Reset flags
	exposePorts = ""
	exposeReset = false

	err := runExpose(exposeCmd, []string{"drone-1"})
	if err == nil {
		t.Error("expected error when .hive doesn't exist")
	}
	if !strings.Contains(err.Error(), "no .hive directory") {
		t.Errorf("expected '.hive directory' error, got: %v", err)
	}
}
