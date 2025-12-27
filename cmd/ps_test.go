package cmd

import (
	"testing"
)

// TestPsCmdUsage verifies the ps command has correct configuration
func TestPsCmdUsage(t *testing.T) {
	if psCmd.Use != "ps [agent]" {
		t.Errorf("expected Use to be 'ps [agent]', got '%s'", psCmd.Use)
	}
	if psCmd.Short == "" {
		t.Error("Short description should not be empty")
	}
	if psCmd.Long == "" {
		t.Error("Long description should not be empty")
	}
}

// TestPsCmdRegistered verifies the ps command is registered with root
func TestPsCmdRegistered(t *testing.T) {
	found := false
	for _, cmd := range rootCmd.Commands() {
		if cmd.Use == "ps [agent]" {
			found = true
			break
		}
	}

	if !found {
		t.Error("ps command should be registered with root command")
	}
}

// TestPsCmdFlags verifies the ps command has correct flags
func TestPsCmdFlags(t *testing.T) {
	verboseFlag := psCmd.Flags().Lookup("verbose")
	if verboseFlag == nil {
		t.Error("ps command should have --verbose flag")
	} else {
		if verboseFlag.Shorthand != "v" {
			t.Errorf("expected verbose flag shorthand to be 'v', got '%s'", verboseFlag.Shorthand)
		}
	}
}

// TestPsCmdArgs verifies the ps command accepts correct number of args
func TestPsCmdArgs(t *testing.T) {
	// Should accept 0 args
	if err := psCmd.Args(psCmd, []string{}); err != nil {
		t.Errorf("ps should accept 0 args: %v", err)
	}

	// Should accept 1 arg
	if err := psCmd.Args(psCmd, []string{"drone-1"}); err != nil {
		t.Errorf("ps should accept 1 arg: %v", err)
	}

	// Should reject 2 args
	if err := psCmd.Args(psCmd, []string{"drone-1", "drone-2"}); err == nil {
		t.Error("ps should reject more than 1 arg")
	}
}

// TestPortInfoStruct verifies PortInfo struct fields
func TestPortInfoStruct(t *testing.T) {
	pi := PortInfo{Port: 3000, Process: "node server.js"}
	if pi.Port != 3000 {
		t.Errorf("expected Port to be 3000, got %d", pi.Port)
	}
	if pi.Process != "node server.js" {
		t.Errorf("expected Process to be 'node server.js', got '%s'", pi.Process)
	}
}

// TestProcessInfoStruct verifies ProcessInfo struct fields
func TestProcessInfoStruct(t *testing.T) {
	proc := ProcessInfo{PID: "123", Command: "npm start"}
	if proc.PID != "123" {
		t.Errorf("expected PID to be '123', got '%s'", proc.PID)
	}
	if proc.Command != "npm start" {
		t.Errorf("expected Command to be 'npm start', got '%s'", proc.Command)
	}
}

// TestGetListeningPortsNoContainer verifies behavior when container doesn't exist
func TestGetListeningPortsNoContainer(t *testing.T) {
	// This should return empty slice without error for non-existent container
	ports := getListeningPorts("nonexistent-container-12345")
	if len(ports) != 0 {
		t.Errorf("expected empty ports for non-existent container, got %d", len(ports))
	}
}

// TestGetUserProcessesNoContainer verifies behavior when container doesn't exist
func TestGetUserProcessesNoContainer(t *testing.T) {
	// This should return empty slice without error for non-existent container
	processes := getUserProcesses("nonexistent-container-12345", false)
	if len(processes) != 0 {
		t.Errorf("expected empty processes for non-existent container, got %d", len(processes))
	}
}

// TestGetUserProcessesVerboseFlag verifies verbose flag handling
func TestGetUserProcessesVerboseFlag(t *testing.T) {
	// Just verify the function handles verbose flag without panic
	_ = getUserProcesses("nonexistent-container-12345", true)
	_ = getUserProcesses("nonexistent-container-12345", false)
}
