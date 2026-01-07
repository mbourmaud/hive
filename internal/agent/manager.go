package agent

import (
	"context"
	"fmt"
	"sync"
)

// Manager orchestrates multiple agents and their lifecycle.
type Manager struct {
	agents  map[string]*Agent
	spawner Spawner
	client  Client
	mu      sync.RWMutex
}

// NewManager creates a new agent manager.
func NewManager(spawner Spawner, client Client) *Manager {
	return &Manager{
		agents:  make(map[string]*Agent),
		spawner: spawner,
		client:  client,
	}
}

// SpawnAgent creates and registers a new agent.
func (m *Manager) SpawnAgent(ctx context.Context, opts SpawnOptions) (*Agent, error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	// Check if agent with same name already exists
	for _, agent := range m.agents {
		if agent.Name == opts.Name && agent.IsRunning() {
			return nil, fmt.Errorf("agent %s already exists", opts.Name)
		}
	}

	agent, err := m.spawner.Spawn(ctx, opts)
	if err != nil {
		return nil, err
	}

	m.agents[agent.ID] = agent
	return agent, nil
}

// StopAgent stops an agent by ID and removes it from the manager.
func (m *Manager) StopAgent(ctx context.Context, id string) error {
	m.mu.Lock()
	agent, ok := m.agents[id]
	if !ok {
		m.mu.Unlock()
		return fmt.Errorf("agent %s not found", id)
	}
	delete(m.agents, id)
	m.mu.Unlock()

	return m.spawner.Stop(ctx, agent)
}

// DestroyAgent stops an agent and removes its worktree.
func (m *Manager) DestroyAgent(ctx context.Context, id string) error {
	m.mu.Lock()
	agent, ok := m.agents[id]
	if !ok {
		m.mu.Unlock()
		return fmt.Errorf("agent %s not found", id)
	}
	delete(m.agents, id)
	m.mu.Unlock()

	return m.spawner.Destroy(ctx, agent)
}

// GetAgent returns an agent by ID.
func (m *Manager) GetAgent(id string) (*Agent, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	agent, ok := m.agents[id]
	if !ok {
		return nil, fmt.Errorf("agent %s not found", id)
	}

	return agent, nil
}

// GetAgentByName returns an agent by name.
func (m *Manager) GetAgentByName(name string) (*Agent, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	for _, agent := range m.agents {
		if agent.Name == name {
			return agent, nil
		}
	}

	return nil, fmt.Errorf("agent %s not found", name)
}

// ListAgents returns all registered agents.
func (m *Manager) ListAgents() []*Agent {
	m.mu.RLock()
	defer m.mu.RUnlock()

	agents := make([]*Agent, 0, len(m.agents))
	for _, agent := range m.agents {
		agents = append(agents, agent)
	}

	return agents
}

// ListRunningAgents returns only running agents.
func (m *Manager) ListRunningAgents() []*Agent {
	m.mu.RLock()
	defer m.mu.RUnlock()

	agents := make([]*Agent, 0)
	for _, agent := range m.agents {
		if agent.IsRunning() {
			agents = append(agents, agent)
		}
	}

	return agents
}

// SendMessage sends a message to an agent.
func (m *Manager) SendMessage(ctx context.Context, id string, message string) error {
	m.mu.RLock()
	agent, ok := m.agents[id]
	m.mu.RUnlock()

	if !ok {
		return fmt.Errorf("agent %s not found", id)
	}

	if !agent.IsRunning() {
		return fmt.Errorf("agent %s is not running", id)
	}

	return m.client.SendMessage(ctx, agent.Port, message)
}

// SendMessageByName sends a message to an agent by name.
func (m *Manager) SendMessageByName(ctx context.Context, name string, message string) error {
	agent, err := m.GetAgentByName(name)
	if err != nil {
		return err
	}

	return m.SendMessage(ctx, agent.ID, message)
}

// GetConversation retrieves the conversation history for an agent.
func (m *Manager) GetConversation(ctx context.Context, id string) ([]Message, error) {
	m.mu.RLock()
	agent, ok := m.agents[id]
	m.mu.RUnlock()

	if !ok {
		return nil, fmt.Errorf("agent %s not found", id)
	}

	return m.client.GetMessages(ctx, agent.Port)
}

// RefreshStatus updates the status of an agent from AgentAPI.
func (m *Manager) RefreshStatus(ctx context.Context, id string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	agent, ok := m.agents[id]
	if !ok {
		return fmt.Errorf("agent %s not found", id)
	}

	status, err := m.client.GetStatus(ctx, agent.Port)
	if err != nil {
		agent.Status = StatusError
		agent.Error = err.Error()
		return err
	}

	agent.Status = status
	agent.Error = ""
	return nil
}

// RefreshAllStatus updates the status of all agents.
func (m *Manager) RefreshAllStatus(ctx context.Context) {
	m.mu.RLock()
	ids := make([]string, 0, len(m.agents))
	for id := range m.agents {
		ids = append(ids, id)
	}
	m.mu.RUnlock()

	for _, id := range ids {
		m.RefreshStatus(ctx, id)
	}
}

// StopAll stops all running agents.
func (m *Manager) StopAll(ctx context.Context) error {
	agents := m.ListRunningAgents()

	var lastErr error
	for _, agent := range agents {
		if err := m.StopAgent(ctx, agent.ID); err != nil {
			lastErr = err
		}
	}

	return lastErr
}

// DestroyAll destroys all agents and their worktrees.
func (m *Manager) DestroyAll(ctx context.Context) error {
	m.mu.Lock()
	agents := make([]*Agent, 0, len(m.agents))
	for _, agent := range m.agents {
		agents = append(agents, agent)
	}
	m.mu.Unlock()

	var lastErr error
	for _, agent := range agents {
		if err := m.DestroyAgent(ctx, agent.ID); err != nil {
			lastErr = err
		}
	}

	return lastErr
}

// Count returns the number of registered agents.
func (m *Manager) Count() int {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return len(m.agents)
}

// RegisterAgent registers an existing agent (used for state restoration).
// The agent is added to the manager but no process is spawned.
func (m *Manager) RegisterAgent(agent *Agent) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.agents[agent.ID] = agent
}

// UnregisterAgent removes an agent from the manager without stopping it.
func (m *Manager) UnregisterAgent(id string) {
	m.mu.Lock()
	defer m.mu.Unlock()
	delete(m.agents, id)
}

// CountRunning returns the number of running agents.
func (m *Manager) CountRunning() int {
	m.mu.RLock()
	defer m.mu.RUnlock()

	count := 0
	for _, agent := range m.agents {
		if agent.IsRunning() {
			count++
		}
	}

	return count
}
