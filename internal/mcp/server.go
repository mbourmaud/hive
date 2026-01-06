package mcp

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strconv"
	"sync"

	"github.com/mbourmaud/hive/internal/agent"
	"github.com/mbourmaud/hive/internal/port"
	"github.com/mbourmaud/hive/internal/solicitation"
	"github.com/mbourmaud/hive/internal/task"
)

// HubInterface defines the methods the MCP server needs from the Hub.
type HubInterface interface {
	// Agent management
	SpawnAgent(ctx context.Context, opts agent.SpawnOptions) (*agent.Agent, error)
	StopAgent(ctx context.Context, id string) error
	DestroyAgent(ctx context.Context, id string) error
	GetAgent(id string) (*agent.Agent, error)
	ListAgents() []*agent.Agent

	// Messaging
	SendMessage(ctx context.Context, agentID, message string) error
	GetConversation(ctx context.Context, agentID string) ([]agent.Message, error)
	GetAgentStatus(agentID string) (string, error)

	// Task management
	CreateTask(ctx context.Context, req task.CreateTaskRequest) (*task.Task, error)
	GetTask(id string) (*task.Task, error)
	ListTasks(agentID string, status task.TaskStatus) []*task.Task
	StartTask(ctx context.Context, id string) error
	CompleteTask(ctx context.Context, id string, result string) error
	FailTask(ctx context.Context, id, errorMsg string) error
	CancelTask(ctx context.Context, id string) error

	// Solicitations
	GetPendingSolicitations() []*solicitation.Solicitation
	GetSolicitation(id string) (*solicitation.Solicitation, error)
	RespondToSolicitation(ctx context.Context, id, response string) error
	DismissSolicitation(ctx context.Context, id string) error

	// Ports
	ListPorts() ([]port.PortLease, []port.PortWaiter)
	ForceReleasePort(portNum int) error

	// Status
	GetStatus() StatusInfo
}

// StatusInfo represents hub status (unique to MCP).
type StatusInfo struct {
	AgentsTotal          int `json:"agents_total"`
	AgentsRunning        int `json:"agents_running"`
	TasksTotal           int `json:"tasks_total"`
	SolicitationsPending int `json:"solicitations_pending"`
	PortsLeased          int `json:"ports_leased"`
}

// Server is an MCP server that exposes Hub functionality to the Queen.
type Server struct {
	hub         HubInterface
	initialized bool
	mu          sync.RWMutex
}

// NewServer creates a new MCP server.
func NewServer(hub HubInterface) *Server {
	return &Server{
		hub: hub,
	}
}

// Run starts the MCP server using stdio.
func (s *Server) Run(ctx context.Context) error {
	reader := bufio.NewReader(os.Stdin)
	writer := os.Stdout

	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
		}

		// Read line (JSON-RPC message)
		line, err := reader.ReadBytes('\n')
		if err != nil {
			if err == io.EOF {
				return nil
			}
			return fmt.Errorf("failed to read stdin: %w", err)
		}

		// Parse and handle request
		response := s.handleRequest(ctx, line)
		if response == nil {
			continue // Notification, no response needed
		}

		// Write response
		responseBytes, err := json.Marshal(response)
		if err != nil {
			continue
		}
		writer.Write(responseBytes)
		writer.Write([]byte("\n"))
	}
}

// handleRequest processes a JSON-RPC request and returns a response.
func (s *Server) handleRequest(ctx context.Context, data []byte) *JSONRPCResponse {
	var req JSONRPCRequest
	if err := json.Unmarshal(data, &req); err != nil {
		return &JSONRPCResponse{
			JSONRPC: "2.0",
			ID:      nil,
			Error: &JSONRPCError{
				Code:    ErrCodeParseError,
				Message: "Parse error",
			},
		}
	}

	// Handle notifications (no ID)
	if req.ID == nil {
		s.handleNotification(ctx, req)
		return nil
	}

	// Handle request
	result, err := s.handleMethod(ctx, req.Method, req.Params)
	if err != nil {
		return &JSONRPCResponse{
			JSONRPC: "2.0",
			ID:      req.ID,
			Error:   err,
		}
	}

	resultBytes, _ := json.Marshal(result)
	return &JSONRPCResponse{
		JSONRPC: "2.0",
		ID:      req.ID,
		Result:  resultBytes,
	}
}

