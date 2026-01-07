package integration

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	"github.com/mbourmaud/hive/internal/agent"
	"github.com/mbourmaud/hive/internal/hub"
	"github.com/mbourmaud/hive/internal/worktree"
)

func skipIfNoAgentAPI(t *testing.T) {
	if _, err := exec.LookPath("agentapi"); err != nil {
		t.Skip("agentapi not found, skipping integration test")
	}
}

func skipIfNoClaude(t *testing.T) {
	if _, err := exec.LookPath("claude"); err != nil {
		t.Skip("claude not found, skipping integration test")
	}
}

func setupTestRepo(t *testing.T) string {
	t.Helper()

	tmpDir := t.TempDir()
	repoDir := filepath.Join(tmpDir, "test-repo")

	if err := os.MkdirAll(repoDir, 0755); err != nil {
		t.Fatalf("Failed to create repo dir: %v", err)
	}

	cmds := [][]string{
		{"git", "init"},
		{"git", "config", "user.email", "test@test.com"},
		{"git", "config", "user.name", "Test User"},
	}

	for _, args := range cmds {
		cmd := exec.Command(args[0], args[1:]...)
		cmd.Dir = repoDir
		if out, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("Failed to run %v: %v\n%s", args, err, out)
		}
	}

	readmePath := filepath.Join(repoDir, "README.md")
	if err := os.WriteFile(readmePath, []byte("# Test Repo\n"), 0644); err != nil {
		t.Fatalf("Failed to create README: %v", err)
	}

	cmd := exec.Command("git", "add", ".")
	cmd.Dir = repoDir
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("Failed to git add: %v\n%s", err, out)
	}

	cmd = exec.Command("git", "commit", "-m", "Initial commit")
	cmd.Dir = repoDir
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("Failed to git commit: %v\n%s", err, out)
	}

	return repoDir
}

func TestWorktreeCreation(t *testing.T) {
	repoDir := setupTestRepo(t)

	ctx := context.Background()
	mgr := worktree.NewGitManager(repoDir, "")

	wt, err := mgr.Create(ctx, worktree.CreateOptions{
		Name:     "test-worktree",
		RepoPath: repoDir,
		Branch:   "feature/test",
	})
	if err != nil {
		t.Fatalf("Failed to create worktree: %v", err)
	}

	if wt.Path == "" {
		t.Error("Worktree path is empty")
	}

	if _, err := os.Stat(wt.Path); os.IsNotExist(err) {
		t.Errorf("Worktree directory does not exist: %s", wt.Path)
	}

	readmePath := filepath.Join(wt.Path, "README.md")
	if _, err := os.Stat(readmePath); os.IsNotExist(err) {
		t.Error("README.md not found in worktree")
	}

	if err := mgr.Delete(ctx, "test-worktree"); err != nil {
		t.Errorf("Failed to delete worktree: %v", err)
	}
}

func TestHubCreation(t *testing.T) {
	repoDir := setupTestRepo(t)

	cfg := hub.Config{
		Port:     0,
		RepoPath: repoDir,
		BasePort: 17440,
		Sandbox:  false,
	}

	h, err := hub.New(cfg)
	if err != nil {
		t.Fatalf("Failed to create hub: %v", err)
	}

	if h.AgentManager() == nil {
		t.Error("AgentManager is nil")
	}

	if h.TaskManager() == nil {
		t.Error("TaskManager is nil")
	}

	if h.PortRegistry() == nil {
		t.Error("PortRegistry is nil")
	}

	if h.SolicitationManager() == nil {
		t.Error("SolicitationManager is nil")
	}
}

func TestStatePersistence(t *testing.T) {
	repoDir := setupTestRepo(t)

	cfg := hub.Config{
		Port:     0,
		RepoPath: repoDir,
		BasePort: 17440,
		Sandbox:  false,
	}

	h, err := hub.New(cfg)
	if err != nil {
		t.Fatalf("Failed to create hub: %v", err)
	}

	testAgent := &agent.Agent{
		ID:           "int-test-123",
		Name:         "integration-test",
		WorktreePath: "/tmp/test",
		Branch:       "main",
		Port:         17440,
		PID:          99999,
		Status:       agent.StatusReady,
		CreatedAt:    time.Now(),
	}

	h.AgentManager().RegisterAgent(testAgent)

	if err := h.SaveState(); err != nil {
		t.Fatalf("Failed to save state: %v", err)
	}

	statePath := filepath.Join(repoDir, ".hive", "hub-state.json")
	if _, err := os.Stat(statePath); os.IsNotExist(err) {
		t.Errorf("State file not created at %s", statePath)
	}

	h2, err := hub.New(cfg)
	if err != nil {
		t.Fatalf("Failed to create second hub: %v", err)
	}

	agents := h2.AgentManager().ListAgents()
	if len(agents) != 0 {
		t.Log("Note: Agent was restored but marked as dead (expected - PID doesn't exist)")
	}
}

func TestAgentSpawnWithSystemPrompt(t *testing.T) {
	skipIfNoAgentAPI(t)
	skipIfNoClaude(t)

	repoDir := setupTestRepo(t)
	ctx := context.Background()

	worktreeMgr := worktree.NewGitManager(repoDir, "")
	client := agent.NewHTTPClient()
	spawner := agent.NewProcessSpawner(worktreeMgr, client)
	spawner.SetBasePort(13300)

	mgr := agent.NewManager(spawner, client)

	a, err := mgr.SpawnAgent(ctx, agent.SpawnOptions{
		Name:      "prompt-test",
		RepoPath:  repoDir,
		Branch:    "feature/prompt-test",
		Specialty: "backend",
		Sandbox:   false,
		HubURL:    "http://localhost:7433",
	})
	if err != nil {
		t.Fatalf("Failed to spawn agent: %v", err)
	}

	defer func() {
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		_ = mgr.DestroyAgent(ctx, a.ID)
	}()

	claudeMdPath := filepath.Join(a.WorktreePath, "CLAUDE.md")
	content, err := os.ReadFile(claudeMdPath)
	if err != nil {
		t.Fatalf("Failed to read CLAUDE.md: %v", err)
	}

	contentStr := string(content)

	checks := []struct {
		name    string
		contain string
	}{
		{"agent name", "prompt-test"},
		{"specialty", "backend"},
		{"hub url", "http://localhost:7433"},
		{"branch", "feature/prompt-test"},
	}

	for _, check := range checks {
		if !contains(contentStr, check.contain) {
			t.Errorf("CLAUDE.md should contain %s (%s)", check.name, check.contain)
		}
	}

	if a.Status != agent.StatusReady {
		t.Errorf("Expected agent status Ready, got %s", a.Status)
	}
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > 0 && containsHelper(s, substr))
}

func containsHelper(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
