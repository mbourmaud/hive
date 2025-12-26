package mcp

// MCPInfo contains information about a known MCP server
type MCPInfo struct {
	Name        string   // Display name
	Package     string   // NPM package name
	Description string   // Short description
	Env         []string // Required environment variables
	EnvOptional []string // Optional environment variables
}

// Registry contains all known MCP servers
var Registry = map[string]MCPInfo{
	// Browser Automation
	"playwright": {
		Name:        "Playwright",
		Package:     "@playwright/mcp",
		Description: "Browser automation for testing and scraping",
		Env:         []string{},
	},

	// Project Management
	"jira": {
		Name:        "Jira",
		Package:     "@anthropic/mcp-jira",
		Description: "Atlassian Jira issue tracking",
		Env:         []string{"JIRA_HOST", "JIRA_EMAIL", "JIRA_TOKEN"},
	},
	"linear": {
		Name:        "Linear",
		Package:     "@anthropic/mcp-linear",
		Description: "Linear issue tracking",
		Env:         []string{"LINEAR_API_KEY"},
	},
	"asana": {
		Name:        "Asana",
		Package:     "@anthropic/mcp-asana",
		Description: "Asana project management",
		Env:         []string{"ASANA_ACCESS_TOKEN"},
	},

	// Code & Version Control
	"github": {
		Name:        "GitHub",
		Package:     "@modelcontextprotocol/server-github",
		Description: "GitHub repositories and issues",
		Env:         []string{"GITHUB_TOKEN"},
	},
	"gitlab": {
		Name:        "GitLab",
		Package:     "@anthropic/mcp-gitlab",
		Description: "GitLab repositories and issues",
		Env:         []string{"GITLAB_TOKEN"},
		EnvOptional: []string{"GITLAB_HOST"},
	},

	// Documentation & Knowledge
	"notion": {
		Name:        "Notion",
		Package:     "@modelcontextprotocol/server-notion",
		Description: "Notion pages and databases",
		Env:         []string{"NOTION_TOKEN"},
	},
	"confluence": {
		Name:        "Confluence",
		Package:     "@anthropic/mcp-confluence",
		Description: "Atlassian Confluence wiki",
		Env:         []string{"CONFLUENCE_HOST", "CONFLUENCE_EMAIL", "CONFLUENCE_TOKEN"},
	},

	// Communication
	"slack": {
		Name:        "Slack",
		Package:     "@modelcontextprotocol/server-slack",
		Description: "Slack messaging",
		Env:         []string{"SLACK_BOT_TOKEN"},
	},

	// Cloud & Infrastructure
	"aws": {
		Name:        "AWS",
		Package:     "@anthropic/mcp-aws",
		Description: "Amazon Web Services",
		Env:         []string{"AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"},
		EnvOptional: []string{"AWS_REGION"},
	},

	// Databases
	"postgres": {
		Name:        "PostgreSQL",
		Package:     "@modelcontextprotocol/server-postgres",
		Description: "PostgreSQL database",
		Env:         []string{"POSTGRES_URL"},
	},

	// AI & Memory
	"memory": {
		Name:        "Memory",
		Package:     "@anthropic/mcp-memory",
		Description: "Persistent memory for Claude",
		Env:         []string{},
	},

	// File & Data
	"filesystem": {
		Name:        "Filesystem",
		Package:     "@modelcontextprotocol/server-filesystem",
		Description: "Local filesystem access",
		Env:         []string{},
	},
	"gdrive": {
		Name:        "Google Drive",
		Package:     "@anthropic/mcp-gdrive",
		Description: "Google Drive files",
		Env:         []string{"GDRIVE_CREDENTIALS"},
	},

	// Monitoring & Observability
	"sentry": {
		Name:        "Sentry",
		Package:     "@anthropic/mcp-sentry",
		Description: "Sentry error tracking",
		Env:         []string{"SENTRY_AUTH_TOKEN", "SENTRY_ORG"},
	},

	// Development Tools
	"sequential-thinking": {
		Name:        "Sequential Thinking",
		Package:     "@anthropic/mcp-sequential-thinking",
		Description: "Step-by-step reasoning tool",
		Env:         []string{},
	},
	"context7": {
		Name:        "Context7",
		Package:     "@anthropic/mcp-context7",
		Description: "Library documentation fetcher",
		Env:         []string{},
	},
}

// Get returns MCP info by name, or nil if not found
func Get(name string) *MCPInfo {
	if info, ok := Registry[name]; ok {
		return &info
	}
	return nil
}

// List returns all known MCP names
func List() []string {
	names := make([]string, 0, len(Registry))
	for name := range Registry {
		names = append(names, name)
	}
	return names
}

// IsKnown checks if an MCP is in the registry
func IsKnown(name string) bool {
	_, ok := Registry[name]
	return ok
}
