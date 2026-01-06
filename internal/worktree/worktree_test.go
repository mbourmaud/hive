package worktree

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func TestIsGitRepository(t *testing.T) {
	// Current directory should be a git repo
	cwd, _ := os.Getwd()
	repoRoot, _ := GetRepoRoot(cwd)

	if !IsGitRepository(repoRoot) {
		t.Skip("not running in a git repository")
	}

	// Temp directory should not be a git repo
	tmpDir := t.TempDir()
	if IsGitRepository(tmpDir) {
		t.Error("temp directory should not be a git repository")
	}
}

func TestGetRepoRoot(t *testing.T) {
	cwd, _ := os.Getwd()

	root, err := GetRepoRoot(cwd)
	if err != nil {
		t.Skip("not running in a git repository")
	}

	// Verify .git exists in root
	if _, err := os.Stat(filepath.Join(root, ".git")); err != nil {
		t.Errorf(".git should exist in repo root: %v", err)
	}
}

func TestNewGitManager(t *testing.T) {
	mgr := NewGitManager("/some/repo", "")

	home, _ := os.UserHomeDir()
	expectedWorkDir := filepath.Join(home, "hive-worktrees")

	if mgr.workDir != expectedWorkDir {
		t.Errorf("expected workDir %s, got %s", expectedWorkDir, mgr.workDir)
	}

	// With custom workDir
	mgr = NewGitManager("/some/repo", "/custom/workdir")
	if mgr.workDir != "/custom/workdir" {
		t.Errorf("expected workDir /custom/workdir, got %s", mgr.workDir)
	}
}

func TestGitManager_Create(t *testing.T) {
	// Create a temporary git repo for testing
	tmpDir := t.TempDir()
	repoDir := filepath.Join(tmpDir, "repo")
	workDir := filepath.Join(tmpDir, "worktrees")

	// Initialize a git repo
	if err := os.MkdirAll(repoDir, 0755); err != nil {
		t.Fatal(err)
	}

	cmds := [][]string{
		{"git", "init"},
		{"git", "config", "user.email", "test@test.com"},
		{"git", "config", "user.name", "Test"},
	}

	for _, args := range cmds {
		cmd := exec.Command(args[0], args[1:]...)
		cmd.Dir = repoDir
		if err := cmd.Run(); err != nil {
			t.Fatalf("failed to run %v: %v", args, err)
		}
	}

	// Create initial commit
	testFile := filepath.Join(repoDir, "test.txt")
	if err := os.WriteFile(testFile, []byte("test"), 0644); err != nil {
		t.Fatal(err)
	}

	cmds = [][]string{
		{"git", "add", "."},
		{"git", "commit", "-m", "initial"},
	}

	for _, args := range cmds {
		cmd := exec.Command(args[0], args[1:]...)
		cmd.Dir = repoDir
		if err := cmd.Run(); err != nil {
			t.Fatalf("failed to run %v: %v", args, err)
		}
	}

	mgr := NewGitManager(repoDir, workDir)
	ctx := context.Background()

	// Test creating a worktree
	wt, err := mgr.Create(ctx, CreateOptions{
		Name: "test-agent",
	})
	if err != nil {
		t.Fatalf("failed to create worktree: %v", err)
	}

	if wt.Name != "test-agent" {
		t.Errorf("expected name test-agent, got %s", wt.Name)
	}

	if wt.Branch != "hive/test-agent" {
		t.Errorf("expected branch hive/test-agent, got %s", wt.Branch)
	}

	// Path should contain the agent name (exact path depends on symlink resolution)
	if !strings.HasSuffix(wt.Path, "test-agent") {
		t.Errorf("expected path to end with test-agent, got %s", wt.Path)
	}

	// Verify worktree exists
	if _, err := os.Stat(filepath.Join(wt.Path, ".git")); err != nil {
		t.Errorf("worktree .git should exist: %v", err)
	}

	// Creating same worktree again should succeed (idempotent)
	wt2, err := mgr.Create(ctx, CreateOptions{
		Name: "test-agent",
	})
	if err != nil {
		t.Fatalf("second create should succeed: %v", err)
	}
	if wt2.Path != wt.Path {
		t.Error("second create should return same worktree")
	}
}

