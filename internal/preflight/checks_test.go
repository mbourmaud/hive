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

	// Create .env and test again
	if err := os.WriteFile(".env", []byte("TEST=value"), 0644); err != nil {
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

	// Create docker-compose.yml and test again
	if err := os.WriteFile("docker-compose.yml", []byte("version: '3'"), 0644); err != nil {
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