// handleNotification processes a JSON-RPC notification.
func (s *Server) handleNotification(_ context.Context, req JSONRPCRequest) {
	switch req.Method {
	case MethodInitialized:
		// Client acknowledged initialization
	case MethodNotificationsCancelled:
		// Handle cancellation
	}
}

// handleMethod dispatches the request to the appropriate handler.
func (s *Server) handleMethod(ctx context.Context, method string, params json.RawMessage) (interface{}, *JSONRPCError) {
	switch method {
	case MethodInitialize:
		return s.handleInitialize(params)
	case MethodToolsList:
		return s.handleToolsList()
	case MethodToolsCall:
		return s.handleToolsCall(ctx, params)
	case MethodResourcesList:
		return s.handleResourcesList()
	case MethodResourcesRead:
		return s.handleResourcesRead(params)
	case MethodPromptsList:
		return s.handlePromptsList()
	case MethodPromptsGet:
		return s.handlePromptsGet(params)
	default:
		return nil, &JSONRPCError{
			Code:    ErrCodeMethodNotFound,
			Message: fmt.Sprintf("Method not found: %s", method),
		}
	}
}

// handleInitialize handles the initialize method.
func (s *Server) handleInitialize(params json.RawMessage) (interface{}, *JSONRPCError) {
	var initParams InitializeParams
	if err := json.Unmarshal(params, &initParams); err != nil {
		return nil, &JSONRPCError{
			Code:    ErrCodeInvalidParams,
			Message: "Invalid params",
		}
	}

	s.mu.Lock()
	s.initialized = true
	s.mu.Unlock()

	return InitializeResult{
		ProtocolVersion: "2024-11-05",
		Capabilities: Capabilities{
			Tools:     &ToolsCapability{},
			Resources: &ResourcesCapability{},
			Prompts:   &PromptsCapability{},
		},
		ServerInfo: ServerInfo{
			Name:    "hive-mcp",
			Version: "1.0.0",
		},
	}, nil
}

