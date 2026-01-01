package config

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	"gopkg.in/yaml.v3"
)

// Config represents the Hive configuration
type Config struct {
	Workspace  WorkspaceConfig      `yaml:"workspace"`
	Redis      RedisConfig          `yaml:"redis"`
	Agents     AgentsConfig         `yaml:"agents"`
	Monitoring MonitoringConfig     `yaml:"monitoring"`
	MCPs       map[string]MCPConfig `yaml:"mcps,omitempty"`
	HostMCPs   HostMCPsConfig       `yaml:"host_mcps,omitempty"` // MCPs that run on host machine
	Tools      []string             `yaml:"tools,omitempty"`     // CLI tools to install in containers
	Volumes    []string             `yaml:"volumes,omitempty"`   // Additional volume mounts for all agents
	Hooks      HooksConfig          `yaml:"hooks,omitempty"`
	Playwright PlaywrightConfig     `yaml:"playwright,omitempty"`
	Network    NetworkConfig        `yaml:"network,omitempty"`
}

// NetworkConfig contains network/proxy settings
type NetworkConfig struct {
	CACert     string            `yaml:"ca_cert,omitempty"`     // Path to CA certificate for corporate proxy
	ExtraHosts []string          `yaml:"extra_hosts,omitempty"` // Extra /etc/hosts entries (e.g., "host.docker.internal:host-gateway")
	Env        map[string]string `yaml:"env,omitempty"`         // Network-related environment variables
}

// HooksConfig contains custom hook scripts
type HooksConfig struct {
	Init string `yaml:"init,omitempty"` // Script executed at container startup
}

// PlaywrightConfig contains Playwright browser settings (for containers)
type PlaywrightConfig struct {
	Mode            string `yaml:"mode,omitempty"`             // "headless" (default) or "connect"
	BrowserEndpoint string `yaml:"browser_endpoint,omitempty"` // WebSocket endpoint for connect mode
}

// HostMCPsConfig contains configuration for MCPs that run on the host machine
// These MCPs are started by hive and exposed via SSE to containers
type HostMCPsConfig struct {
	Playwright *PlaywrightHostMCP `yaml:"playwright,omitempty"`
	IOS        *IOSHostMCP        `yaml:"ios,omitempty"`
	Clipboard  *ClipboardHostMCP  `yaml:"clipboard,omitempty"`
}

// PlaywrightHostMCP configures the Playwright MCP server running on host
type PlaywrightHostMCP struct {
	Enabled  bool   `yaml:"enabled"`            // Enable Playwright host MCP (default: false)
	Port     int    `yaml:"port,omitempty"`     // SSE port (default: 8931)
	Headless bool   `yaml:"headless,omitempty"` // Run browser headless (default: true)
	Browser  string `yaml:"browser,omitempty"`  // Browser type: chromium, firefox, webkit (default: chromium)
}

// IOSHostMCP configures the iOS MCP server running on host
type IOSHostMCP struct {
	Enabled bool `yaml:"enabled"`        // Enable iOS host MCP (default: false)
	Port    int  `yaml:"port,omitempty"` // SSE port (default: 8932)
}

// ClipboardHostMCP configures the Clipboard MCP server running on host
type ClipboardHostMCP struct {
	Enabled bool `yaml:"enabled"`        // Enable Clipboard host MCP (default: false)
	Port    int  `yaml:"port,omitempty"` // SSE port (default: 8933)
}

// GetPlaywrightPort returns the port for Playwright MCP (default: 8931)
func (h *HostMCPsConfig) GetPlaywrightPort() int {
	if h.Playwright != nil && h.Playwright.Port > 0 {
		return h.Playwright.Port
	}
	return 8931
}

// GetIOSPort returns the port for iOS MCP (default: 8932)
func (h *HostMCPsConfig) GetIOSPort() int {
	if h.IOS != nil && h.IOS.Port > 0 {
		return h.IOS.Port
	}
	return 8932
}

