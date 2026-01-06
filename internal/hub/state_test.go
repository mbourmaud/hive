package hub

import (
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/mbourmaud/hive/internal/agent"
)

func TestStateManager_SaveAndLoad(t *testing.T) {
	tmpDir := t.TempDir()
	sm := NewStateManager(tmpDir)

	cfg := Config{
		Port:     8080,
		RepoPath: tmpDir,
	}

	hub, err := New(cfg)
	if err != nil {
		t.Fatalf("Failed to create hub: %v", err)
	}

	testAgent := &agent.Agent{
		ID:           "test-123",
		Name:         "test-agent",
		WorktreePath: "/tmp/test-worktree",
		Branch:       "feature/test",
		Port:         3284,
		PID:          12345,
		Status:       agent.StatusReady,
		Specialty:    "backend",
		CreatedAt:    time.Now(),
	}

	hub.agentManager.RegisterAgent(testAgent)

	if err := sm.SaveState(hub); err != nil {
		t.Fatalf("Failed to save state: %v", err)
	}

	statePath := filepath.Join(tmpDir, ".hive", DefaultStateFile)
	if _, err := os.Stat(statePath); os.IsNotExist(err) {
		t.Fatalf("State file was not created at %s", statePath)
	}

	state, err := sm.LoadState()
	if err != nil {
		t.Fatalf("Failed to load state: %v", err)
	}

	if state == nil {
		t.Fatal("Loaded state is nil")
	}

	if state.Version != 1 {
		t.Errorf("Expected version 1, got %d", state.Version)
	}

	if len(state.Agents) != 1 {
		t.Fatalf("Expected 1 agent, got %d", len(state.Agents))
	}

	loadedAgent := state.Agents[0]
	if loadedAgent.ID != testAgent.ID {
		t.Errorf("Agent ID mismatch: expected %s, got %s", testAgent.ID, loadedAgent.ID)
	}
	if loadedAgent.Name != testAgent.Name {
		t.Errorf("Agent Name mismatch: expected %s, got %s", testAgent.Name, loadedAgent.Name)
	}
	if loadedAgent.Port != testAgent.Port {
		t.Errorf("Agent Port mismatch: expected %d, got %d", testAgent.Port, loadedAgent.Port)
	}
	if loadedAgent.PID != testAgent.PID {
		t.Errorf("Agent PID mismatch: expected %d, got %d", testAgent.PID, loadedAgent.PID)
	}
}

func TestStateManager_LoadNonExistent(t *testing.T) {
	tmpDir := t.TempDir()
	sm := NewStateManager(tmpDir)

	state, err := sm.LoadState()
	if err != nil {
		t.Fatalf("Expected no error for non-existent state, got: %v", err)
	}

	if state != nil {
		t.Errorf("Expected nil state for non-existent file, got: %+v", state)
	}
}

func TestStateManager_Exists(t *testing.T) {
	tmpDir := t.TempDir()
	sm := NewStateManager(tmpDir)

	if sm.Exists() {
		t.Error("Expected Exists() to return false for non-existent state")
	}

	cfg := Config{
		Port:     8080,
		RepoPath: tmpDir,
	}

	hub, err := New(cfg)
	if err != nil {
		t.Fatalf("Failed to create hub: %v", err)
	}

	if err := sm.SaveState(hub); err != nil {
		t.Fatalf("Failed to save state: %v", err)
	}

	if !sm.Exists() {
		t.Error("Expected Exists() to return true after saving state")
	}
}

func TestStateManager_DeleteState(t *testing.T) {
	tmpDir := t.TempDir()
	sm := NewStateManager(tmpDir)

	cfg := Config{
		Port:     8080,
		RepoPath: tmpDir,
	}

	hub, err := New(cfg)
	if err != nil {
		t.Fatalf("Failed to create hub: %v", err)
	}

	if err := sm.SaveState(hub); err != nil {
		t.Fatalf("Failed to save state: %v", err)
	}

	if !sm.Exists() {
		t.Fatal("State file should exist after saving")
	}

	if err := sm.DeleteState(); err != nil {
		t.Fatalf("Failed to delete state: %v", err)
	}

	if sm.Exists() {
		t.Error("State file should not exist after deletion")
	}
}
