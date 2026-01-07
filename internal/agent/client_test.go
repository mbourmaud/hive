package agent

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func TestHTTPClient_SendMessage(t *testing.T) {
	var receivedBody MessageRequest

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			t.Errorf("expected POST, got %s", r.Method)
		}
		if r.URL.Path != "/message" {
			t.Errorf("expected /message, got %s", r.URL.Path)
		}

		json.NewDecoder(r.Body).Decode(&receivedBody)
		w.WriteHeader(http.StatusOK)
	}))
	defer server.Close()

	// Extract port from test server
	port := extractPort(t, server.URL)

	client := &HTTPClient{
		httpClient: http.DefaultClient,
		baseHost:   "127.0.0.1",
	}

	err := client.SendMessage(context.Background(), port, "Hello agent")
	if err != nil {
		t.Fatalf("SendMessage failed: %v", err)
	}

	if receivedBody.Content != "Hello agent" {
		t.Errorf("expected content 'Hello agent', got '%s'", receivedBody.Content)
	}
	if receivedBody.Type != "user" {
		t.Errorf("expected type 'user', got '%s'", receivedBody.Type)
	}
}

func TestHTTPClient_GetMessages(t *testing.T) {
	messages := []Message{
		{Role: "user", Content: "Hello"},
		{Role: "assistant", Content: "Hi there!"},
	}

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			t.Errorf("expected GET, got %s", r.Method)
		}
		if r.URL.Path != "/messages" {
			t.Errorf("expected /messages, got %s", r.URL.Path)
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]interface{}{"messages": messages})
	}))
	defer server.Close()

	port := extractPort(t, server.URL)

	client := &HTTPClient{
		httpClient: http.DefaultClient,
		baseHost:   "127.0.0.1",
	}

	result, err := client.GetMessages(context.Background(), port)
	if err != nil {
		t.Fatalf("GetMessages failed: %v", err)
	}

	if len(result) != 2 {
		t.Fatalf("expected 2 messages, got %d", len(result))
	}

	if result[0].Role != "user" || result[0].Content != "Hello" {
		t.Error("first message mismatch")
	}
	if result[1].Role != "assistant" || result[1].Content != "Hi there!" {
		t.Error("second message mismatch")
	}
}

func TestHTTPClient_GetStatus(t *testing.T) {
	tests := []struct {
		serverStatus   string
		expectedStatus AgentStatus
	}{
		{"stable", StatusReady},
		{"running", StatusBusy},
	}

	for _, tt := range tests {
		t.Run(tt.serverStatus, func(t *testing.T) {
			server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				w.Header().Set("Content-Type", "application/json")
				json.NewEncoder(w).Encode(StatusResponse{Status: tt.serverStatus})
			}))
			defer server.Close()

			port := extractPort(t, server.URL)

			client := &HTTPClient{
				httpClient: http.DefaultClient,
				baseHost:   "127.0.0.1",
			}

			status, err := client.GetStatus(context.Background(), port)
			if err != nil {
				t.Fatalf("GetStatus failed: %v", err)
			}

			if status != tt.expectedStatus {
				t.Errorf("expected %s, got %s", tt.expectedStatus, status)
			}
		})
	}
}

func TestHTTPClient_GetStatus_Error(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusInternalServerError)
	}))
	defer server.Close()

	port := extractPort(t, server.URL)

	client := &HTTPClient{
		httpClient: http.DefaultClient,
		baseHost:   "127.0.0.1",
	}

	status, err := client.GetStatus(context.Background(), port)
	if err == nil {
		t.Error("expected error for 500 response")
	}
	if status != StatusError {
		t.Errorf("expected StatusError, got %s", status)
	}
}

func TestHTTPClient_WaitReady(t *testing.T) {
	callCount := 0

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		callCount++
		w.Header().Set("Content-Type", "application/json")

		// Return "running" first, then "stable"
		if callCount < 3 {
			json.NewEncoder(w).Encode(StatusResponse{Status: "running"})
		} else {
			json.NewEncoder(w).Encode(StatusResponse{Status: "stable"})
		}
	}))
	defer server.Close()

	port := extractPort(t, server.URL)

	client := &HTTPClient{
		httpClient: http.DefaultClient,
		baseHost:   "127.0.0.1",
	}

	err := client.WaitReady(context.Background(), port, 5*time.Second)
	if err != nil {
		t.Fatalf("WaitReady failed: %v", err)
	}
}

func TestHTTPClient_WaitReady_Timeout(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusServiceUnavailable)
	}))
	defer server.Close()

	port := extractPort(t, server.URL)

	client := &HTTPClient{
		httpClient: http.DefaultClient,
		baseHost:   "127.0.0.1",
	}

	err := client.WaitReady(context.Background(), port, 1*time.Second)
	if err == nil {
		t.Error("expected timeout error")
	}
}

func TestHTTPClient_Health(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(StatusResponse{Status: "stable"})
	}))
	defer server.Close()

	port := extractPort(t, server.URL)

	client := &HTTPClient{
		httpClient: http.DefaultClient,
		baseHost:   "127.0.0.1",
	}

	if !client.Health(context.Background(), port) {
		t.Error("expected health check to pass")
	}
}

func TestMockClient(t *testing.T) {
	mock := &MockClient{
		CurrentStatus: StatusReady,
	}

	// Test SendMessage
	err := mock.SendMessage(context.Background(), 7440, "test")
	if err != nil {
		t.Fatalf("SendMessage failed: %v", err)
	}

	if len(mock.Messages) != 1 {
		t.Fatalf("expected 1 message, got %d", len(mock.Messages))
	}

	// Test GetMessages
	messages, err := mock.GetMessages(context.Background(), 7440)
	if err != nil {
		t.Fatalf("GetMessages failed: %v", err)
	}

	if len(messages) != 1 {
		t.Fatalf("expected 1 message, got %d", len(messages))
	}

	// Test GetStatus
	status, err := mock.GetStatus(context.Background(), 7440)
	if err != nil {
		t.Fatalf("GetStatus failed: %v", err)
	}

	if status != StatusReady {
		t.Errorf("expected StatusReady, got %s", status)
	}

	// Test WaitReady
	err = mock.WaitReady(context.Background(), 7440, time.Second)
	if err != nil {
		t.Fatalf("WaitReady failed: %v", err)
	}

	// Test Health
	if !mock.Health(context.Background(), 7440) {
		t.Error("expected Health to return true")
	}
}

func extractPort(t *testing.T, url string) int {
	t.Helper()
	parts := strings.Split(url, ":")
	if len(parts) < 3 {
		t.Fatalf("invalid URL: %s", url)
	}
	var port int
	_, err := strings.NewReader(parts[2]).Read(make([]byte, 10))
	if err != nil {
		t.Fatalf("failed to extract port: %v", err)
	}

	// Parse port from URL like http://127.0.0.1:12345
	portStr := parts[2]
	for i, c := range portStr {
		if c < '0' || c > '9' {
			portStr = portStr[:i]
			break
		}
	}

	n, _ := json.Number(portStr).Int64()
	port = int(n)

	return port
}
