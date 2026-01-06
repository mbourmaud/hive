package monitor

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestHubClient_GetAgents(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/agents" {
			t.Errorf("Expected /agents path, got %s", r.URL.Path)
		}
		agents := []Agent{
			{ID: "1", Name: "test-agent", Status: "ready", Port: 3284},
		}
		json.NewEncoder(w).Encode(agents)
	}))
	defer server.Close()

	client := NewHubClient(server.URL)
	agents, err := client.GetAgents()
	if err != nil {
		t.Fatalf("GetAgents failed: %v", err)
	}

	if len(agents) != 1 {
		t.Errorf("Expected 1 agent, got %d", len(agents))
	}
	if agents[0].Name != "test-agent" {
		t.Errorf("Expected agent name 'test-agent', got '%s'", agents[0].Name)
	}
}

func TestHubClient_GetTasks(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/tasks" {
			t.Errorf("Expected /tasks path, got %s", r.URL.Path)
		}
		tasks := []Task{
			{ID: "1", AgentID: "agent-1", Status: "in_progress", Title: "Test Task"},
		}
		json.NewEncoder(w).Encode(tasks)
	}))
	defer server.Close()

	client := NewHubClient(server.URL)
	tasks, err := client.GetTasks()
	if err != nil {
		t.Fatalf("GetTasks failed: %v", err)
	}

	if len(tasks) != 1 {
		t.Errorf("Expected 1 task, got %d", len(tasks))
	}
	if tasks[0].Title != "Test Task" {
		t.Errorf("Expected task title 'Test Task', got '%s'", tasks[0].Title)
	}
}

func TestHubClient_GetSolicitations(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/solicitations" {
			t.Errorf("Expected /solicitations path, got %s", r.URL.Path)
		}
		solicitations := []Solicitation{
			{ID: "1", AgentID: "agent-1", Type: "blocker", Urgency: "high", Message: "Help needed"},
		}
		json.NewEncoder(w).Encode(solicitations)
	}))
	defer server.Close()

	client := NewHubClient(server.URL)
	solicitations, err := client.GetSolicitations()
	if err != nil {
		t.Fatalf("GetSolicitations failed: %v", err)
	}

	if len(solicitations) != 1 {
		t.Errorf("Expected 1 solicitation, got %d", len(solicitations))
	}
	if solicitations[0].Type != "blocker" {
		t.Errorf("Expected type 'blocker', got '%s'", solicitations[0].Type)
	}
}

func TestHubClient_GetConversation(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/agents/agent-1/conversation" {
			t.Errorf("Expected /agents/agent-1/conversation path, got %s", r.URL.Path)
		}
		messages := []Message{
			{Role: "user", Content: "Hello"},
			{Role: "assistant", Content: "Hi there"},
		}
		json.NewEncoder(w).Encode(messages)
	}))
	defer server.Close()

	client := NewHubClient(server.URL)
	messages, err := client.GetConversation("agent-1")
	if err != nil {
		t.Fatalf("GetConversation failed: %v", err)
	}

	if len(messages) != 2 {
		t.Errorf("Expected 2 messages, got %d", len(messages))
	}
	if messages[0].Role != "user" {
		t.Errorf("Expected first message role 'user', got '%s'", messages[0].Role)
	}
}

func TestHubClient_KillAgent(t *testing.T) {
	called := false
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodDelete {
			t.Errorf("Expected DELETE method, got %s", r.Method)
		}
		if r.URL.Path != "/agents/agent-1" {
			t.Errorf("Expected /agents/agent-1 path, got %s", r.URL.Path)
		}
		called = true
		w.WriteHeader(http.StatusOK)
	}))
	defer server.Close()

	client := NewHubClient(server.URL)
	err := client.KillAgent("agent-1")
	if err != nil {
		t.Fatalf("KillAgent failed: %v", err)
	}

	if !called {
		t.Error("DELETE request was not made")
	}
}

func TestHubClient_SendMessage(t *testing.T) {
	called := false
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			t.Errorf("Expected POST method, got %s", r.Method)
		}
		if r.URL.Path != "/agents/agent-1/message" {
			t.Errorf("Expected /agents/agent-1/message path, got %s", r.URL.Path)
		}
		called = true
		w.WriteHeader(http.StatusOK)
	}))
	defer server.Close()

	client := NewHubClient(server.URL)
	err := client.SendMessage("agent-1", "Hello agent!")
	if err != nil {
		t.Fatalf("SendMessage failed: %v", err)
	}

	if !called {
		t.Error("POST request was not made")
	}
}

func TestHubClient_CreateTask(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			t.Errorf("Expected POST method, got %s", r.Method)
		}
		if r.URL.Path != "/tasks" {
			t.Errorf("Expected /tasks path, got %s", r.URL.Path)
		}
		json.NewEncoder(w).Encode(map[string]string{"id": "task-1"})
	}))
	defer server.Close()

	client := NewHubClient(server.URL)
	result, err := client.CreateTask(CreateTaskRequest{
		AgentID: "agent-1",
		Title:   "Test Task",
	})
	if err != nil {
		t.Fatalf("CreateTask failed: %v", err)
	}

	if result["id"] != "task-1" {
		t.Errorf("Expected task id 'task-1', got '%v'", result["id"])
	}
}

func TestHubClient_RespondSolicitation(t *testing.T) {
	called := false
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			t.Errorf("Expected POST method, got %s", r.Method)
		}
		if r.URL.Path != "/solicitations/sol-1/respond" {
			t.Errorf("Expected /solicitations/sol-1/respond path, got %s", r.URL.Path)
		}
		called = true
		w.WriteHeader(http.StatusOK)
	}))
	defer server.Close()

	client := NewHubClient(server.URL)
	err := client.RespondSolicitation("sol-1", "Here is my response")
	if err != nil {
		t.Fatalf("RespondSolicitation failed: %v", err)
	}

	if !called {
		t.Error("POST request was not made")
	}
}
