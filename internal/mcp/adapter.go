package mcp

import (
	"context"

	"github.com/mbourmaud/hive/internal/agent"
	"github.com/mbourmaud/hive/internal/hub"
	"github.com/mbourmaud/hive/internal/port"
	"github.com/mbourmaud/hive/internal/solicitation"
	"github.com/mbourmaud/hive/internal/task"
)

// HubAdapter adapts the Hub to the HubInterface required by the MCP server.
type HubAdapter struct {
	agentManager    *agent.Manager
	taskManager     *task.Manager
	solicitationMgr *solicitation.Manager
	portRegistry    *port.Registry
	repoPath        string
	hubURL          string
}

// NewHubAdapter creates a new adapter for the Hub.
func NewHubAdapter(h *hub.Hub, repoPath, hubURL string) *HubAdapter {
	return &HubAdapter{
		agentManager:    h.AgentManager(),
		taskManager:     h.TaskManager(),
		solicitationMgr: h.SolicitationManager(),
		portRegistry:    h.PortRegistry(),
		repoPath:        repoPath,
		hubURL:          hubURL,
	}
}

// SpawnAgent spawns a new agent.
func (a *HubAdapter) SpawnAgent(ctx context.Context, opts agent.SpawnOptions) (*agent.Agent, error) {
	// Fill in required fields from adapter config
	opts.RepoPath = a.repoPath
	opts.HubURL = a.hubURL
	return a.agentManager.SpawnAgent(ctx, opts)
}

// StopAgent stops an agent.
func (a *HubAdapter) StopAgent(ctx context.Context, id string) error {
	return a.agentManager.StopAgent(ctx, id)
}

// DestroyAgent destroys an agent.
func (a *HubAdapter) DestroyAgent(ctx context.Context, id string) error {
	return a.agentManager.DestroyAgent(ctx, id)
}

// GetAgent gets an agent by ID.
func (a *HubAdapter) GetAgent(id string) (*agent.Agent, error) {
	return a.agentManager.GetAgent(id)
}

// ListAgents lists all agents.
func (a *HubAdapter) ListAgents() []*agent.Agent {
	return a.agentManager.ListAgents()
}

// SendMessage sends a message to an agent.
func (a *HubAdapter) SendMessage(ctx context.Context, agentID, message string) error {
	return a.agentManager.SendMessage(ctx, agentID, message)
}

// GetConversation gets the conversation with an agent.
func (a *HubAdapter) GetConversation(ctx context.Context, agentID string) ([]agent.Message, error) {
	return a.agentManager.GetConversation(ctx, agentID)
}

// GetAgentStatus gets an agent's status.
func (a *HubAdapter) GetAgentStatus(agentID string) (string, error) {
	ag, err := a.agentManager.GetAgent(agentID)
	if err != nil {
		return "", err
	}
	return string(ag.Status), nil
}

// CreateTask creates a new task.
func (a *HubAdapter) CreateTask(_ context.Context, req task.CreateTaskRequest) (*task.Task, error) {
	return a.taskManager.Create(req)
}

// GetTask gets a task by ID.
func (a *HubAdapter) GetTask(id string) (*task.Task, error) {
	return a.taskManager.Get(id)
}

// ListTasks lists tasks.
func (a *HubAdapter) ListTasks(agentID string, status task.TaskStatus) []*task.Task {
	return a.taskManager.List(agentID, status)
}

// StartTask starts a task.
func (a *HubAdapter) StartTask(_ context.Context, id string) error {
	return a.taskManager.Start(id)
}

// CompleteTask completes a task.
func (a *HubAdapter) CompleteTask(_ context.Context, id, result string) error {
	return a.taskManager.Complete(id, task.CompleteTaskRequest{Result: result})
}

// FailTask fails a task.
func (a *HubAdapter) FailTask(_ context.Context, id, errorMsg string) error {
	return a.taskManager.Fail(id, task.FailTaskRequest{Error: errorMsg})
}

// CancelTask cancels a task.
func (a *HubAdapter) CancelTask(_ context.Context, id string) error {
	return a.taskManager.Cancel(id, "cancelled by queen")
}

// GetPendingSolicitations gets pending solicitations.
func (a *HubAdapter) GetPendingSolicitations() []*solicitation.Solicitation {
	return a.solicitationMgr.ListPending()
}

// GetSolicitation gets a solicitation by ID.
func (a *HubAdapter) GetSolicitation(id string) (*solicitation.Solicitation, error) {
	return a.solicitationMgr.Get(id)
}

// RespondToSolicitation responds to a solicitation.
func (a *HubAdapter) RespondToSolicitation(_ context.Context, id, response string) error {
	return a.solicitationMgr.Respond(id, solicitation.RespondRequest{Response: response})
}

// DismissSolicitation dismisses a solicitation.
func (a *HubAdapter) DismissSolicitation(_ context.Context, id string) error {
	return a.solicitationMgr.Dismiss(id, solicitation.DismissRequest{})
}

// ListPorts lists port allocations.
func (a *HubAdapter) ListPorts() ([]port.PortLease, []port.PortWaiter) {
	return a.portRegistry.ListLeases(), a.portRegistry.ListWaiters()
}

// ForceReleasePort force releases a port.
func (a *HubAdapter) ForceReleasePort(portNum int) error {
	return a.portRegistry.ForceRelease(port.ForceReleaseRequest{Port: portNum})
}

// GetStatus gets the hub status.
func (a *HubAdapter) GetStatus() StatusInfo {
	return StatusInfo{
		AgentsTotal:          a.agentManager.Count(),
		AgentsRunning:        a.agentManager.CountRunning(),
		TasksTotal:           len(a.taskManager.List("", "")),
		SolicitationsPending: len(a.solicitationMgr.ListPending()),
		PortsLeased:          len(a.portRegistry.ListLeases()),
	}
}
