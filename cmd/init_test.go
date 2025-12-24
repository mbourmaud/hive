package cmd

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// setupTestRepo creates a test git repository
func setupTestRepo(t *testing.T) (string, string, func()) {
	t.Helper()

	// Create temp directory
	tmpDir, err := os.MkdirTemp("", "hive-test-*")
	if err != nil {
		t.Fatalf("failed to create temp dir: %v", err)
	}

	// Change to temp directory
	origDir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get current dir: %v", err)
	}
	if err := os.Chdir(tmpDir); err != nil {
		t.Fatalf("failed to chdir: %v", err)
	}

	// Initialize git repo
	cmds := [][]string{
		{"git", "init"},
		{"git", "config", "user.email", "test@example.com"},
		{"git", "config", "user.name", "Test User"},
		{"git", "commit", "--allow-empty", "-m", "initial commit"},
	}

	for _, cmdArgs := range cmds {
		cmd := exec.Command(cmdArgs[0], cmdArgs[1:]...)
		if err := cmd.Run(); err != nil {
			cleanup := func() {
				os.Chdir(origDir)
				os.RemoveAll(tmpDir)
			}
			cleanup()
			t.Fatalf("failed to run %v: %v", cmdArgs, err)
		}
	}

	// Detect default branch name (main or master)
	branchCmd := exec.Command("git", "rev-parse", "--abbrev-ref", "HEAD")
	branchOut, err := branchCmd.Output()
	if err != nil {
		cleanup := func() {
			os.Chdir(origDir)
			os.RemoveAll(tmpDir)
		}
		cleanup()
		t.Fatalf("failed to detect default branch: %v", err)
	}
	defaultBranch := strings.TrimSpace(string(branchOut))

	cleanup := func() {
		os.Chdir(origDir)
		os.RemoveAll(tmpDir)
	}

	return tmpDir, defaultBranch, cleanup
}

// TestCreateWorktree_FreshRepo tests creating a worktree in a fresh repo
func TestCreateWorktree_FreshRepo(t *testing.T) {
	_, defaultBranch, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreePath := filepath.Join(".hive", "workspaces", "queen")
	err := createWorktree(worktreePath, defaultBranch, "queen")
	if err != nil {
		t.Fatalf("createWorktree failed: %v", err)
	}

	// Verify worktree exists
	if _, err := os.Stat(filepath.Join(worktreePath, ".git")); os.IsNotExist(err) {
		t.Error("worktree .git file not created")
	}

	// Verify branch exists
	cmd := exec.Command("git", "branch", "--list", "hive/queen")
	output, err := cmd.Output()
	if err != nil || !strings.Contains(string(output), "hive/queen") {
		t.Error("hive/queen branch not created")
	}
}

// TestCreateWorktree_AfterClean simulates state after hive clean
func TestCreateWorktree_AfterClean(t *testing.T) {
	_, defaultBranch, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreePath := filepath.Join(".hive", "workspaces", "queen")

	// Create initial worktree
	if err := createWorktree(worktreePath, defaultBranch, "queen"); err != nil {
		t.Fatalf("initial createWorktree failed: %v", err)
	}

	// Simulate hive clean: remove directory but leave branch
	os.RemoveAll(".hive")

	// Try to create worktree again (should succeed)
	err := createWorktree(worktreePath, defaultBranch, "queen")
	if err != nil {
		t.Fatalf("createWorktree after clean failed: %v", err)
	}

	// Verify worktree exists
	if _, err := os.Stat(filepath.Join(worktreePath, ".git")); os.IsNotExist(err) {
		t.Error("worktree not recreated after clean")
	}
}

// TestCreateWorktree_OrphanedDirectory simulates orphaned worktree directory
func TestCreateWorktree_OrphanedDirectory(t *testing.T) {
	_, defaultBranch, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreePath := filepath.Join(".hive", "workspaces", "queen")

	// Create initial worktree
	if err := createWorktree(worktreePath, defaultBranch, "queen"); err != nil {
		t.Fatalf("initial createWorktree failed: %v", err)
	}

	// Remove worktree from git but leave directory
	cmd := exec.Command("git", "worktree", "remove", worktreePath, "--force")
	cmd.Run() // Ignore errors

	// Try to create worktree again (should handle orphaned directory)
	err := createWorktree(worktreePath, defaultBranch, "queen")
	if err != nil {
		t.Fatalf("createWorktree with orphaned directory failed: %v", err)
	}

	// Verify worktree exists and is valid
	listCmd := exec.Command("git", "worktree", "list")
	output, err := listCmd.Output()
	if err != nil || !strings.Contains(string(output), worktreePath) {
		t.Error("worktree not properly recreated")
	}
}

// TestCreateWorktree_ExistingBranch tests creating worktree when branch already exists
func TestCreateWorktree_ExistingBranch(t *testing.T) {
	_, defaultBranch, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreePath := filepath.Join(".hive", "workspaces", "queen")

	// Create branch manually
	cmd := exec.Command("git", "branch", "hive/queen")
	if err := cmd.Run(); err != nil {
		t.Fatalf("failed to create branch: %v", err)
	}

	// Try to create worktree (should use existing branch)
	err := createWorktree(worktreePath, defaultBranch, "queen")
	if err != nil {
		t.Fatalf("createWorktree with existing branch failed: %v", err)
	}

	// Verify worktree exists
	if _, err := os.Stat(filepath.Join(worktreePath, ".git")); os.IsNotExist(err) {
		t.Error("worktree not created with existing branch")
	}
}

// TestCreateWorktree_MultipleWorkers tests creating multiple worktrees
func TestCreateWorktree_MultipleWorkers(t *testing.T) {
	_, defaultBranch, cleanup := setupTestRepo(t)
	defer cleanup()

	agents := []string{"queen", "drone-1", "drone-2"}

	for _, agent := range agents {
		worktreePath := filepath.Join(".hive", "workspaces", agent)
		err := createWorktree(worktreePath, defaultBranch, agent)
		if err != nil {
			t.Fatalf("createWorktree failed for %s: %v", agent, err)
		}
	}

	// Verify all worktrees exist
	listCmd := exec.Command("git", "worktree", "list")
	output, err := listCmd.Output()
	if err != nil {
		t.Fatalf("failed to list worktrees: %v", err)
	}

	for _, agent := range agents {
		if !strings.Contains(string(output), agent) {
			t.Errorf("worktree for %s not found in list", agent)
		}
	}
}

// TestCreateWorktree_Idempotent tests that calling createWorktree twice is safe
func TestCreateWorktree_Idempotent(t *testing.T) {
	_, defaultBranch, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreePath := filepath.Join(".hive", "workspaces", "queen")

	// Create worktree
	if err := createWorktree(worktreePath, defaultBranch, "queen"); err != nil {
		t.Fatalf("first createWorktree failed: %v", err)
	}

	// Call again (should be no-op)
	if err := createWorktree(worktreePath, defaultBranch, "queen"); err != nil {
		t.Fatalf("second createWorktree failed: %v", err)
	}

	// Verify only one worktree exists
	listCmd := exec.Command("git", "worktree", "list")
	output, err := listCmd.Output()
	if err != nil {
		t.Fatalf("failed to list worktrees: %v", err)
	}

	// Count occurrences of worktree path
	count := strings.Count(string(output), worktreePath)
	if count != 1 {
		t.Errorf("expected 1 worktree, found %d", count)
	}
}
