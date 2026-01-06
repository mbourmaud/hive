package agent

import (
	"context"
	"testing"
	"time"
)

func TestManager_SpawnAgent(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	agent, err := mgr.SpawnAgent(ctx, SpawnOptions{
		Name:      "test-agent",
		Branch:    "main",
		Specialty: "fullstack",
	})
	if err != nil {
		t.Fatalf("SpawnAgent failed: %v", err)
	}

	if agent.Name != "test-agent" {
		t.Errorf("expected name test-agent, got %s", agent.Name)
	}

	if agent.Status != StatusReady {
		t.Errorf("expected status ready, got %s", agent.Status)
	}

	// Spawning same name should fail
	_, err = mgr.SpawnAgent(ctx, SpawnOptions{Name: "test-agent"})
	if err == nil {
		t.Error("spawning duplicate agent should fail")
	}
}

func TestManager_GetAgent(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	agent, _ := mgr.SpawnAgent(ctx, SpawnOptions{Name: "findme"})

	// Get by ID
	found, err := mgr.GetAgent(agent.ID)
	if err != nil {
		t.Fatalf("GetAgent failed: %v", err)
	}
	if found.ID != agent.ID {
		t.Error("agent ID mismatch")
	}

	// Get non-existent
	_, err = mgr.GetAgent("notfound")
	if err == nil {
		t.Error("getting non-existent agent should fail")
	}
}

func TestManager_GetAgentByName(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	agent, _ := mgr.SpawnAgent(ctx, SpawnOptions{Name: "named-agent"})

	// Get by name
	found, err := mgr.GetAgentByName("named-agent")
	if err != nil {
		t.Fatalf("GetAgentByName failed: %v", err)
	}
	if found.ID != agent.ID {
		t.Error("agent ID mismatch")
	}

	// Get non-existent
	_, err = mgr.GetAgentByName("notfound")
	if err == nil {
		t.Error("getting non-existent agent should fail")
	}
}

func TestManager_ListAgents(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	mgr.SpawnAgent(ctx, SpawnOptions{Name: "agent-1"})
	mgr.SpawnAgent(ctx, SpawnOptions{Name: "agent-2"})
	mgr.SpawnAgent(ctx, SpawnOptions{Name: "agent-3"})

	agents := mgr.ListAgents()
	if len(agents) != 3 {
		t.Errorf("expected 3 agents, got %d", len(agents))
	}
}

func TestManager_ListRunningAgents(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	agent1, _ := mgr.SpawnAgent(ctx, SpawnOptions{Name: "running-1"})
	mgr.SpawnAgent(ctx, SpawnOptions{Name: "running-2"})

	// Stop one agent
	mgr.StopAgent(ctx, agent1.ID)

	running := mgr.ListRunningAgents()
	if len(running) != 1 {
		t.Errorf("expected 1 running agent, got %d", len(running))
	}
}

func TestManager_StopAgent(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	agent, _ := mgr.SpawnAgent(ctx, SpawnOptions{Name: "to-stop"})

	err := mgr.StopAgent(ctx, agent.ID)
	if err != nil {
		t.Fatalf("StopAgent failed: %v", err)
	}

	// Verify status changed
	found, _ := mgr.GetAgent(agent.ID)
	if found.Status != StatusStopped {
		t.Errorf("expected status stopped, got %s", found.Status)
	}

	// Stop non-existent
	err = mgr.StopAgent(ctx, "notfound")
	if err == nil {
		t.Error("stopping non-existent agent should fail")
	}
}

func TestManager_DestroyAgent(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	agent, _ := mgr.SpawnAgent(ctx, SpawnOptions{Name: "to-destroy"})

	err := mgr.DestroyAgent(ctx, agent.ID)
	if err != nil {
		t.Fatalf("DestroyAgent failed: %v", err)
	}

	// Agent should be removed
	_, err = mgr.GetAgent(agent.ID)
	if err == nil {
		t.Error("destroyed agent should not be found")
	}
}

func TestManager_SendMessage(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	agent, _ := mgr.SpawnAgent(ctx, SpawnOptions{Name: "message-agent"})

	err := mgr.SendMessage(ctx, agent.ID, "Hello agent")
	if err != nil {
		t.Fatalf("SendMessage failed: %v", err)
	}

	if len(client.Messages) != 1 {
		t.Fatalf("expected 1 message, got %d", len(client.Messages))
	}

	if client.Messages[0].Content != "Hello agent" {
		t.Error("message content mismatch")
	}
}

