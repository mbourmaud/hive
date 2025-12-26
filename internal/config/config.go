package config

import (
	"fmt"
	"os"
	"path/filepath"

	"gopkg.in/yaml.v3"
)

// Config represents the Hive configuration
type Config struct {
	Workspace  WorkspaceConfig      `yaml:"workspace"`
	Git        GitConfig            `yaml:"git"`
	Redis      RedisConfig          `yaml:"redis"`
	Agents     AgentsConfig         `yaml:"agents"`
	Monitoring MonitoringConfig     `yaml:"monitoring"`
	MCPs       map[string]MCPConfig `yaml:"mcps,omitempty"`
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
	Name   string `yaml:"name"`
	GitURL string `yaml:"git_url,omitempty"`
}

// GitConfig contains git user settings
type GitConfig struct {
	UserEmail string `yaml:"user_email"`
	UserName  string `yaml:"user_name"`
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
}

// WorkersConfig contains worker settings
type WorkersConfig struct {
	Count               int               `yaml:"count"`
	Model               string            `yaml:"model,omitempty"`
	Dockerfile          string            `yaml:"dockerfile,omitempty"`
	PollIntervalSeconds int               `yaml:"poll_interval_seconds,omitempty"`
	Env                 map[string]string `yaml:"env,omitempty"`
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
		Git: GitConfig{
			UserEmail: "",
			UserName:  "",
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

	// Git
	if c.Git.UserEmail != "" {
		env["GIT_USER_EMAIL"] = c.Git.UserEmail
	}
	if c.Git.UserName != "" {
		env["GIT_USER_NAME"] = c.Git.UserName
	}

	// Models
	if c.Agents.Queen.Model != "" {
		env["QUEEN_MODEL"] = c.Agents.Queen.Model
	}
	if c.Agents.Workers.Model != "" {
		env["WORKER_MODEL"] = c.Agents.Workers.Model
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
		"GIT_USER_EMAIL", "GIT_USER_NAME",
		"QUEEN_MODEL", "WORKER_MODEL",
		"HIVE_DOCKERFILE", "POLL_INTERVAL",
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
