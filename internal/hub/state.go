// Package hub provides state persistence for the Hive Hub.
package hub

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/mbourmaud/hive/internal/agent"
	"github.com/mbourmaud/hive/internal/port"
	"github.com/mbourmaud/hive/internal/solicitation"
	"github.com/mbourmaud/hive/internal/task"
)

const (
	// DefaultStateFile is the default filename for state persistence.
	DefaultStateFile = "hub-state.json"
)

// PersistentState represents the complete Hub state that can be saved/restored.
type PersistentState struct {
	Version       int                          `json:"version"`
	SavedAt       time.Time                    `json:"saved_at"`
	Agents        []AgentState                 `json:"agents"`
	Tasks         []*task.Task                 `json:"tasks"`
	Solicitations []*solicitation.Solicitation `json:"solicitations"`
	Ports         []port.PortLease             `json:"ports"`
}

// AgentState represents a serializable agent state.
type AgentState struct {
	ID           string            `json:"id"`
	Name         string            `json:"name"`
	WorktreePath string            `json:"worktree_path"`
	Branch       string            `json:"branch"`
	Port         int               `json:"port"`
	PID          int               `json:"pid"`
	Status       agent.AgentStatus `json:"status"`
	Specialty    string            `json:"specialty,omitempty"`
	CreatedAt    time.Time         `json:"created_at"`
}

// StateManager handles loading and saving Hub state.
type StateManager struct {
	statePath string
	mu        sync.RWMutex
}

// NewStateManager creates a new state manager.
func NewStateManager(repoPath string) *StateManager {
	hivePath := filepath.Join(repoPath, ".hive")
	return &StateManager{
		statePath: filepath.Join(hivePath, DefaultStateFile),
	}
}

// SetStatePath sets a custom state file path.
func (sm *StateManager) SetStatePath(path string) {
	sm.mu.Lock()
	defer sm.mu.Unlock()
	sm.statePath = path
}

// StatePath returns the current state file path.
func (sm *StateManager) StatePath() string {
	sm.mu.RLock()
	defer sm.mu.RUnlock()
	return sm.statePath
}

// SaveState persists the current Hub state to disk.
func (sm *StateManager) SaveState(h *Hub) error {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	// Ensure directory exists
	dir := filepath.Dir(sm.statePath)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("failed to create state directory: %w", err)
	}

	// Collect agent states (only active agents - ready or busy)
	agents := h.agentManager.ListAgents()
	agentStates := make([]AgentState, 0, len(agents))
	for _, a := range agents {
		// Skip stopped/error agents - they shouldn't be persisted
		if a.Status == agent.StatusStopped || a.Status == agent.StatusError {
			continue
		}
		agentStates = append(agentStates, AgentState{
			ID:           a.ID,
			Name:         a.Name,
			WorktreePath: a.WorktreePath,
			Branch:       a.Branch,
			Port:         a.Port,
			PID:          a.PID,
			Status:       a.Status,
			Specialty:    a.Specialty,
			CreatedAt:    a.CreatedAt,
		})
	}

	// Collect tasks (all of them)
	tasks := h.taskManager.List("", "")

	// Collect solicitations (pending ones)
	solicitations := h.solicitationMgr.ListPending()

	// Collect port leases
	ports := h.portRegistry.ListLeases()

	state := PersistentState{
		Version:       1,
		SavedAt:       time.Now(),
		Agents:        agentStates,
		Tasks:         tasks,
		Solicitations: solicitations,
		Ports:         ports,
	}

	data, err := json.MarshalIndent(state, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal state: %w", err)
	}

	// Write to temp file first, then rename (atomic)
	tmpPath := sm.statePath + ".tmp"
	if err := os.WriteFile(tmpPath, data, 0644); err != nil {
		return fmt.Errorf("failed to write state file: %w", err)
	}

	if err := os.Rename(tmpPath, sm.statePath); err != nil {
		os.Remove(tmpPath) // Clean up
		return fmt.Errorf("failed to rename state file: %w", err)
	}

	return nil
}

// LoadState loads the persisted Hub state from disk.
func (sm *StateManager) LoadState() (*PersistentState, error) {
	sm.mu.RLock()
	defer sm.mu.RUnlock()

	data, err := os.ReadFile(sm.statePath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil // No state file, clean start
		}
		return nil, fmt.Errorf("failed to read state file: %w", err)
	}

	var state PersistentState
	if err := json.Unmarshal(data, &state); err != nil {
		return nil, fmt.Errorf("failed to unmarshal state: %w", err)
	}

	return &state, nil
}

// DeleteState removes the state file.
func (sm *StateManager) DeleteState() error {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	if err := os.Remove(sm.statePath); err != nil && !os.IsNotExist(err) {
		return fmt.Errorf("failed to delete state file: %w", err)
	}
	return nil
}

// Exists checks if a state file exists.
func (sm *StateManager) Exists() bool {
	sm.mu.RLock()
	defer sm.mu.RUnlock()

	_, err := os.Stat(sm.statePath)
	return err == nil
}