func TestManager_SendMessageByName(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	mgr.SpawnAgent(ctx, SpawnOptions{Name: "named"})

	err := mgr.SendMessageByName(ctx, "named", "Hello by name")
	if err != nil {
		t.Fatalf("SendMessageByName failed: %v", err)
	}
}

func TestManager_GetConversation(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{
		CurrentStatus: StatusReady,
		Messages: []Message{
			{Role: "user", Content: "Hello"},
			{Role: "assistant", Content: "Hi!"},
		},
	}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	agent, _ := mgr.SpawnAgent(ctx, SpawnOptions{Name: "conv-agent"})

	messages, err := mgr.GetConversation(ctx, agent.ID)
	if err != nil {
		t.Fatalf("GetConversation failed: %v", err)
	}

	if len(messages) != 2 {
		t.Errorf("expected 2 messages, got %d", len(messages))
	}
}

func TestManager_Count(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	if mgr.Count() != 0 {
		t.Error("expected 0 agents initially")
	}

	mgr.SpawnAgent(ctx, SpawnOptions{Name: "count-1"})
	mgr.SpawnAgent(ctx, SpawnOptions{Name: "count-2"})

	if mgr.Count() != 2 {
		t.Errorf("expected 2 agents, got %d", mgr.Count())
	}
}

func TestManager_CountRunning(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	agent1, _ := mgr.SpawnAgent(ctx, SpawnOptions{Name: "run-1"})
	mgr.SpawnAgent(ctx, SpawnOptions{Name: "run-2"})

	if mgr.CountRunning() != 2 {
		t.Errorf("expected 2 running, got %d", mgr.CountRunning())
	}

	mgr.StopAgent(ctx, agent1.ID)

	if mgr.CountRunning() != 1 {
		t.Errorf("expected 1 running, got %d", mgr.CountRunning())
	}
}

func TestManager_StopAll(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	mgr.SpawnAgent(ctx, SpawnOptions{Name: "stop-all-1"})
	mgr.SpawnAgent(ctx, SpawnOptions{Name: "stop-all-2"})
	mgr.SpawnAgent(ctx, SpawnOptions{Name: "stop-all-3"})

	err := mgr.StopAll(ctx)
	if err != nil {
		t.Fatalf("StopAll failed: %v", err)
	}

	if mgr.CountRunning() != 0 {
		t.Errorf("expected 0 running after StopAll, got %d", mgr.CountRunning())
	}
}

func TestManager_DestroyAll(t *testing.T) {
	spawner := NewMockSpawner()
	client := &MockClient{CurrentStatus: StatusReady}
	mgr := NewManager(spawner, client)

	ctx := context.Background()

	mgr.SpawnAgent(ctx, SpawnOptions{Name: "destroy-1"})
	mgr.SpawnAgent(ctx, SpawnOptions{Name: "destroy-2"})

	err := mgr.DestroyAll(ctx)
	if err != nil {
		t.Fatalf("DestroyAll failed: %v", err)
	}

	if mgr.Count() != 0 {
		t.Errorf("expected 0 agents after DestroyAll, got %d", mgr.Count())
	}
}

func TestAgent_IsRunning(t *testing.T) {
	tests := []struct {
		status   AgentStatus
		expected bool
	}{
		{StatusStarting, true},
		{StatusReady, true},
		{StatusBusy, true},
		{StatusStopped, false},
		{StatusError, false},
	}

	for _, tt := range tests {
		t.Run(string(tt.status), func(t *testing.T) {
			agent := &Agent{Status: tt.status}
			if agent.IsRunning() != tt.expected {
				t.Errorf("IsRunning() = %v, expected %v", agent.IsRunning(), tt.expected)
			}
		})
	}
}

func TestAgent_IsStopped(t *testing.T) {
	tests := []struct {
		status   AgentStatus
		expected bool
	}{
		{StatusStarting, false},
		{StatusReady, false},
		{StatusBusy, false},
		{StatusStopped, true},
		{StatusError, true},
	}

	for _, tt := range tests {
		t.Run(string(tt.status), func(t *testing.T) {
			agent := &Agent{Status: tt.status}
			if agent.IsStopped() != tt.expected {
				t.Errorf("IsStopped() = %v, expected %v", agent.IsStopped(), tt.expected)
			}
		})
	}
}

func TestNewHTTPClient(t *testing.T) {
	client := NewHTTPClient()

	if client.baseHost != "localhost" {
		t.Errorf("expected baseHost localhost, got %s", client.baseHost)
	}

	if client.httpClient.Timeout != 30*time.Second {
		t.Errorf("expected 30s timeout, got %v", client.httpClient.Timeout)
	}
}
