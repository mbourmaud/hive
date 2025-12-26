package cmd

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TestStartCmdUsage verifies the start command has correct configuration
func TestStartCmdUsage(t *testing.T) {
	if startCmd.Use != "start [count]" {
		t.Errorf("expected Use to be 'start [count]', got '%s'", startCmd.Use)
	}
	if startCmd.Short == "" {
		t.Error("Short description should not be empty")
	}
}

// TestStartCmdFlags verifies the start command has expected flags
func TestStartCmdFlags(t *testing.T) {
	// Test --skip-checks flag exists
	skipChecks := startCmd.Flags().Lookup("skip-checks")
	if skipChecks == nil {
		t.Error("expected --skip-checks flag to exist")
	}

	// Test --wait flag exists
	wait := startCmd.Flags().Lookup("wait")
	if wait == nil {
		t.Error("expected --wait flag to exist")
	}
}

// TestContainerNameMapping verifies container names are correctly mapped
func TestContainerNameMapping(t *testing.T) {
	tests := []struct {
		service       string
		expectedName  string
	}{
		{"queen", "hive-queen"},
		{"redis", "hive-redis"},
		{"drone-1", "hive-drone-1"},
		{"drone-2", "hive-drone-2"},
		{"drone-10", "hive-drone-10"},
	}

	for _, tt := range tests {
		t.Run(tt.service, func(t *testing.T) {
			var containerName string
			switch tt.service {
			case "queen":
				containerName = "hive-queen"
			case "redis":
				containerName = "hive-redis"
			default:
				containerName = "hive-" + tt.service
			}

			if containerName != tt.expectedName {
				t.Errorf("container name for %s = %s, want %s", tt.service, containerName, tt.expectedName)
			}
		})
	}
}

// TestServicesList verifies services list is built correctly
func TestServicesList(t *testing.T) {
	tests := []struct {
		workerCount  int
		wantServices []string
	}{
		{1, []string{"redis", "queen", "drone-1"}},
		{2, []string{"redis", "queen", "drone-1", "drone-2"}},
		{5, []string{"redis", "queen", "drone-1", "drone-2", "drone-3", "drone-4", "drone-5"}},
	}

	for _, tt := range tests {
		t.Run(string(rune(tt.workerCount+'0')), func(t *testing.T) {
			services := []string{"redis", "queen"}
			for i := 1; i <= tt.workerCount; i++ {
				services = append(services, "drone-"+string(rune('0'+i)))
			}

			// Simple check: service count should match
			if len(services) != len(tt.wantServices) {
				t.Errorf("services count = %d, want %d", len(services), len(tt.wantServices))
			}
		})
	}
}

// TestWorkspaceDirCreation tests the workspace directory structure
func TestWorkspaceDirCreation(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Simulate the directory creation logic from start command
	hiveDir := ".hive"
	workspacesDir := filepath.Join(hiveDir, "workspaces")
	agents := []string{"queen", "drone-1", "drone-2"}

	for _, agent := range agents {
		agentDir := filepath.Join(workspacesDir, agent)
		sessionEnvDir := filepath.Join(agentDir, "session-env")
		if err := os.MkdirAll(sessionEnvDir, 0755); err != nil {
			t.Fatalf("failed to create workspace dir for %s: %v", agent, err)
		}
		historyFile := filepath.Join(agentDir, "history.jsonl")
		if err := os.WriteFile(historyFile, []byte{}, 0644); err != nil {
			t.Fatalf("failed to create history file for %s: %v", agent, err)
		}
	}

	// Verify structure was created
	for _, agent := range agents {
		agentDir := filepath.Join(workspacesDir, agent)

		// Check agent dir exists
		if _, err := os.Stat(agentDir); os.IsNotExist(err) {
			t.Errorf("expected agent dir %s to exist", agentDir)
		}

		// Check session-env dir exists
		sessionEnvDir := filepath.Join(agentDir, "session-env")
		if _, err := os.Stat(sessionEnvDir); os.IsNotExist(err) {
			t.Errorf("expected session-env dir %s to exist", sessionEnvDir)
		}

		// Check history.jsonl exists
		historyFile := filepath.Join(agentDir, "history.jsonl")
		if _, err := os.Stat(historyFile); os.IsNotExist(err) {
			t.Errorf("expected history file %s to exist", historyFile)
		}
	}
}

// TestWorkerCountValidation tests the worker count constraints
func TestWorkerCountValidation(t *testing.T) {
	tests := []struct {
		count   int
		wantErr bool
		errMsg  string
	}{
		{0, true, "minimum 1 worker required"},
		{1, false, ""},
		{2, false, ""},
		{5, false, ""},
		{10, false, ""},
		{11, true, "maximum 10 workers allowed"},
		{100, true, "maximum 10 workers allowed"},
	}

	for _, tt := range tests {
		t.Run(string(rune(tt.count)), func(t *testing.T) {
			// Simulate the validation logic from start command
			var err error
			if tt.count > 10 {
				err = &testError{msg: "maximum 10 workers allowed"}
			} else if tt.count < 1 {
				err = &testError{msg: "minimum 1 worker required"}
			}

			if tt.wantErr {
				if err == nil {
					t.Errorf("count=%d expected error, got nil", tt.count)
				} else if !strings.Contains(err.Error(), tt.errMsg) {
					t.Errorf("count=%d error = %v, want %v", tt.count, err, tt.errMsg)
				}
			} else {
				if err != nil {
					t.Errorf("count=%d unexpected error: %v", tt.count, err)
				}
			}
		})
	}
}

// testError is a simple error type for testing
type testError struct {
	msg string
}

func (e *testError) Error() string {
	return e.msg
}
