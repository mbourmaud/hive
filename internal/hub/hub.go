// Package hub provides the central Hive server for managing agents.
package hub

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"sync"
	"syscall"
	"time"

	"github.com/mbourmaud/hive/internal/agent"
	"github.com/mbourmaud/hive/internal/port"
	"github.com/mbourmaud/hive/internal/solicitation"
	"github.com/mbourmaud/hive/internal/task"
	"github.com/mbourmaud/hive/internal/worktree"
)

// Config holds the hub configuration.
type Config struct {
	Port         int    `yaml:"port"`
	WorktreesDir string `yaml:"worktrees_dir"`
	BasePort     int    `yaml:"base_port"`
	RepoPath     string `yaml:"repo_path"`
	Sandbox      bool   `yaml:"sandbox"`
}

// DefaultConfig returns the default hub configuration.
func DefaultConfig() Config {
	return Config{
		Port:         8080,
		WorktreesDir: "",
		BasePort:     3284,
		Sandbox:      true,
	}
}

// Hub is the central server that manages agents, tasks, and resources.
type Hub struct {
	config          Config
	agentManager    *agent.Manager
	worktreeMgr     worktree.Manager
	portRegistry    *port.Registry
	taskManager     *task.Manager
	solicitationMgr *solicitation.Manager
	stateManager    *StateManager
	server          *http.Server
	eventHub        *EventHub
	mu              sync.RWMutex
}

// New creates a new Hub instance.
func New(cfg Config) (*Hub, error) {
	if cfg.RepoPath == "" {
		return nil, fmt.Errorf("repo_path is required")
	}

	eventHub := NewEventHub()

	// Create hub first so we can wire up event handlers
	h := &Hub{
		config:   cfg,
		eventHub: eventHub,
	}

	// Port registry with event handler
	h.portRegistry = port.NewRegistry(func(e port.PortEvent) {
		h.eventHub.Broadcast(Event{
			Type: portEventType(e.Type),
			Data: e,
		})
	})

	// Task manager with event handler
	h.taskManager = task.NewManager(func(e task.TaskEvent) {
		h.eventHub.Broadcast(Event{
			Type: taskEventType(e.Type),
			Data: e,
		})
	})

	// Solicitation manager with event handler
	h.solicitationMgr = solicitation.NewManager(func(e solicitation.Event) {
		h.eventHub.Broadcast(Event{
			Type: solicitationEventType(e.Type),
			Data: e,
		})
	})

	// Worktree and agent managers
	worktreeMgr := worktree.NewGitManager(cfg.RepoPath, cfg.WorktreesDir)
	client := agent.NewHTTPClient()
	spawner := agent.NewProcessSpawner(worktreeMgr, client)

	if cfg.BasePort > 0 {
		spawner.SetBasePort(cfg.BasePort)
	}

	h.worktreeMgr = worktreeMgr
	h.agentManager = agent.NewManager(spawner, client)
	h.stateManager = NewStateManager(cfg.RepoPath)

	return h, nil
}

// Start starts the hub server.
func (h *Hub) Start(ctx context.Context) error {
	// Restore state from previous run
	if err := h.restoreState(ctx); err != nil {
		// Log but continue - state restoration is best-effort
		fmt.Printf("Warning: failed to restore state: %v\n", err)
	}

	mux := http.NewServeMux()

	// Agent endpoints
	mux.HandleFunc("GET /agents", h.handleListAgents)
	mux.HandleFunc("POST /agents", h.handleSpawnAgent)
	mux.HandleFunc("GET /agents/{id}", h.handleGetAgent)
	mux.HandleFunc("DELETE /agents/{id}", h.handleStopAgent)
	mux.HandleFunc("DELETE /agents/{id}/destroy", h.handleDestroyAgent)

	// Message endpoints
	mux.HandleFunc("POST /agents/{id}/message", h.handleSendMessage)
	mux.HandleFunc("GET /agents/{id}/messages", h.handleGetConversation)
	mux.HandleFunc("GET /agents/{id}/conversation", h.handleGetConversation)
	mux.HandleFunc("GET /agents/{id}/status", h.handleGetAgentStatus)

	// Task endpoints
	mux.HandleFunc("GET /tasks", h.handleListTasks)
	mux.HandleFunc("POST /tasks", h.handleCreateTask)
	mux.HandleFunc("GET /tasks/{id}", h.handleGetTask)
	mux.HandleFunc("POST /tasks/{id}/start", h.handleStartTask)
	mux.HandleFunc("PUT /tasks/{id}/steps/{step}", h.handleUpdateStep)
	mux.HandleFunc("POST /tasks/{id}/complete", h.handleCompleteTask)
	mux.HandleFunc("POST /tasks/{id}/fail", h.handleFailTask)
	mux.HandleFunc("DELETE /tasks/{id}", h.handleCancelTask)

	// Solicitation endpoints
	mux.HandleFunc("GET /solicitations", h.handleListSolicitations)
	mux.HandleFunc("POST /solicitations", h.handleCreateSolicitation)
	mux.HandleFunc("GET /solicitations/{id}", h.handleGetSolicitation)
	mux.HandleFunc("POST /solicitations/{id}/respond", h.handleRespondSolicitation)
	mux.HandleFunc("POST /solicitations/{id}/dismiss", h.handleDismissSolicitation)

	// Port endpoints
	mux.HandleFunc("GET /ports", h.handleListPorts)
	mux.HandleFunc("GET /ports/{port}", h.handleGetPort)
	mux.HandleFunc("POST /ports/acquire", h.handleAcquirePort)
	mux.HandleFunc("POST /ports/release", h.handleReleasePort)
	mux.HandleFunc("POST /ports/{port}/force-release", h.handleForceReleasePort)

	// WebSocket endpoint
	mux.HandleFunc("GET /ws", h.handleWebSocket)

	// Status endpoints
	mux.HandleFunc("GET /health", h.handleHealth)
	mux.HandleFunc("GET /status", h.handleStatus)

	h.server = &http.Server{
		Addr:              fmt.Sprintf(":%d", h.config.Port),
		Handler:           h.withMiddleware(mux),
		ReadHeaderTimeout: 10 * time.Second,
	}

	// Start event hub
	go h.eventHub.Run(ctx)

	return h.server.ListenAndServe()
}

