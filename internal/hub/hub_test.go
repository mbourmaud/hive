package hub

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"
)

// setupTestRepo creates a temporary git repository for testing.
func setupTestRepo(t *testing.T) (repoDir, workDir string, cleanup func()) {
	t.Helper()

	tmpDir := t.TempDir()
	repoDir = filepath.Join(tmpDir, "repo")
	workDir = filepath.Join(tmpDir, "worktrees")

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

	exec.Command("git", "-C", repoDir, "add", ".").Run()
	exec.Command("git", "-C", repoDir, "commit", "-m", "initial").Run()

	return repoDir, workDir, func() {
		// Cleanup worktrees
		exec.Command("git", "-C", repoDir, "worktree", "prune").Run()
	}
}

func TestNew(t *testing.T) {
	repoDir, workDir, cleanup := setupTestRepo(t)
	defer cleanup()

	cfg := Config{
		Port:         7433,
		RepoPath:     repoDir,
		WorktreesDir: workDir,
		BasePort:     7440,
	}

	hub, err := New(cfg)
	if err != nil {
		t.Fatalf("New failed: %v", err)
	}

	if hub.agentManager == nil {
		t.Error("agentManager should not be nil")
	}

	if hub.eventHub == nil {
		t.Error("eventHub should not be nil")
	}
}

func TestNew_MissingRepoPath(t *testing.T) {
	cfg := Config{
		Port: 7433,
	}

	_, err := New(cfg)
	if err == nil {
		t.Error("expected error for missing repo_path")
	}
}

func TestHub_handleHealth(t *testing.T) {
	repoDir, workDir, cleanup := setupTestRepo(t)
	defer cleanup()

	hub, _ := New(Config{
		RepoPath:     repoDir,
		WorktreesDir: workDir,
	})

	req := httptest.NewRequest("GET", "/health", nil)
	w := httptest.NewRecorder()

	hub.handleHealth(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected status 200, got %d", w.Code)
	}

	var resp map[string]interface{}
	json.NewDecoder(w.Body).Decode(&resp)

	if resp["status"] != "ok" {
		t.Errorf("expected status ok, got %v", resp["status"])
	}
}

func TestHub_handleListAgents_Empty(t *testing.T) {
	repoDir, workDir, cleanup := setupTestRepo(t)
	defer cleanup()

	hub, _ := New(Config{
		RepoPath:     repoDir,
		WorktreesDir: workDir,
	})

	req := httptest.NewRequest("GET", "/agents", nil)
	w := httptest.NewRecorder()

	hub.handleListAgents(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected status 200, got %d", w.Code)
	}

	var agents []AgentResponse
	json.NewDecoder(w.Body).Decode(&agents)

	if len(agents) != 0 {
		t.Errorf("expected 0 agents, got %d", len(agents))
	}
}

func TestHub_handleSpawnAgent_MissingName(t *testing.T) {
	repoDir, workDir, cleanup := setupTestRepo(t)
	defer cleanup()

	hub, _ := New(Config{
		RepoPath:     repoDir,
		WorktreesDir: workDir,
	})

	body := bytes.NewBufferString(`{}`)
	req := httptest.NewRequest("POST", "/agents", body)
	w := httptest.NewRecorder()

	hub.handleSpawnAgent(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", w.Code)
	}
}

func TestHub_handleSpawnAgent_InvalidJSON(t *testing.T) {
	repoDir, workDir, cleanup := setupTestRepo(t)
	defer cleanup()

	hub, _ := New(Config{
		RepoPath:     repoDir,
		WorktreesDir: workDir,
	})

	body := bytes.NewBufferString(`{invalid}`)
	req := httptest.NewRequest("POST", "/agents", body)
	w := httptest.NewRecorder()

	hub.handleSpawnAgent(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", w.Code)
	}
}