// IsPlaywrightEnabled returns true if Playwright host MCP is enabled
func (h *HostMCPsConfig) IsPlaywrightEnabled() bool {
	return h.Playwright != nil && h.Playwright.Enabled
}

// IsIOSEnabled returns true if iOS host MCP is enabled
func (h *HostMCPsConfig) IsIOSEnabled() bool {
	return h.IOS != nil && h.IOS.Enabled
}

// GetPlaywrightBrowser returns the browser type (default: chromium)
func (h *HostMCPsConfig) GetPlaywrightBrowser() string {
	if h.Playwright != nil && h.Playwright.Browser != "" {
		return h.Playwright.Browser
	}
	return "chromium"
}

// IsPlaywrightHeadless returns true if browser should run headless (default: true)
func (h *HostMCPsConfig) IsPlaywrightHeadless() bool {
	if h.Playwright == nil {
		return true
	}
	return h.Playwright.Headless
}

// GetClipboardPort returns the port for Clipboard MCP (default: 8933)
func (h *HostMCPsConfig) GetClipboardPort() int {
	if h.Clipboard != nil && h.Clipboard.Port > 0 {
		return h.Clipboard.Port
	}
	return 8933
}

// IsClipboardEnabled returns true if Clipboard host MCP is enabled
func (h *HostMCPsConfig) IsClipboardEnabled() bool {
	return h.Clipboard != nil && h.Clipboard.Enabled
}

// MCPConfig represents a Model Context Protocol server configuration
type MCPConfig struct {
	Package string   `yaml:"package,omitempty"` // NPM package name (e.g., "@playwright/mcp")
	Command string   `yaml:"command,omitempty"` // Custom command (if not using package)
	Args    []string `yaml:"args,omitempty"`    // Additional arguments
	Env     []string `yaml:"env,omitempty"`     // Required environment variables (stored in .env.project)
}

// WorkspaceConfig contains workspace settings
type WorkspaceConfig struct {
	Name            string `yaml:"name"`
	GitURL          string `yaml:"git_url,omitempty"`
	ContainerPrefix string `yaml:"container_prefix,omitempty"` // Prefix for container names (default: sanitized project dir name)
	MountSource     bool   `yaml:"mount_source,omitempty"`     // Mount parent directory as workspace (instead of git worktrees)
}

// RedisConfig contains Redis settings
type RedisConfig struct {
	Port int `yaml:"port"`
}

// AgentsConfig contains agent settings
type AgentsConfig struct {
	Queen   AgentConfig   `yaml:"queen"`
	Workers WorkersConfig `yaml:"workers"`
}

// AgentConfig contains individual agent settings
type AgentConfig struct {
	Model      string            `yaml:"model,omitempty"`
	Dockerfile string            `yaml:"dockerfile,omitempty"`
	Env        map[string]string `yaml:"env,omitempty"`
	Ports      []string          `yaml:"ports,omitempty"` // Port mappings "container:host" (e.g., "3000:13000")
}

// WorkersConfig contains worker settings
type WorkersConfig struct {
	Count               int                  `yaml:"count"`
	Mode                string               `yaml:"mode,omitempty"` // "interactive" (default) or "daemon"
	Model               string               `yaml:"model,omitempty"`
	Dockerfile          string               `yaml:"dockerfile,omitempty"`
	PollIntervalSeconds int                  `yaml:"poll_interval_seconds,omitempty"`
	Env                 map[string]string    `yaml:"env,omitempty"`
	Ports               []string             `yaml:"ports,omitempty"`         // Port mappings for workers (auto-incremented per drone)
	PortsPerDrone       map[int][]string     `yaml:"ports_per_drone,omitempty"` // Per-drone port mappings (e.g., 1: ["4200:4200"])
}

// MonitoringConfig contains background clock/polling settings
type MonitoringConfig struct {
	Queen  QueenMonitoringConfig  `yaml:"queen"`
	Worker WorkerMonitoringConfig `yaml:"worker"`
}

