package monitor

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strings"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

type Agent struct {
	ID           string `json:"id"`
	Name         string `json:"name"`
	WorktreePath string `json:"worktree_path"`
	Branch       string `json:"branch"`
	Port         int    `json:"port"`
	Status       string `json:"status"`
	Specialty    string `json:"specialty,omitempty"`
	CreatedAt    string `json:"created_at"`
	Error        string `json:"error,omitempty"`
}

type TaskStep struct {
	ID       int      `json:"id"`
	Action   string   `json:"action"`
	DoD      []string `json:"dod,omitempty"`
	Autonomy string   `json:"autonomy,omitempty"`
	Status   string   `json:"status,omitempty"`
}

type TaskPlan struct {
	ID          string     `json:"id"`
	Title       string     `json:"title"`
	Description string     `json:"description,omitempty"`
	Steps       []TaskStep `json:"steps,omitempty"`
}

type Task struct {
	ID          string    `json:"id"`
	AgentID     string    `json:"agent_id"`
	AgentName   string    `json:"agent_name,omitempty"`
	Plan        *TaskPlan `json:"plan,omitempty"`
	Status      string    `json:"status"`
	CurrentStep int       `json:"current_step"`
}

type Solicitation struct {
	ID        string `json:"id"`
	AgentID   string `json:"agent_id"`
	AgentName string `json:"agent_name"`
	Type      string `json:"type"`
	Urgency   string `json:"urgency"`
	Message   string `json:"message"`
	Status    string `json:"status"`
	CreatedAt string `json:"created_at"`
}

type Event struct {
	Type string          `json:"type"`
	Data json.RawMessage `json:"data"`
}

type HubClient struct {
	baseURL    string
	wsURL      string
	httpClient *http.Client
	wsConn     *websocket.Conn
	mu         sync.Mutex

	OnEvent func(Event)
}

func NewHubClient(hubURL string) *HubClient {
	u, _ := url.Parse(hubURL)
	wsScheme := "ws"
	if u.Scheme == "https" {
		wsScheme = "wss"
	}
	wsURL := fmt.Sprintf("%s://%s/ws", wsScheme, u.Host)

	return &HubClient{
		baseURL: hubURL,
		wsURL:   wsURL,
		httpClient: &http.Client{
			Timeout: 10 * time.Second,
		},
	}
}

func (c *HubClient) GetAgents() ([]Agent, error) {
	resp, err := c.httpClient.Get(c.baseURL + "/agents")
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var agents []Agent
	if err := json.NewDecoder(resp.Body).Decode(&agents); err != nil {
		return nil, err
	}
	return agents, nil
}

func (c *HubClient) GetTasks() ([]Task, error) {
	resp, err := c.httpClient.Get(c.baseURL + "/tasks")
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var tasks []Task
	if err := json.NewDecoder(resp.Body).Decode(&tasks); err != nil {
		return nil, err
	}
	return tasks, nil
}

func (c *HubClient) GetSolicitations() ([]Solicitation, error) {
	resp, err := c.httpClient.Get(c.baseURL + "/solicitations")
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var solicitations []Solicitation
	if err := json.NewDecoder(resp.Body).Decode(&solicitations); err != nil {
		return nil, err
	}
	return solicitations, nil
}

func (c *HubClient) GetHealth() (map[string]interface{}, error) {
	resp, err := c.httpClient.Get(c.baseURL + "/health")
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var health map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&health); err != nil {
		return nil, err
	}
	return health, nil
}

type Message struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

func (c *HubClient) GetConversation(agentID string) ([]Message, error) {
	resp, err := c.httpClient.Get(c.baseURL + "/agents/" + agentID + "/conversation")
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var messages []Message
	if err := json.NewDecoder(resp.Body).Decode(&messages); err != nil {
		return nil, err
	}
	return messages, nil
}

func (c *HubClient) KillAgent(agentID string) error {
	req, err := http.NewRequest("DELETE", c.baseURL+"/agents/"+agentID, nil)
	if err != nil {
		return err
	}
	resp, err := c.httpClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (c *HubClient) DestroyAgent(agentID string) error {
	req, err := http.NewRequest("DELETE", c.baseURL+"/agents/"+agentID+"/destroy", nil)
	if err != nil {
		return err
	}
	resp, err := c.httpClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (c *HubClient) SendMessage(agentID, content string) error {
	body := fmt.Sprintf(`{"content": %q}`, content)
	resp, err := c.httpClient.Post(
		c.baseURL+"/agents/"+agentID+"/message",
		"application/json",
		strings.NewReader(body),
	)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (c *HubClient) ConnectWebSocket() error {
	c.mu.Lock()
	defer c.mu.Unlock()

	conn, _, err := websocket.DefaultDialer.Dial(c.wsURL, nil)
	if err != nil {
		return err
	}
	c.wsConn = conn

	go c.readMessages()
	return nil
}

func (c *HubClient) readMessages() {
	for {
		c.mu.Lock()
		conn := c.wsConn
		c.mu.Unlock()

		if conn == nil {
			return
		}

		_, message, err := conn.ReadMessage()
		if err != nil {
			return
		}

		var event Event
		if err := json.Unmarshal(message, &event); err != nil {
			continue
		}

		if c.OnEvent != nil {
			c.OnEvent(event)
		}
	}
}

func (c *HubClient) Close() {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.wsConn != nil {
		c.wsConn.Close()
		c.wsConn = nil
	}
}

// Task Management

type CreateTaskRequest struct {
	AgentID     string              `json:"agent_id"`
	AgentName   string              `json:"agent_name,omitempty"`
	Title       string              `json:"title"`
	Description string              `json:"description,omitempty"`
	Steps       []CreateStepRequest `json:"steps"`
}

type CreateStepRequest struct {
	Action      string   `json:"action"`
	Description string   `json:"description,omitempty"`
	DoD         []string `json:"dod"`
	Autonomy    string   `json:"autonomy"`
}

func (c *HubClient) CreateTask(req CreateTaskRequest) (map[string]interface{}, error) {
	body, _ := json.Marshal(req)
	resp, err := c.httpClient.Post(
		c.baseURL+"/tasks",
		"application/json",
		strings.NewReader(string(body)),
	)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}
	return result, nil
}

func (c *HubClient) StartTask(taskID string) error {
	resp, err := c.httpClient.Post(c.baseURL+"/tasks/"+taskID+"/start", "application/json", nil)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (c *HubClient) CompleteTask(taskID string) error {
	resp, err := c.httpClient.Post(c.baseURL+"/tasks/"+taskID+"/complete", "application/json", nil)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (c *HubClient) CancelTask(taskID string) error {
	req, err := http.NewRequest("DELETE", c.baseURL+"/tasks/"+taskID, nil)
	if err != nil {
		return err
	}
	resp, err := c.httpClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

// Solicitation Management

func (c *HubClient) RespondSolicitation(solicitationID, response string) error {
	body := fmt.Sprintf(`{"response": %q}`, response)
	resp, err := c.httpClient.Post(
		c.baseURL+"/solicitations/"+solicitationID+"/respond",
		"application/json",
		strings.NewReader(body),
	)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (c *HubClient) DismissSolicitation(solicitationID string) error {
	resp, err := c.httpClient.Post(
		c.baseURL+"/solicitations/"+solicitationID+"/dismiss",
		"application/json",
		nil,
	)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	return nil
}

func (c *HubClient) GetAgentEventsURL(agentID string) string {
	return c.baseURL + "/agents/" + agentID + "/events"
}
