package preflight

import (
	"os"
	"path/filepath"
	"testing"
)

func TestCheckDockerCompose(t *testing.T) {
	// This test depends on the environment
	result := CheckDockerCompose()
	// We just verify it returns a result without panicking
	if result.Name != "Docker Compose" {
		t.Errorf("expected name 'Docker Compose', got '%s'", result.Name)
	}
}

func TestCheckEnvFile(t *testing.T) {
	// Save current directory
	originalDir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get current directory: %v", err)
	}
	defer func() { _ = os.Chdir(originalDir) }()

	// Test in temp directory without .env
	tmpDir := t.TempDir()
	if err := os.Chdir(tmpDir); err != nil {
		t.Fatalf("failed to change to temp directory: %v", err)
	}

	result := CheckEnvFile()
	if result.Passed {
		t.Error("expected check to fail when .env doesn't exist")
	}

	// Create .hive/.env and test again
	if err := os.MkdirAll(".hive", 0755); err != nil {
		t.Fatalf("failed to create .hive directory: %v", err)
	}
	if err := os.WriteFile(".hive/.env", []byte("TEST=value"), 0644); err != nil {
		t.Fatalf("failed to create .env: %v", err)
	}

	result = CheckEnvFile()
	if !result.Passed {
		t.Error("expected check to pass when .env exists")
	}
}

func TestCheckDockerComposeFile(t *testing.T) {
	// Save current directory
	originalDir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get current directory: %v", err)
	}
	defer func() { _ = os.Chdir(originalDir) }()

	// Test in temp directory without docker-compose.yml
	tmpDir := t.TempDir()
	if err := os.Chdir(tmpDir); err != nil {
		t.Fatalf("failed to change to temp directory: %v", err)
	}

	result := CheckDockerComposeFile()
	if result.Passed {
		t.Error("expected check to fail when docker-compose.yml doesn't exist")
	}

	// Create .hive/docker-compose.yml and test again
	if err := os.MkdirAll(".hive", 0755); err != nil {
		t.Fatalf("failed to create .hive directory: %v", err)
	}
	if err := os.WriteFile(".hive/docker-compose.yml", []byte("version: '3'"), 0644); err != nil {
		t.Fatalf("failed to create docker-compose.yml: %v", err)
	}

	result = CheckDockerComposeFile()
	if !result.Passed {
		t.Error("expected check to pass when docker-compose.yml exists")
	}
}

func TestCheckWorkspaceDir(t *testing.T) {
	tmpDir := t.TempDir()
	workspacePath := filepath.Join(tmpDir, "workspaces", "test")

	result := CheckWorkspaceDir(workspacePath)
	if !result.Passed {
		t.Errorf("expected check to pass for writable workspace, got: %s", result.Message)
	}

	// Verify directory was created
	if _, err := os.Stat(workspacePath); os.IsNotExist(err) {
		t.Error("workspace directory should have been created")
	}
}

func TestCheckResult(t *testing.T) {
	result := CheckResult{
		Name:    "Test Check",
		Passed:  true,
		Message: "All good",
	}

	if result.Name != "Test Check" {
		t.Errorf("unexpected name: %s", result.Name)
	}
	if !result.Passed {
		t.Error("expected Passed to be true")
	}
	if result.Message != "All good" {
		t.Errorf("unexpected message: %s", result.Message)
	}
}

func TestPrintResults(t *testing.T) {
	results := []CheckResult{
		{Name: "Check 1", Passed: true, Message: "OK"},
		{Name: "Check 2", Passed: true, Message: "OK"},
	}

	allPassed := PrintResults(results)
	if !allPassed {
		t.Error("expected all checks to pass")
	}

	// Test with failure
	results = append(results, CheckResult{Name: "Check 3", Passed: false, Message: "Failed"})
	allPassed = PrintResults(results)
	if allPassed {
		t.Error("expected some checks to fail")
	}
}

func TestCheckClaudeConfig(t *testing.T) {
	// Test with existing .claude directory
	result := CheckClaudeConfig()
	// On most dev machines, ~/.claude exists
	if result.Name != "Claude config" {
		t.Errorf("expected name 'Claude config', got '%s'", result.Name)
	}
}

func TestCheckClaudeConfig_NoClaudeDir(t *testing.T) {
	// Create temp home without .claude
	tmpHome := t.TempDir()
	oldHome := os.Getenv("HOME")
	os.Setenv("HOME", tmpHome)
	defer os.Setenv("HOME", oldHome)

	result := CheckClaudeConfig()
	if result.Passed {
		t.Error("expected check to fail when ~/.claude doesn't exist")
	}
	if result.Message == "" {
		t.Error("expected error message when ~/.claude doesn't exist")
	}
}

func TestCheckClaudeConfig_WithClaudeDir(t *testing.T) {
	// Create temp home with .claude
	tmpHome := t.TempDir()
	if err := os.MkdirAll(filepath.Join(tmpHome, ".claude"), 0755); err != nil {
		t.Fatalf("failed to create .claude directory: %v", err)
	}

	oldHome := os.Getenv("HOME")
	os.Setenv("HOME", tmpHome)
	defer os.Setenv("HOME", oldHome)

	result := CheckClaudeConfig()
	if !result.Passed {
		t.Errorf("expected check to pass when ~/.claude exists, got: %s", result.Message)
	}
}

func TestCheckWorkspaceDir_NotWritable(t *testing.T) {
	// Test with a path that can't be created (invalid path)
	result := CheckWorkspaceDir("/nonexistent/deep/path/that/cannot/be/created")
	// This should fail on most systems due to permissions
	// But we can't guarantee the behavior, so just verify it returns a result
	if result.Name != "Workspace directory" {
		t.Errorf("expected name 'Workspace directory', got '%s'", result.Name)
	}
}

func TestCheckDockerSocket(t *testing.T) {
	result := CheckDockerSocket()
	// Just verify it returns a result without panicking
	if result.Name != "Docker socket" {
		t.Errorf("expected name 'Docker socket', got '%s'", result.Name)
	}
}

func TestCheckDocker(t *testing.T) {
	result := CheckDocker()
	// Just verify it returns a result without panicking
	if result.Name != "Docker daemon" {
		t.Errorf("expected name 'Docker daemon', got '%s'", result.Name)
	}
}

func TestRunAllChecks(t *testing.T) {
	// Save current directory
	originalDir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get current directory: %v", err)
	}
	defer func() { _ = os.Chdir(originalDir) }()

	// Create temp dir with required files
	tmpDir := t.TempDir()
	if err := os.Chdir(tmpDir); err != nil {
		t.Fatalf("failed to change directory: %v", err)
	}

	// Create .hive/.env and docker-compose.yml
	if err := os.MkdirAll(".hive", 0755); err != nil {
		t.Fatalf("failed to create .hive: %v", err)
	}
	os.WriteFile(".hive/.env", []byte("TEST=1"), 0644)
	os.WriteFile(".hive/docker-compose.yml", []byte("version: '3'"), 0644)

	results := RunAllChecks()

	// Verify we got results
	if len(results) == 0 {
		t.Error("expected at least one check result")
	}

	// Verify each result has a name
	for _, r := range results {
		if r.Name == "" {
			t.Error("check result has empty name")
		}
	}
}