// handleToolsList returns the list of available tools.
func (s *Server) handleToolsList() (interface{}, *JSONRPCError) {
	tools := []Tool{
		{
			Name:        "manage_agent",
			Description: "Spawn, stop, or destroy an agent",
			InputSchema: InputSchema{
				Type: "object",
				Properties: map[string]Property{
					"action": {
						Type:        "string",
						Description: "Action to perform",
						Enum:        []string{"spawn", "stop", "destroy", "list", "get"},
					},
					"agent_id": {
						Type:        "string",
						Description: "Agent ID (for stop, destroy, get)",
					},
					"name": {
						Type:        "string",
						Description: "Agent name (for spawn)",
					},
					"branch": {
						Type:        "string",
						Description: "Branch to work on (for spawn)",
					},
					"specialty": {
						Type:        "string",
						Description: "Agent specialty (for spawn)",
						Enum:        []string{"front", "back", "infra", "fullstack"},
					},
					"model": {
						Type:        "string",
						Description: "Claude model to use (for spawn)",
					},
				},
				Required: []string{"action"},
			},
		},
		{
			Name:        "send_message",
			Description: "Send a message to an agent",
			InputSchema: InputSchema{
				Type: "object",
				Properties: map[string]Property{
					"agent_id": {
						Type:        "string",
						Description: "Agent ID to send message to",
					},
					"message": {
						Type:        "string",
						Description: "Message content",
					},
				},
				Required: []string{"agent_id", "message"},
			},
		},
		{
			Name:        "get_conversation",
			Description: "Get the conversation history with an agent",
			InputSchema: InputSchema{
				Type: "object",
				Properties: map[string]Property{
					"agent_id": {
						Type:        "string",
						Description: "Agent ID",
					},
				},
				Required: []string{"agent_id"},
			},
		},
		{
			Name:        "manage_task",
			Description: "Create, start, complete, fail, or cancel a task",
			InputSchema: InputSchema{
				Type: "object",
				Properties: map[string]Property{
					"action": {
						Type:        "string",
						Description: "Action to perform",
						Enum:        []string{"create", "start", "complete", "fail", "cancel", "list", "get"},
					},
					"task_id": {
						Type:        "string",
						Description: "Task ID (for start, complete, fail, cancel, get)",
					},
					"agent_id": {
						Type:        "string",
						Description: "Agent ID (for create, list)",
					},
					"title": {
						Type:        "string",
						Description: "Task title (for create)",
					},
					"description": {
						Type:        "string",
						Description: "Task description (for create)",
					},
					"result": {
						Type:        "string",
						Description: "Result message (for complete)",
					},
					"error": {
						Type:        "string",
						Description: "Error message (for fail)",
					},
					"status": {
						Type:        "string",
						Description: "Filter by status (for list)",
					},
				},
				Required: []string{"action"},
			},
		},
		{
			Name:        "respond_solicitation",
			Description: "Respond to or dismiss a solicitation from an agent",
			InputSchema: InputSchema{
				Type: "object",
				Properties: map[string]Property{
					"action": {
						Type:        "string",
						Description: "Action to perform",
						Enum:        []string{"respond", "dismiss", "list", "get"},
					},
					"solicitation_id": {
						Type:        "string",
						Description: "Solicitation ID",
					},
					"response": {
						Type:        "string",
						Description: "Response message (for respond)",
					},
				},
				Required: []string{"action"},
			},
		},
		{
			Name:        "manage_port",
			Description: "Manage port allocations",
			InputSchema: InputSchema{
				Type: "object",
				Properties: map[string]Property{
					"action": {
						Type:        "string",
						Description: "Action to perform",
						Enum:        []string{"list", "force_release"},
					},
					"port": {
						Type:        "string",
						Description: "Port number (for force_release)",
					},
				},
				Required: []string{"action"},
			},
		},
		{
			Name:        "get_status",
			Description: "Get overall hive status",
			InputSchema: InputSchema{
				Type:       "object",
				Properties: map[string]Property{},
			},
		},
	}

	return ToolsListResult{Tools: tools}, nil
}

// handleToolsCall executes a tool.
func (s *Server) handleToolsCall(ctx context.Context, params json.RawMessage) (interface{}, *JSONRPCError) {
	var callParams ToolsCallParams
	if err := json.Unmarshal(params, &callParams); err != nil {
		return nil, &JSONRPCError{
			Code:    ErrCodeInvalidParams,
			Message: "Invalid params",
		}
	}

	result, err := s.executeTool(ctx, callParams.Name, callParams.Arguments)
	if err != nil {
		return ToolsCallResult{
			Content: []Content{{Type: "text", Text: fmt.Sprintf("Error: %s", err.Error())}},
			IsError: true,
		}, nil
	}

	resultJSON, _ := json.MarshalIndent(result, "", "  ")
	return ToolsCallResult{
		Content: []Content{{Type: "text", Text: string(resultJSON)}},
	}, nil
}

// executeTool executes a specific tool.
func (s *Server) executeTool(ctx context.Context, name string, args map[string]interface{}) (interface{}, error) {
	switch name {
	case "manage_agent":
		return s.toolManageAgent(ctx, args)
	case "send_message":
		return s.toolSendMessage(ctx, args)
	case "get_conversation":
		return s.toolGetConversation(ctx, args)
	case "manage_task":
		return s.toolManageTask(ctx, args)
	case "respond_solicitation":
		return s.toolRespondSolicitation(ctx, args)
	case "manage_port":
		return s.toolManagePort(args)
	case "get_status":
		return s.toolGetStatus()
	default:
		return nil, fmt.Errorf("unknown tool: %s", name)
	}
}