// QueenMonitoringConfig contains Queen's monitoring settings
type QueenMonitoringConfig struct {
	Enabled         bool `yaml:"enabled"`
	IntervalSeconds int  `yaml:"interval_seconds"`
}

// WorkerMonitoringConfig contains Worker's monitoring settings
type WorkerMonitoringConfig struct {
	Enabled         bool `yaml:"enabled"`
	IntervalSeconds int  `yaml:"interval_seconds"`
}

// Default returns a Config with default values
func Default() *Config {
	return &Config{
		Workspace: WorkspaceConfig{
			Name: "my-project",
		},
		Redis: RedisConfig{
			Port: 6380,
		},
		Agents: AgentsConfig{
			Queen: AgentConfig{
				Model:      "sonnet",
				Dockerfile: "docker/Dockerfile.node",
			},
			Workers: WorkersConfig{
				Count:               2,
				Model:               "sonnet",
				Dockerfile:          "docker/Dockerfile.node",
				PollIntervalSeconds: 1,
			},
		},
		Monitoring: MonitoringConfig{
			Queen: QueenMonitoringConfig{
				Enabled:         true,
				IntervalSeconds: 30,
			},
			Worker: WorkerMonitoringConfig{
				Enabled:         true,
				IntervalSeconds: 1,
			},
		},
	}
}

// Load reads and parses the hive.yaml file
func Load(path string) (*Config, error) {
	// Clean and validate path to prevent directory traversal
	cleanPath := filepath.Clean(path)
	data, err := os.ReadFile(cleanPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read config file: %w", err)
	}

	cfg := Default()
	if err := yaml.Unmarshal(data, cfg); err != nil {
		return nil, fmt.Errorf("failed to parse config file: %w", err)
	}

	return cfg, nil
}

// LoadOrDefault tries to load hive.yaml, falls back to default
func LoadOrDefault() *Config {
	configPath := filepath.Join(".", "hive.yaml")
	cfg, err := Load(configPath)
	if err != nil {
		// Config file doesn't exist or is invalid, use defaults
		return Default()
	}
	return cfg
}

// Save writes the config to a file
func (c *Config) Save(path string) error {
	data, err := yaml.Marshal(c)
	if err != nil {
		return fmt.Errorf("failed to marshal config: %w", err)
	}

	if err := os.WriteFile(path, data, 0600); err != nil {
		return fmt.Errorf("failed to write config file: %w", err)
	}

	return nil
}

// GenerateEnvVars generates environment variables from config for docker-compose
func (c *Config) GenerateEnvVars() map[string]string {
	env := make(map[string]string)

	// Workspace
	env["WORKSPACE_NAME"] = c.Workspace.Name
	if c.Workspace.GitURL != "" {
		env["GIT_REPO_URL"] = c.Workspace.GitURL
	}

	// Models
	if c.Agents.Queen.Model != "" {
		env["QUEEN_MODEL"] = c.Agents.Queen.Model
	}
	if c.Agents.Workers.Model != "" {
		env["WORKER_MODEL"] = c.Agents.Workers.Model
	}

	// Worker mode (interactive or daemon)
	if c.Agents.Workers.Mode != "" {
		env["WORKER_MODE"] = c.Agents.Workers.Mode
	}

	// Dockerfile
	if c.Agents.Queen.Dockerfile != "" {
		env["HIVE_DOCKERFILE"] = c.Agents.Queen.Dockerfile
	} else if c.Agents.Workers.Dockerfile != "" {
		env["HIVE_DOCKERFILE"] = c.Agents.Workers.Dockerfile
	}

	// Poll interval
	if c.Agents.Workers.PollIntervalSeconds > 0 {
		env["POLL_INTERVAL"] = fmt.Sprintf("%d", c.Agents.Workers.PollIntervalSeconds)
	}

	// Redis port
	if c.Redis.Port > 0 {
		env["REDIS_EXTERNAL_PORT"] = fmt.Sprintf("%d", c.Redis.Port)
	}

	// Monitoring config
	if c.Monitoring.Queen.Enabled {
		env["QUEEN_MONITORING_ENABLED"] = "true"
	} else {
		env["QUEEN_MONITORING_ENABLED"] = "false"
	}
	if c.Monitoring.Queen.IntervalSeconds > 0 {
		env["QUEEN_MONITORING_INTERVAL"] = fmt.Sprintf("%d", c.Monitoring.Queen.IntervalSeconds)
	}
	if c.Monitoring.Worker.Enabled {
		env["WORKER_MONITORING_ENABLED"] = "true"
	} else {
		env["WORKER_MONITORING_ENABLED"] = "false"
	}
	if c.Monitoring.Worker.IntervalSeconds > 0 {
		env["WORKER_MONITORING_INTERVAL"] = fmt.Sprintf("%d", c.Monitoring.Worker.IntervalSeconds)
	}

	return env
}

