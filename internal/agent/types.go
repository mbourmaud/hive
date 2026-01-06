// Package agent provides agent lifecycle management using AgentAPI.
package agent

import (
	"os"
	"time"
)

// AgentStatus represents the current state of an agent.
type AgentStatus string

const (
	// StatusStarting means the agent is being spawned.
	StatusStarting AgentStatus = "starting"
	// StatusReady means the agent is ready to receive messages.
	StatusReady AgentStatus = "ready"
	// StatusBusy means the agent is processing a message.
	StatusBusy AgentStatus = "busy"
	// StatusStopped means the agent has been stopped.
	StatusStopped AgentStatus = "stopped"
	// StatusError means the agent encountered an error.
	StatusError AgentStatus = "error"
)

// Agent represents a running Claude Code agent controlled via AgentAPI.
type Agent struct {
	ID           string      `json:"id"`
	Name         string      `json:"name"`
	WorktreePath string      `json:"worktree_path"`
	Branch       string      `json:"branch"`
	Port         int         `json:"port"`
	PID          int         `json:"pid"`
	Process      *os.Process `json:"-"`
	Status       AgentStatus `json:"status"`
	Specialty    string      `json:"specialty,omitempty"`
	CreatedAt    time.Time   `json:"created_at"`
	Error        string      `json:"error,omitempty"`
}

// Message represents a message in a conversation.
type Message struct {
	Role      string    `json:"role"` // "user" or "assistant"
	Content   string    `json:"content"`
	Timestamp time.Time `json:"timestamp,omitempty"`
}

// SpawnOptions contains options for spawning an agent.
type SpawnOptions struct {
	Name       string // Logical name (e.g., "front", "back")
	RepoPath   string // Path to the git repository
	Branch     string // Branch to work on
	BaseBranch string // Base branch to create from (default: main)
	Specialty  string // Agent specialty (front, back, infra, fullstack)
	Sandbox    bool   // Enable Claude Code sandbox mode
	Model      string // Claude model to use (optional)
	HubURL     string // URL of the Hive Hub for agent commands
}

// StatusResponse represents the response from AgentAPI /status endpoint.
type StatusResponse struct {
	Status string `json:"status"` // "stable" or "running"
}

// MessageRequest represents a message to send to AgentAPI.
type MessageRequest struct {
	Content string `json:"content"`
	Type    string `json:"type"` // "user"
}

// Conversation represents a full conversation with an agent.
type Conversation struct {
	AgentID  string    `json:"agent_id"`
	Messages []Message `json:"messages"`
}

// IsRunning returns true if the agent is in a running state.
func (a *Agent) IsRunning() bool {
	return a.Status == StatusStarting || a.Status == StatusReady || a.Status == StatusBusy
}

// IsStopped returns true if the agent has been stopped.
func (a *Agent) IsStopped() bool {
	return a.Status == StatusStopped || a.Status == StatusError
}