// Tool implementations

func (s *Server) toolManageAgent(ctx context.Context, args map[string]interface{}) (interface{}, error) {
	action, _ := args["action"].(string)

	switch action {
	case "spawn":
		name, _ := args["name"].(string)
		if name == "" {
			return nil, fmt.Errorf("name is required for spawn")
		}
		branch, _ := args["branch"].(string)
		specialty, _ := args["specialty"].(string)
		model, _ := args["model"].(string)

		return s.hub.SpawnAgent(ctx, agent.SpawnOptions{
			Name:      name,
			Branch:    branch,
			Specialty: specialty,
			Model:     model,
		})

	case "stop":
		agentID, _ := args["agent_id"].(string)
		if agentID == "" {
			return nil, fmt.Errorf("agent_id is required for stop")
		}
		if err := s.hub.StopAgent(ctx, agentID); err != nil {
			return nil, err
		}
		return map[string]string{"status": "stopped"}, nil

	case "destroy":
		agentID, _ := args["agent_id"].(string)
		if agentID == "" {
			return nil, fmt.Errorf("agent_id is required for destroy")
		}
		if err := s.hub.DestroyAgent(ctx, agentID); err != nil {
			return nil, err
		}
		return map[string]string{"status": "destroyed"}, nil

	case "list":
		return s.hub.ListAgents(), nil

	case "get":
		agentID, _ := args["agent_id"].(string)
		if agentID == "" {
			return nil, fmt.Errorf("agent_id is required for get")
		}
		return s.hub.GetAgent(agentID)

	default:
		return nil, fmt.Errorf("unknown action: %s", action)
	}
}

func (s *Server) toolSendMessage(ctx context.Context, args map[string]interface{}) (interface{}, error) {
	agentID, _ := args["agent_id"].(string)
	message, _ := args["message"].(string)

	if agentID == "" || message == "" {
		return nil, fmt.Errorf("agent_id and message are required")
	}

	if err := s.hub.SendMessage(ctx, agentID, message); err != nil {
		return nil, err
	}

	return map[string]string{"status": "sent"}, nil
}

func (s *Server) toolGetConversation(ctx context.Context, args map[string]interface{}) (interface{}, error) {
	agentID, _ := args["agent_id"].(string)
	if agentID == "" {
		return nil, fmt.Errorf("agent_id is required")
	}

	return s.hub.GetConversation(ctx, agentID)
}

func (s *Server) toolManageTask(ctx context.Context, args map[string]interface{}) (interface{}, error) {
	action, _ := args["action"].(string)

	switch action {
	case "create":
		agentID, _ := args["agent_id"].(string)
		title, _ := args["title"].(string)
		if agentID == "" || title == "" {
			return nil, fmt.Errorf("agent_id and title are required for create")
		}
		description, _ := args["description"].(string)

		return s.hub.CreateTask(ctx, task.CreateTaskRequest{
			AgentID:     agentID,
			Title:       title,
			Description: description,
		})

	case "start":
		taskID, _ := args["task_id"].(string)
		if taskID == "" {
			return nil, fmt.Errorf("task_id is required for start")
		}
		if err := s.hub.StartTask(ctx, taskID); err != nil {
			return nil, err
		}
		return map[string]string{"status": "started"}, nil

	case "complete":
		taskID, _ := args["task_id"].(string)
		if taskID == "" {
			return nil, fmt.Errorf("task_id is required for complete")
		}
		result, _ := args["result"].(string)
		if err := s.hub.CompleteTask(ctx, taskID, result); err != nil {
			return nil, err
		}
		return map[string]string{"status": "completed"}, nil

	case "fail":
		taskID, _ := args["task_id"].(string)
		if taskID == "" {
			return nil, fmt.Errorf("task_id is required for fail")
		}
		errorMsg, _ := args["error"].(string)
		if err := s.hub.FailTask(ctx, taskID, errorMsg); err != nil {
			return nil, err
		}
		return map[string]string{"status": "failed"}, nil

	case "cancel":
		taskID, _ := args["task_id"].(string)
		if taskID == "" {
			return nil, fmt.Errorf("task_id is required for cancel")
		}
		if err := s.hub.CancelTask(ctx, taskID); err != nil {
			return nil, err
		}
		return map[string]string{"status": "cancelled"}, nil

	case "list":
		agentID, _ := args["agent_id"].(string)
		status, _ := args["status"].(string)
		return s.hub.ListTasks(agentID, task.TaskStatus(status)), nil

	case "get":
		taskID, _ := args["task_id"].(string)
		if taskID == "" {
			return nil, fmt.Errorf("task_id is required for get")
		}
		return s.hub.GetTask(taskID)

	default:
		return nil, fmt.Errorf("unknown action: %s", action)
	}
}

