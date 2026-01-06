package monitor

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
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

type Task struct {
	ID          string `json:"id"`
	AgentID     string `json:"agent_id"`
	AgentName   string `json:"agent_name,omitempty"`
	Status      string `json:"status"`
	CurrentStep int    `json:"current_step"`
	TotalSteps  int    `json:"total_steps,omitempty"`
	Title       string `json:"title,omitempty"`
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