func TestHub_handleGetAgent_NotFound(t *testing.T) {
	repoDir, workDir, cleanup := setupTestRepo(t)
	defer cleanup()

	hub, _ := New(Config{
		RepoPath:     repoDir,
		WorktreesDir: workDir,
	})

	req := httptest.NewRequest("GET", "/agents/nonexistent", nil)
	req.SetPathValue("id", "nonexistent")
	w := httptest.NewRecorder()

	hub.handleGetAgent(w, req)

	if w.Code != http.StatusNotFound {
		t.Errorf("expected status 404, got %d", w.Code)
	}
}

func TestHub_handleSendMessage_MissingContent(t *testing.T) {
	repoDir, workDir, cleanup := setupTestRepo(t)
	defer cleanup()

	hub, _ := New(Config{
		RepoPath:     repoDir,
		WorktreesDir: workDir,
	})

	body := bytes.NewBufferString(`{}`)
	req := httptest.NewRequest("POST", "/agents/test/message", body)
	req.SetPathValue("id", "test")
	w := httptest.NewRecorder()

	hub.handleSendMessage(w, req)

	if w.Code != http.StatusBadRequest {
		t.Errorf("expected status 400, got %d", w.Code)
	}
}

func TestHub_handleSendMessage_AgentNotFound(t *testing.T) {
	repoDir, workDir, cleanup := setupTestRepo(t)
	defer cleanup()

	hub, _ := New(Config{
		RepoPath:     repoDir,
		WorktreesDir: workDir,
	})

	body := bytes.NewBufferString(`{"content": "hello"}`)
	req := httptest.NewRequest("POST", "/agents/nonexistent/message", body)
	req.SetPathValue("id", "nonexistent")
	w := httptest.NewRecorder()

	hub.handleSendMessage(w, req)

	if w.Code != http.StatusNotFound {
		t.Errorf("expected status 404, got %d", w.Code)
	}
}

func TestEventHub(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	hub := NewEventHub()
	go hub.Run(ctx)

	// Allow hub to start
	time.Sleep(10 * time.Millisecond)

	// Subscribe
	client := hub.Subscribe()
	time.Sleep(10 * time.Millisecond)

	if hub.ClientCount() != 1 {
		t.Errorf("expected 1 client, got %d", hub.ClientCount())
	}

	// Broadcast event
	hub.Broadcast(Event{
		Type: EventAgentSpawned,
		Data: map[string]string{"id": "test"},
	})

	// Receive event
	select {
	case event := <-client:
		if event.Type != EventAgentSpawned {
			t.Errorf("expected EventAgentSpawned, got %s", event.Type)
		}
	case <-time.After(time.Second):
		t.Error("timeout waiting for event")
	}

	// Unsubscribe
	hub.Unsubscribe(client)
	time.Sleep(10 * time.Millisecond)

	if hub.ClientCount() != 0 {
		t.Errorf("expected 0 clients, got %d", hub.ClientCount())
	}
}

func TestDefaultConfig(t *testing.T) {
	cfg := DefaultConfig()

	if cfg.Port != 7433 {
		t.Errorf("expected port 7433, got %d", cfg.Port)
	}

	if cfg.BasePort != 7440 {
		t.Errorf("expected base port 7440, got %d", cfg.BasePort)
	}

	if !cfg.Sandbox {
		t.Error("expected sandbox to be true by default")
	}
}

func TestHub_withMiddleware(t *testing.T) {
	repoDir, workDir, cleanup := setupTestRepo(t)
	defer cleanup()

	hub, _ := New(Config{
		RepoPath:     repoDir,
		WorktreesDir: workDir,
	})

	handler := hub.withMiddleware(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	}))

	// Test CORS preflight
	req := httptest.NewRequest("OPTIONS", "/", nil)
	w := httptest.NewRecorder()
	handler.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected status 200 for OPTIONS, got %d", w.Code)
	}

	if w.Header().Get("Access-Control-Allow-Origin") != "*" {
		t.Error("missing CORS header")
	}

	// Test regular request
	req = httptest.NewRequest("GET", "/", nil)
	w = httptest.NewRecorder()
	handler.ServeHTTP(w, req)

	if w.Header().Get("Content-Type") != "application/json" {
		t.Error("missing Content-Type header")
	}
}