func (s *Server) toolRespondSolicitation(ctx context.Context, args map[string]interface{}) (interface{}, error) {
	action, _ := args["action"].(string)

	switch action {
	case "respond":
		solID, _ := args["solicitation_id"].(string)
		if solID == "" {
			return nil, fmt.Errorf("solicitation_id is required for respond")
		}
		response, _ := args["response"].(string)
		if err := s.hub.RespondToSolicitation(ctx, solID, response); err != nil {
			return nil, err
		}
		return map[string]string{"status": "responded"}, nil

	case "dismiss":
		solID, _ := args["solicitation_id"].(string)
		if solID == "" {
			return nil, fmt.Errorf("solicitation_id is required for dismiss")
		}
		if err := s.hub.DismissSolicitation(ctx, solID); err != nil {
			return nil, err
		}
		return map[string]string{"status": "dismissed"}, nil

	case "list":
		return s.hub.GetPendingSolicitations(), nil

	case "get":
		solID, _ := args["solicitation_id"].(string)
		if solID == "" {
			return nil, fmt.Errorf("solicitation_id is required for get")
		}
		return s.hub.GetSolicitation(solID)

	default:
		return nil, fmt.Errorf("unknown action: %s", action)
	}
}

// PortsInfo is the response format for listing ports.
type PortsInfo struct {
	Leases  []port.PortLease  `json:"leases"`
	Waiters []port.PortWaiter `json:"waiters"`
}

func (s *Server) toolManagePort(args map[string]interface{}) (interface{}, error) {
	action, _ := args["action"].(string)

	switch action {
	case "list":
		leases, waiters := s.hub.ListPorts()
		return PortsInfo{Leases: leases, Waiters: waiters}, nil

	case "force_release":
		portStr, _ := args["port"].(string)
		if portStr == "" {
			return nil, fmt.Errorf("port is required for force_release")
		}
		port, err := strconv.Atoi(portStr)
		if err != nil {
			return nil, fmt.Errorf("invalid port number: %s", portStr)
		}
		if port <= 0 || port > 65535 {
			return nil, fmt.Errorf("port must be between 1 and 65535")
		}
		if err := s.hub.ForceReleasePort(port); err != nil {
			return nil, err
		}
		return map[string]string{"status": "released"}, nil

	default:
		return nil, fmt.Errorf("unknown action: %s", action)
	}
}

func (s *Server) toolGetStatus() (interface{}, error) {
	return s.hub.GetStatus(), nil
}

// Resources and Prompts handlers

func (s *Server) handleResourcesList() (interface{}, *JSONRPCError) {
	resources := []Resource{
		{
			URI:         "hive://status",
			Name:        "Hive Status",
			Description: "Current status of the hive",
			MimeType:    "application/json",
		},
		{
			URI:         "hive://agents",
			Name:        "Agents",
			Description: "List of all agents",
			MimeType:    "application/json",
		},
		{
			URI:         "hive://solicitations",
			Name:        "Solicitations",
			Description: "Pending solicitations from agents",
			MimeType:    "application/json",
		},
	}
	return ResourcesListResult{Resources: resources}, nil
}

