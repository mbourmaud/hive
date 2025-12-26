package cmd

import (
	"testing"
)

// TestStopCmdUsage verifies the stop command has correct configuration
func TestStopCmdUsage(t *testing.T) {
	if stopCmd.Use != "stop" {
		t.Errorf("expected Use to be 'stop', got '%s'", stopCmd.Use)
	}
	if stopCmd.Short == "" {
		t.Error("Short description should not be empty")
	}
	if stopCmd.Long == "" {
		t.Error("Long description should not be empty")
	}
}

// TestStopCmdRegistered verifies the stop command is registered with root
func TestStopCmdRegistered(t *testing.T) {
	// The stop command should be a subcommand of root
	found := false
	for _, cmd := range rootCmd.Commands() {
		if cmd.Use == "stop" {
			found = true
			break
		}
	}

	if !found {
		t.Error("stop command should be registered with root command")
	}
}
