// Package mcp provides Model Context Protocol server for the Queen.
package mcp

import "encoding/json"

// JSONRPCRequest represents an incoming JSON-RPC 2.0 request.
type JSONRPCRequest struct {
	JSONRPC string          `json:"jsonrpc"`
	ID      interface{}     `json:"id"`
	Method  string          `json:"method"`
	Params  json.RawMessage `json:"params,omitempty"`
}

// JSONRPCResponse represents an outgoing JSON-RPC 2.0 response.
type JSONRPCResponse struct {
	JSONRPC string          `json:"jsonrpc"`
	ID      interface{}     `json:"id"`
	Result  json.RawMessage `json:"result,omitempty"`
	Error   *JSONRPCError   `json:"error,omitempty"`
}

// JSONRPCError represents a JSON-RPC 2.0 error.
type JSONRPCError struct {
	Code    int         `json:"code"`
	Message string      `json:"message"`
	Data    interface{} `json:"data,omitempty"`
}

// Standard JSON-RPC error codes.
const (
	ErrCodeParseError     = -32700
	ErrCodeInvalidRequest = -32600
	ErrCodeMethodNotFound = -32601
	ErrCodeInvalidParams  = -32602
	ErrCodeInternalError  = -32603
)

// MCP method names.
const (
	MethodInitialize             = "initialize"
	MethodInitialized            = "notifications/initialized"
	MethodToolsList              = "tools/list"
	MethodToolsCall              = "tools/call"
	MethodResourcesList          = "resources/list"
	MethodResourcesRead          = "resources/read"
	MethodPromptsList            = "prompts/list"
	MethodPromptsGet             = "prompts/get"
	MethodNotificationsCancelled = "notifications/cancelled"
)

// InitializeParams represents parameters for the initialize method.
type InitializeParams struct {
	ProtocolVersion string       `json:"protocolVersion"`
	Capabilities    Capabilities `json:"capabilities"`
	ClientInfo      ClientInfo   `json:"clientInfo"`
}

// Capabilities represents MCP capabilities.
type Capabilities struct {
	Tools     *ToolsCapability     `json:"tools,omitempty"`
	Resources *ResourcesCapability `json:"resources,omitempty"`
	Prompts   *PromptsCapability   `json:"prompts,omitempty"`
}

// ToolsCapability represents tools capability.
type ToolsCapability struct{}

// ResourcesCapability represents resources capability.
type ResourcesCapability struct{}

// PromptsCapability represents prompts capability.
type PromptsCapability struct{}

// ClientInfo represents client information.
type ClientInfo struct {
	Name    string `json:"name"`
	Version string `json:"version,omitempty"`
}

// InitializeResult represents the result of initialize.
type InitializeResult struct {
	ProtocolVersion string       `json:"protocolVersion"`
	Capabilities    Capabilities `json:"capabilities"`
	ServerInfo      ServerInfo   `json:"serverInfo"`
}

// ServerInfo represents server information.
type ServerInfo struct {
	Name    string `json:"name"`
	Version string `json:"version"`
}

// Tool represents an MCP tool definition.
type Tool struct {
	Name        string      `json:"name"`
	Description string      `json:"description"`
	InputSchema InputSchema `json:"inputSchema"`
}

// InputSchema represents a JSON schema for tool input.
type InputSchema struct {
	Type       string              `json:"type"`
	Properties map[string]Property `json:"properties,omitempty"`
	Required   []string            `json:"required,omitempty"`
}

// Property represents a JSON schema property.
type Property struct {
	Type        string      `json:"type"`
	Description string      `json:"description,omitempty"`
	Enum        []string    `json:"enum,omitempty"`
	Default     interface{} `json:"default,omitempty"`
}

// ToolsListResult represents the result of tools/list.
type ToolsListResult struct {
	Tools []Tool `json:"tools"`
}

// ToolsCallParams represents parameters for tools/call.
type ToolsCallParams struct {
	Name      string                 `json:"name"`
	Arguments map[string]interface{} `json:"arguments,omitempty"`
}

// ToolsCallResult represents the result of tools/call.
type ToolsCallResult struct {
	Content []Content `json:"content"`
	IsError bool      `json:"isError,omitempty"`
}

// Content represents content in a tool result.
type Content struct {
	Type string `json:"type"`
	Text string `json:"text,omitempty"`
}

// Resource represents an MCP resource.
type Resource struct {
	URI         string `json:"uri"`
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
	MimeType    string `json:"mimeType,omitempty"`
}

// ResourcesListResult represents the result of resources/list.
type ResourcesListResult struct {
	Resources []Resource `json:"resources"`
}

// ResourcesReadParams represents parameters for resources/read.
type ResourcesReadParams struct {
	URI string `json:"uri"`
}

// ResourcesReadResult represents the result of resources/read.
type ResourcesReadResult struct {
	Contents []ResourceContent `json:"contents"`
}

// ResourceContent represents resource content.
type ResourceContent struct {
	URI      string `json:"uri"`
	MimeType string `json:"mimeType,omitempty"`
	Text     string `json:"text,omitempty"`
}

// Prompt represents an MCP prompt.
type Prompt struct {
	Name        string           `json:"name"`
	Description string           `json:"description,omitempty"`
	Arguments   []PromptArgument `json:"arguments,omitempty"`
}

// PromptArgument represents a prompt argument.
type PromptArgument struct {
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
	Required    bool   `json:"required,omitempty"`
}

// PromptsListResult represents the result of prompts/list.
type PromptsListResult struct {
	Prompts []Prompt `json:"prompts"`
}

// PromptsGetParams represents parameters for prompts/get.
type PromptsGetParams struct {
	Name      string            `json:"name"`
	Arguments map[string]string `json:"arguments,omitempty"`
}

// PromptsGetResult represents the result of prompts/get.
type PromptsGetResult struct {
	Messages []PromptMessage `json:"messages"`
}

// PromptMessage represents a message in a prompt.
type PromptMessage struct {
	Role    string  `json:"role"`
	Content Content `json:"content"`
}
