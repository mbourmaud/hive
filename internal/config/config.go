package config

import (
	"fmt"
	"os"
	"path/filepath"

	"gopkg.in/yaml.v3"
)

// Config represents the Hive configuration
type Config struct {
	Workspace  WorkspaceConfig  `yaml:"workspace"`
	Redis      RedisConfig      `yaml:"redis"`
	Agents     AgentsConfig     `yaml:"agents"`
	Monitoring MonitoringConfig `yaml:"monitoring"`
}

// WorkspaceConfig contains workspace settings
type WorkspaceConfig struct {
	Name   string `yaml:"name"`
	GitURL string `yaml:"git_url,omitempty"`
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
	Count      int               `yaml:"count"`
	Model      string            `yaml:"model,omitempty"`
	Dockerfile string            `yaml:"dockerfile,omitempty"`
	Env        map[string]string `yaml:"env,omitempty"`
}

// MonitoringConfig contains background clock/polling settings
type MonitoringConfig struct {
	Queen  QueenMonitoringConfig  `yaml:"queen"`
	Worker WorkerMonitoringConfig `yaml:"worker"`
}

// QueenMonitoringConfig contains Queen's monitoring settings
type QueenMonitoringConfig struct {
	Enabled        bool `yaml:"enabled"`
	IntervalMinutes int `yaml:"interval_minutes"`
}

// WorkerMonitoringConfig contains Worker's monitoring settings
type WorkerMonitoringConfig struct {
	Enabled        bool `yaml:"enabled"`
	IntervalMinutes int `yaml:"interval_minutes"`
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
				Count:      2,
				Model:      "sonnet",
				Dockerfile: "docker/Dockerfile.node",
			},
		},
		Monitoring: MonitoringConfig{
			Queen: QueenMonitoringConfig{
				Enabled:         true,
				IntervalMinutes: 5,
			},
			Worker: WorkerMonitoringConfig{
				Enabled:         true,
				IntervalMinutes: 2,
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
