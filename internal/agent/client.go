package agent

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

// Client provides HTTP communication with AgentAPI.
type Client interface {
	// SendMessage sends a message to an agent.
	SendMessage(ctx context.Context, port int, message string) error
	// GetMessages retrieves all messages from an agent's conversation.
	GetMessages(ctx context.Context, port int) ([]Message, error)
	// GetStatus returns the current status of an agent.
	GetStatus(ctx context.Context, port int) (AgentStatus, error)
	// WaitReady waits for an agent to become ready.
	WaitReady(ctx context.Context, port int, timeout time.Duration) error
	// Health checks if an agent is responding.
	Health(ctx context.Context, port int) bool
}

// HTTPClient implements Client using HTTP requests to AgentAPI.
type HTTPClient struct {
	httpClient *http.Client
	baseHost   string
}

// NewHTTPClient creates a new AgentAPI HTTP client.
func NewHTTPClient() *HTTPClient {
	return &HTTPClient{
		httpClient: &http.Client{
			Timeout: 30 * time.Second,
		},
		baseHost: "localhost",
	}
}

// SendMessage sends a message to an agent via POST /message.
func (c *HTTPClient) SendMessage(ctx context.Context, port int, message string) error {
	url := fmt.Sprintf("http://%s:%d/message", c.baseHost, port)

	payload := MessageRequest{
		Content: message,
		Type:    "user",
	}

	body, err := json.Marshal(payload)
	if err != nil {
		return fmt.Errorf("failed to marshal message: %w", err)
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(body))
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("failed to send message: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		respBody, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("agent returned status %d: %s", resp.StatusCode, string(respBody))
	}

	return nil
}

func (c *HTTPClient) GetMessages(ctx context.Context, port int) ([]Message, error) {
	url := fmt.Sprintf("http://%s:%d/messages", c.baseHost, port)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to get messages: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		respBody, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("agent returned status %d: %s", resp.StatusCode, string(respBody))
	}

	var wrapper struct {
		Messages []Message `json:"messages"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&wrapper); err != nil {
		return nil, fmt.Errorf("failed to decode messages: %w", err)
	}

	return wrapper.Messages, nil
}

// GetStatus returns the current status via GET /status.
func (c *HTTPClient) GetStatus(ctx context.Context, port int) (AgentStatus, error) {
	url := fmt.Sprintf("http://%s:%d/status", c.baseHost, port)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return StatusError, fmt.Errorf("failed to create request: %w", err)
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return StatusError, fmt.Errorf("failed to get status: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return StatusError, fmt.Errorf("agent returned status %d", resp.StatusCode)
	}

	var statusResp StatusResponse
	if err := json.NewDecoder(resp.Body).Decode(&statusResp); err != nil {
		return StatusError, fmt.Errorf("failed to decode status: %w", err)
	}

	switch statusResp.Status {
	case "stable":
		return StatusReady, nil
	case "running":
		return StatusBusy, nil
	default:
		return StatusError, fmt.Errorf("unknown status: %s", statusResp.Status)
	}
}

// WaitReady waits for an agent to become ready with polling.
func (c *HTTPClient) WaitReady(ctx context.Context, port int, timeout time.Duration) error {
	ctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	ticker := time.NewTicker(500 * time.Millisecond)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return fmt.Errorf("timeout waiting for agent to become ready")
		case <-ticker.C:
			status, err := c.GetStatus(ctx, port)
			if err == nil && (status == StatusReady || status == StatusBusy) {
				return nil
			}
		}
	}
}

// Health checks if an agent is responding.
func (c *HTTPClient) Health(ctx context.Context, port int) bool {
	_, err := c.GetStatus(ctx, port)
	return err == nil
}

// MockClient is a mock implementation for testing.
type MockClient struct {
	Messages      []Message
	CurrentStatus AgentStatus
	SendError     error
	GetError      error
	StatusError   error
}

// SendMessage mock implementation.
func (m *MockClient) SendMessage(_ context.Context, _ int, message string) error {
	if m.SendError != nil {
		return m.SendError
	}
	m.Messages = append(m.Messages, Message{
		Role:      "user",
		Content:   message,
		Timestamp: time.Now(),
	})
	return nil
}

// GetMessages mock implementation.
func (m *MockClient) GetMessages(_ context.Context, _ int) ([]Message, error) {
	if m.GetError != nil {
		return nil, m.GetError
	}
	return m.Messages, nil
}

// GetStatus mock implementation.
func (m *MockClient) GetStatus(_ context.Context, _ int) (AgentStatus, error) {
	if m.StatusError != nil {
		return StatusError, m.StatusError
	}
	return m.CurrentStatus, nil
}

// WaitReady mock implementation.
func (m *MockClient) WaitReady(_ context.Context, _ int, _ time.Duration) error {
	if m.CurrentStatus == StatusReady || m.CurrentStatus == StatusBusy {
		return nil
	}
	return fmt.Errorf("agent not ready")
}

// Health mock implementation.
func (m *MockClient) Health(_ context.Context, _ int) bool {
	return m.StatusError == nil
}