func (s *Server) handleResourcesRead(params json.RawMessage) (interface{}, *JSONRPCError) {
	var readParams ResourcesReadParams
	if err := json.Unmarshal(params, &readParams); err != nil {
		return nil, &JSONRPCError{
			Code:    ErrCodeInvalidParams,
			Message: "Invalid params",
		}
	}

	var content interface{}

	switch readParams.URI {
	case "hive://status":
		content = s.hub.GetStatus()
	case "hive://agents":
		content = s.hub.ListAgents()
	case "hive://solicitations":
		content = s.hub.GetPendingSolicitations()
	default:
		return nil, &JSONRPCError{
			Code:    ErrCodeInvalidParams,
			Message: fmt.Sprintf("Unknown resource: %s", readParams.URI),
		}
	}

	contentJSON, _ := json.MarshalIndent(content, "", "  ")
	return ResourcesReadResult{
		Contents: []ResourceContent{{
			URI:      readParams.URI,
			MimeType: "application/json",
			Text:     string(contentJSON),
		}},
	}, nil
}

func (s *Server) handlePromptsList() (interface{}, *JSONRPCError) {
	prompts := []Prompt{
		{
			Name:        "task_plan",
			Description: "Generate a task plan for an agent",
			Arguments: []PromptArgument{
				{Name: "objective", Description: "The task objective", Required: true},
				{Name: "specialty", Description: "Agent specialty"},
			},
		},
		{
			Name:        "review_solicitation",
			Description: "Review and respond to an agent's solicitation",
			Arguments: []PromptArgument{
				{Name: "solicitation_id", Description: "The solicitation ID", Required: true},
			},
		},
	}
	return PromptsListResult{Prompts: prompts}, nil
}

func (s *Server) handlePromptsGet(params json.RawMessage) (interface{}, *JSONRPCError) {
	var getParams PromptsGetParams
	if err := json.Unmarshal(params, &getParams); err != nil {
		return nil, &JSONRPCError{
			Code:    ErrCodeInvalidParams,
			Message: "Invalid params",
		}
	}

	switch getParams.Name {
	case "task_plan":
		objective := getParams.Arguments["objective"]
		specialty := getParams.Arguments["specialty"]
		if specialty == "" {
			specialty = "fullstack"
		}
		return PromptsGetResult{
			Messages: []PromptMessage{
				{
					Role: "user",
					Content: Content{
						Type: "text",
						Text: fmt.Sprintf(`Create a detailed task plan for an agent with specialty "%s" to accomplish the following objective:

%s

The plan should include:
1. Clear steps with specific actions
2. Definition of Done (DoD) for each step
3. Appropriate autonomy level for each step (full, ask_if_unclear, validate_before_next, notify_when_done)

Consider the agent's specialty and break down complex tasks appropriately.`, specialty, objective),
					},
				},
			},
		}, nil

	case "review_solicitation":
		solID := getParams.Arguments["solicitation_id"]
		sol, err := s.hub.GetSolicitation(solID)
		if err != nil {
			return nil, &JSONRPCError{
				Code:    ErrCodeInternalError,
				Message: err.Error(),
			}
		}

		return PromptsGetResult{
			Messages: []PromptMessage{
				{
					Role: "user",
					Content: Content{
						Type: "text",
						Text: fmt.Sprintf(`Review and respond to the following solicitation from agent "%s":

Type: %s
Urgency: %s
Message: %s

Please provide a helpful response that:
1. Addresses the agent's concern directly
2. Provides clear guidance or decision
3. Considers the urgency level`, sol.AgentName, string(sol.Type), string(sol.Urgency), sol.Message),
					},
				},
			},
		}, nil

	default:
		return nil, &JSONRPCError{
			Code:    ErrCodeInvalidParams,
			Message: fmt.Sprintf("Unknown prompt: %s", getParams.Name),
		}
	}
}