func TestGitManager_Delete(t *testing.T) {
	// Create a temporary git repo for testing
	tmpDir := t.TempDir()
	repoDir := filepath.Join(tmpDir, "repo")
	workDir := filepath.Join(tmpDir, "worktrees")

	// Initialize a git repo with initial commit
	if err := os.MkdirAll(repoDir, 0755); err != nil {
		t.Fatal(err)
	}

	cmds := [][]string{
		{"git", "init"},
		{"git", "config", "user.email", "test@test.com"},
		{"git", "config", "user.name", "Test"},
	}

	for _, args := range cmds {
		cmd := exec.Command(args[0], args[1:]...)
		cmd.Dir = repoDir
		cmd.Run()
	}

	testFile := filepath.Join(repoDir, "test.txt")
	os.WriteFile(testFile, []byte("test"), 0644)

	exec.Command("git", "-C", repoDir, "add", ".").Run()
	exec.Command("git", "-C", repoDir, "commit", "-m", "initial").Run()

	mgr := NewGitManager(repoDir, workDir)
	ctx := context.Background()

	// Create a worktree first
	wt, err := mgr.Create(ctx, CreateOptions{Name: "to-delete"})
	if err != nil {
		t.Fatalf("failed to create worktree: %v", err)
	}

	// Verify it exists
	if _, err := os.Stat(wt.Path); err != nil {
		t.Fatal("worktree should exist")
	}

	// Delete it
	if err := mgr.Delete(ctx, "to-delete"); err != nil {
		t.Fatalf("failed to delete worktree: %v", err)
	}

	// Verify it's gone
	if _, err := os.Stat(wt.Path); err == nil {
		t.Error("worktree should be deleted")
	}
}

func TestGitManager_List(t *testing.T) {
	// Create a temporary git repo for testing
	tmpDir := t.TempDir()
	repoDir := filepath.Join(tmpDir, "repo")
	workDir := filepath.Join(tmpDir, "worktrees")

	// Initialize a git repo with initial commit
	if err := os.MkdirAll(repoDir, 0755); err != nil {
		t.Fatal(err)
	}

	cmds := [][]string{
		{"git", "init"},
		{"git", "config", "user.email", "test@test.com"},
		{"git", "config", "user.name", "Test"},
	}

	for _, args := range cmds {
		cmd := exec.Command(args[0], args[1:]...)
		cmd.Dir = repoDir
		cmd.Run()
	}

	testFile := filepath.Join(repoDir, "test.txt")
	os.WriteFile(testFile, []byte("test"), 0644)

	exec.Command("git", "-C", repoDir, "add", ".").Run()
	exec.Command("git", "-C", repoDir, "commit", "-m", "initial").Run()

	mgr := NewGitManager(repoDir, workDir)
	ctx := context.Background()

	// Create some worktrees
	_, err := mgr.Create(ctx, CreateOptions{Name: "agent-1"})
	if err != nil {
		t.Fatalf("failed to create agent-1: %v", err)
	}

	_, err = mgr.Create(ctx, CreateOptions{Name: "agent-2"})
	if err != nil {
		t.Fatalf("failed to create agent-2: %v", err)
	}

	// List worktrees
	worktrees, err := mgr.List(ctx)
	if err != nil {
		t.Fatalf("failed to list worktrees: %v", err)
	}

	if len(worktrees) != 2 {
		t.Errorf("expected 2 worktrees, got %d", len(worktrees))
	}

	names := make(map[string]bool)
	for _, wt := range worktrees {
		names[wt.Name] = true
	}

	if !names["agent-1"] || !names["agent-2"] {
		t.Error("expected agent-1 and agent-2 in list")
	}
}

func TestGitManager_Get(t *testing.T) {
	// Create a temporary git repo for testing
	tmpDir := t.TempDir()
	repoDir := filepath.Join(tmpDir, "repo")
	workDir := filepath.Join(tmpDir, "worktrees")

	// Initialize a git repo with initial commit
	os.MkdirAll(repoDir, 0755)

	cmds := [][]string{
		{"git", "init"},
		{"git", "config", "user.email", "test@test.com"},
		{"git", "config", "user.name", "Test"},
	}

	for _, args := range cmds {
		cmd := exec.Command(args[0], args[1:]...)
		cmd.Dir = repoDir
		cmd.Run()
	}

	testFile := filepath.Join(repoDir, "test.txt")
	os.WriteFile(testFile, []byte("test"), 0644)

	exec.Command("git", "-C", repoDir, "add", ".").Run()
	exec.Command("git", "-C", repoDir, "commit", "-m", "initial").Run()

	mgr := NewGitManager(repoDir, workDir)
	ctx := context.Background()

	// Create a worktree
	created, _ := mgr.Create(ctx, CreateOptions{Name: "findme"})

	// Get it
	found, err := mgr.Get(ctx, "findme")
	if err != nil {
		t.Fatalf("failed to get worktree: %v", err)
	}

	if found.Path != created.Path {
		t.Error("found worktree should match created")
	}

	// Get non-existent
	_, err = mgr.Get(ctx, "notfound")
	if err == nil {
		t.Error("getting non-existent worktree should fail")
	}
}

func TestCreateOptions_Validation(t *testing.T) {
	tmpDir := t.TempDir()
	mgr := NewGitManager(tmpDir, tmpDir)
	ctx := context.Background()

	// Empty name should fail
	_, err := mgr.Create(ctx, CreateOptions{Name: ""})
	if err == nil {
		t.Error("empty name should fail")
	}
}