// Stop gracefully stops the hub server.
func (h *Hub) Stop(ctx context.Context) error {
	_ = h.SaveState()

	if err := h.agentManager.StopAll(ctx); err != nil {
		fmt.Printf("Warning: failed to stop all agents: %v\n", err)
	}

	if h.server != nil {
		return h.server.Shutdown(ctx)
	}
	return nil
}

// AgentManager returns the agent manager.
func (h *Hub) AgentManager() *agent.Manager {
	return h.agentManager
}

// PortRegistry returns the port registry.
func (h *Hub) PortRegistry() *port.Registry {
	return h.portRegistry
}

// TaskManager returns the task manager.
func (h *Hub) TaskManager() *task.Manager {
	return h.taskManager
}

// SolicitationManager returns the solicitation manager.
func (h *Hub) SolicitationManager() *solicitation.Manager {
	return h.solicitationMgr
}

// withMiddleware adds common middleware to all requests.
func (h *Hub) withMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// CORS headers
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")

		if r.Method == "OPTIONS" {
			w.WriteHeader(http.StatusOK)
			return
		}

		// Content-Type for JSON responses
		w.Header().Set("Content-Type", "application/json")

		next.ServeHTTP(w, r)
	})
}

// JSON response helpers
func (h *Hub) jsonResponse(w http.ResponseWriter, status int, data interface{}) {
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(data)
}

func (h *Hub) jsonError(w http.ResponseWriter, status int, message string) {
	h.jsonResponse(w, status, map[string]string{"error": message})
}

func (h *Hub) restoreState(ctx context.Context) error {
	state, err := h.stateManager.LoadState()
	if err != nil {
		return err
	}
	if state == nil {
		return nil
	}

	restored := 0
	failed := 0

	for _, as := range state.Agents {
		a := &agent.Agent{
			ID:           as.ID,
			Name:         as.Name,
			WorktreePath: as.WorktreePath,
			Branch:       as.Branch,
			Port:         as.Port,
			PID:          as.PID,
			Status:       as.Status,
			Specialty:    as.Specialty,
			CreatedAt:    as.CreatedAt,
		}

		if h.isAgentAlive(ctx, a) {
			a.Status = agent.StatusReady
			h.agentManager.RegisterAgent(a)
			restored++
		} else {
			a.Status = agent.StatusStopped
			failed++
		}
	}

	if restored > 0 || failed > 0 {
		fmt.Printf("State restored: %d agents reconnected, %d agents dead\n", restored, failed)
	}

	return nil
}

func (h *Hub) isAgentAlive(ctx context.Context, a *agent.Agent) bool {
	if a.PID <= 0 {
		return false
	}

	process, err := os.FindProcess(a.PID)
	if err != nil {
		return false
	}
	if err := process.Signal(syscall.Signal(0)); err != nil {
		return false
	}

	status, err := agent.NewHTTPClient().GetStatus(ctx, a.Port)
	if err != nil {
		return false
	}

	return status == agent.StatusReady || status == agent.StatusBusy
}

func (h *Hub) SaveState() error {
	return h.stateManager.SaveState(h)
}

// Event type mapping helpers
func portEventType(t string) EventType {
	switch t {
	case "acquired":
		return EventPortAcquired
	case "released":
		return EventPortReleased
	case "waiting":
		return EventPortWaiting
	case "timeout":
		return EventPortTimeout
	case "conflict":
		return EventPortConflict
	default:
		return EventType("port." + t)
	}
}

func taskEventType(t string) EventType {
	switch t {
	case "created":
		return EventTaskCreated
	case "started":
		return EventTaskStarted
	case "progress":
		return EventTaskProgress
	case "completed":
		return EventTaskCompleted
	case "failed":
		return EventTaskFailed
	case "cancelled":
		return EventTaskCancelled
	default:
		return EventType("task." + t)
	}
}

func solicitationEventType(t string) EventType {
	switch t {
	case "new":
		return EventSolicitationNew
	case "responded":
		return EventSolicitationResponded
	case "dismissed":
		return EventSolicitationDismissed
	case "expired":
		return EventSolicitationExpired
	default:
		return EventType("solicitation." + t)
	}
}