// WriteEnvGenerated writes the generated env vars to .hive/.env.generated
func (c *Config) WriteEnvGenerated(hiveDir string) error {
	env := c.GenerateEnvVars()

	var content string
	content += "# Auto-generated from hive.yaml - DO NOT EDIT\n"
	content += "# This file is regenerated on each hive start/update\n\n"

	// Write in a predictable order
	keys := []string{
		"WORKSPACE_NAME", "GIT_REPO_URL",
		"QUEEN_MODEL", "WORKER_MODEL", "WORKER_MODE",
		"HIVE_DOCKERFILE", "POLL_INTERVAL",
		"REDIS_EXTERNAL_PORT",
		"QUEEN_MONITORING_ENABLED", "QUEEN_MONITORING_INTERVAL",
		"WORKER_MONITORING_ENABLED", "WORKER_MONITORING_INTERVAL",
	}
	for _, key := range keys {
		if val, ok := env[key]; ok {
			content += fmt.Sprintf("%s=%s\n", key, val)
		}
	}

	path := filepath.Join(hiveDir, ".env.generated")
	return os.WriteFile(path, []byte(content), 0600)
}

// Validate checks if the configuration is valid
func (c *Config) Validate() error {
	if c.Workspace.Name == "" {
		return fmt.Errorf("workspace.name is required")
	}

	if c.Redis.Port < 1024 || c.Redis.Port > 65535 {
		return fmt.Errorf("redis.port must be between 1024 and 65535")
	}

	if c.Agents.Workers.Count < 1 || c.Agents.Workers.Count > 10 {
		return fmt.Errorf("agents.workers.count must be between 1 and 10")
	}

	return nil
}

// SanitizeProjectName converts a directory name to a valid container prefix
// - Converts to lowercase
// - Replaces spaces and special characters with hyphens
// - Removes leading/trailing hyphens
// - Limits length to 20 characters
func SanitizeProjectName(name string) string {
	// Convert to lowercase
	name = strings.ToLower(name)

	// Replace any non-alphanumeric characters with hyphens
	reg := regexp.MustCompile(`[^a-z0-9]+`)
	name = reg.ReplaceAllString(name, "-")

	// Remove leading/trailing hyphens
	name = strings.Trim(name, "-")

	// Limit length
	if len(name) > 20 {
		name = name[:20]
		// Remove trailing hyphen if we cut in the middle
		name = strings.TrimRight(name, "-")
	}

	// Fallback if empty
	if name == "" {
		name = "hive"
	}

	return name
}

// GetContainerPrefix returns the effective container prefix for this config
// Priority: 1. Explicit container_prefix in config, 2. Sanitized workspace name, 3. "hive"
func (c *Config) GetContainerPrefix() string {
	if c.Workspace.ContainerPrefix != "" {
		return c.Workspace.ContainerPrefix
	}
	if c.Workspace.Name != "" {
		return SanitizeProjectName(c.Workspace.Name)
	}
	return "hive"
}

// GetContainerPrefixFromDir returns a container prefix based on the current directory
func GetContainerPrefixFromDir() string {
	cwd, err := os.Getwd()
	if err != nil {
		return "hive"
	}
	return SanitizeProjectName(filepath.Base(cwd))
}
