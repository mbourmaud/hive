package config

import (
	"os"
	"path/filepath"
	"testing"
)

func TestDefault(t *testing.T) {
	cfg := Default()

	// Test default workspace
	if cfg.Workspace.Name != "my-project" {
		t.Errorf("expected workspace name 'my-project', got '%s'", cfg.Workspace.Name)
	}

	// Test default Redis port
	if cfg.Redis.Port != 6380 {
		t.Errorf("expected redis port 6380, got %d", cfg.Redis.Port)
	}

	// Test default agent settings
	if cfg.Agents.Queen.Model != "sonnet" {
		t.Errorf("expected queen model 'sonnet', got '%s'", cfg.Agents.Queen.Model)
	}
	if cfg.Agents.Queen.Dockerfile != "docker/Dockerfile.node" {
		t.Errorf("expected queen dockerfile 'docker/Dockerfile.node', got '%s'", cfg.Agents.Queen.Dockerfile)
	}

	// Test default worker settings
	if cfg.Agents.Workers.Count != 2 {
		t.Errorf("expected workers count 2, got %d", cfg.Agents.Workers.Count)
	}
	if cfg.Agents.Workers.Model != "sonnet" {
		t.Errorf("expected workers model 'sonnet', got '%s'", cfg.Agents.Workers.Model)
	}
}

func TestLoad(t *testing.T) {
	// Create a temporary config file
	tmpDir := t.TempDir()
	configPath := filepath.Join(tmpDir, "hive.yaml")

	content := `workspace:
  name: test-project
  git_url: https://github.com/test/repo.git
redis:
  port: 6381
agents:
  queen:
    model: opus
    dockerfile: docker/Dockerfile.golang
  workers:
    count: 5
    model: haiku
    dockerfile: docker/Dockerfile.python
`
	if err := os.WriteFile(configPath, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}

	cfg, err := Load(configPath)
	if err != nil {
		t.Fatalf("failed to load config: %v", err)
	}

	// Verify loaded values
	if cfg.Workspace.Name != "test-project" {
		t.Errorf("expected workspace name 'test-project', got '%s'", cfg.Workspace.Name)
	}
	if cfg.Workspace.GitURL != "https://github.com/test/repo.git" {
		t.Errorf("expected git url 'https://github.com/test/repo.git', got '%s'", cfg.Workspace.GitURL)
	}
	if cfg.Redis.Port != 6381 {
		t.Errorf("expected redis port 6381, got %d", cfg.Redis.Port)
	}
	if cfg.Agents.Queen.Model != "opus" {
		t.Errorf("expected queen model 'opus', got '%s'", cfg.Agents.Queen.Model)
	}
	if cfg.Agents.Workers.Count != 5 {
		t.Errorf("expected workers count 5, got %d", cfg.Agents.Workers.Count)
	}
	if cfg.Agents.Workers.Model != "haiku" {
		t.Errorf("expected workers model 'haiku', got '%s'", cfg.Agents.Workers.Model)
	}
}

func TestLoadNonExistent(t *testing.T) {
	_, err := Load("/nonexistent/path/hive.yaml")
	if err == nil {
		t.Error("expected error when loading non-existent config file")
	}
}

func TestLoadInvalidYAML(t *testing.T) {
	tmpDir := t.TempDir()
	configPath := filepath.Join(tmpDir, "hive.yaml")

	// Write invalid YAML
	content := `workspace:
  name: [invalid yaml
  this is not valid
`
	if err := os.WriteFile(configPath, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}

	_, err := Load(configPath)
	if err == nil {
		t.Error("expected error when loading invalid YAML config")
	}
}

func TestLoadPartialConfig(t *testing.T) {
	// Test that partial config uses defaults for missing values
	tmpDir := t.TempDir()
	configPath := filepath.Join(tmpDir, "hive.yaml")

	content := `workspace:
  name: partial-project
`
	if err := os.WriteFile(configPath, []byte(content), 0644); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}

	cfg, err := Load(configPath)
	if err != nil {
		t.Fatalf("failed to load config: %v", err)
	}

	// Custom value should be set
	if cfg.Workspace.Name != "partial-project" {
		t.Errorf("expected workspace name 'partial-project', got '%s'", cfg.Workspace.Name)
	}

	// Default values should be used for missing fields
	if cfg.Redis.Port != 6380 {
		t.Errorf("expected default redis port 6380, got %d", cfg.Redis.Port)
	}
	if cfg.Agents.Workers.Count != 2 {
		t.Errorf("expected default workers count 2, got %d", cfg.Agents.Workers.Count)
	}
}

func TestSave(t *testing.T) {
	tmpDir := t.TempDir()
	configPath := filepath.Join(tmpDir, "hive.yaml")

	cfg := &Config{
		Workspace: WorkspaceConfig{
			Name:   "saved-project",
			GitURL: "https://github.com/saved/repo.git",
		},
		Redis: RedisConfig{
			Port: 6382,
		},
		Agents: AgentsConfig{
			Queen: AgentConfig{
				Model:      "opus",
				Dockerfile: "docker/Dockerfile.rust",
			},
			Workers: WorkersConfig{
				Count:      7,
				Model:      "sonnet",
				Dockerfile: "docker/Dockerfile.golang",
			},
		},
	}

	if err := cfg.Save(configPath); err != nil {
		t.Fatalf("failed to save config: %v", err)
	}

	// Load it back and verify
	loaded, err := Load(configPath)
	if err != nil {
		t.Fatalf("failed to reload saved config: %v", err)
	}

	if loaded.Workspace.Name != cfg.Workspace.Name {
		t.Errorf("saved workspace name mismatch: expected '%s', got '%s'", cfg.Workspace.Name, loaded.Workspace.Name)
	}
	if loaded.Redis.Port != cfg.Redis.Port {
		t.Errorf("saved redis port mismatch: expected %d, got %d", cfg.Redis.Port, loaded.Redis.Port)
	}
	if loaded.Agents.Workers.Count != cfg.Agents.Workers.Count {
		t.Errorf("saved workers count mismatch: expected %d, got %d", cfg.Agents.Workers.Count, loaded.Agents.Workers.Count)
	}
}

func TestSaveInvalidPath(t *testing.T) {
	cfg := Default()
	err := cfg.Save("/nonexistent/directory/hive.yaml")
	if err == nil {
		t.Error("expected error when saving to invalid path")
	}
}

func TestValidate(t *testing.T) {
	tests := []struct {
		name    string
		config  *Config
		wantErr bool
		errMsg  string
	}{
		{
			name:    "valid default config",
			config:  Default(),
			wantErr: false,
		},
		{
			name: "empty workspace name",
			config: &Config{
				Workspace: WorkspaceConfig{Name: ""},
				Redis:     RedisConfig{Port: 6380},
				Agents:    AgentsConfig{Workers: WorkersConfig{Count: 2}},
			},
			wantErr: true,
			errMsg:  "workspace.name is required",
		},
		{
			name: "redis port too low",
			config: &Config{
				Workspace: WorkspaceConfig{Name: "test"},
				Redis:     RedisConfig{Port: 80},
				Agents:    AgentsConfig{Workers: WorkersConfig{Count: 2}},
			},
			wantErr: true,
			errMsg:  "redis.port must be between 1024 and 65535",
		},
		{
			name: "redis port too high",
			config: &Config{
				Workspace: WorkspaceConfig{Name: "test"},
				Redis:     RedisConfig{Port: 70000},
				Agents:    AgentsConfig{Workers: WorkersConfig{Count: 2}},
			},
			wantErr: true,
			errMsg:  "redis.port must be between 1024 and 65535",
		},
		{
			name: "workers count too low",
			config: &Config{
				Workspace: WorkspaceConfig{Name: "test"},
				Redis:     RedisConfig{Port: 6380},
				Agents:    AgentsConfig{Workers: WorkersConfig{Count: 0}},
			},
			wantErr: true,
			errMsg:  "agents.workers.count must be between 1 and 10",
		},
		{
			name: "workers count too high",
			config: &Config{
				Workspace: WorkspaceConfig{Name: "test"},
				Redis:     RedisConfig{Port: 6380},
				Agents:    AgentsConfig{Workers: WorkersConfig{Count: 15}},
			},
			wantErr: true,
			errMsg:  "agents.workers.count must be between 1 and 10",
		},
		{
			name: "valid edge case - min workers",
			config: &Config{
				Workspace: WorkspaceConfig{Name: "test"},
				Redis:     RedisConfig{Port: 1024},
				Agents:    AgentsConfig{Workers: WorkersConfig{Count: 1}},
			},
			wantErr: false,
		},
		{
			name: "valid edge case - max workers",
			config: &Config{
				Workspace: WorkspaceConfig{Name: "test"},
				Redis:     RedisConfig{Port: 65535},
				Agents:    AgentsConfig{Workers: WorkersConfig{Count: 10}},
			},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := tt.config.Validate()
			if tt.wantErr {
				if err == nil {
					t.Errorf("expected error '%s', got nil", tt.errMsg)
				} else if err.Error() != tt.errMsg {
					t.Errorf("expected error '%s', got '%s'", tt.errMsg, err.Error())
				}
			} else {
				if err != nil {
					t.Errorf("unexpected error: %v", err)
				}
			}
		})
	}
}

func TestLoadOrDefault(t *testing.T) {
	// Save current directory
	originalDir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get current directory: %v", err)
	}
	defer func() { _ = os.Chdir(originalDir) }()

	// Test with no config file (should return default)
	tmpDir := t.TempDir()
	if err := os.Chdir(tmpDir); err != nil {
		t.Fatalf("failed to change to temp directory: %v", err)
	}

	cfg := LoadOrDefault()
	if cfg.Workspace.Name != "my-project" {
		t.Errorf("expected default workspace name 'my-project', got '%s'", cfg.Workspace.Name)
	}

	// Test with config file
	content := `workspace:
  name: loaded-project
`
	if err := os.WriteFile("hive.yaml", []byte(content), 0644); err != nil {
		t.Fatalf("failed to write test config: %v", err)
	}

	cfg = LoadOrDefault()
	if cfg.Workspace.Name != "loaded-project" {
		t.Errorf("expected workspace name 'loaded-project', got '%s'", cfg.Workspace.Name)
	}
}

func TestConfigWithEnv(t *testing.T) {
	// Test that env maps are properly handled
	cfg := &Config{
		Workspace: WorkspaceConfig{Name: "env-test"},
		Redis:     RedisConfig{Port: 6380},
		Agents: AgentsConfig{
			Queen: AgentConfig{
				Model: "opus",
				Env: map[string]string{
					"CUSTOM_VAR": "value1",
					"DEBUG":      "true",
				},
			},
			Workers: WorkersConfig{
				Count: 2,
				Model: "sonnet",
				Env: map[string]string{
					"WORKER_VAR": "value2",
				},
			},
		},
	}

	tmpDir := t.TempDir()
	configPath := filepath.Join(tmpDir, "hive.yaml")

	if err := cfg.Save(configPath); err != nil {
		t.Fatalf("failed to save config with env: %v", err)
	}

	loaded, err := Load(configPath)
	if err != nil {
		t.Fatalf("failed to load config with env: %v", err)
	}

	if loaded.Agents.Queen.Env["CUSTOM_VAR"] != "value1" {
		t.Errorf("expected queen env CUSTOM_VAR='value1', got '%s'", loaded.Agents.Queen.Env["CUSTOM_VAR"])
	}
	if loaded.Agents.Workers.Env["WORKER_VAR"] != "value2" {
		t.Errorf("expected workers env WORKER_VAR='value2', got '%s'", loaded.Agents.Workers.Env["WORKER_VAR"])
	}
}

func TestGenerateEnvVars(t *testing.T) {
	cfg := &Config{
		Workspace: WorkspaceConfig{
			Name:   "test-workspace",
			GitURL: "https://github.com/test/repo.git",
		},
		Redis: RedisConfig{
			Port: 6380,
		},
		Agents: AgentsConfig{
			Queen: AgentConfig{
				Model:      "opus",
				Dockerfile: "docker/Dockerfile.go",
			},
			Workers: WorkersConfig{
				Count:               3,
				Model:               "sonnet",
				Mode:                "daemon",
				PollIntervalSeconds: 5,
			},
		},
		Monitoring: MonitoringConfig{
			Queen: QueenMonitoringConfig{
				Enabled:         true,
				IntervalSeconds: 30,
			},
			Worker: WorkerMonitoringConfig{
				Enabled:         false,
				IntervalSeconds: 10,
			},
		},
	}

	env := cfg.GenerateEnvVars()

	// Test workspace vars
	if env["WORKSPACE_NAME"] != "test-workspace" {
		t.Errorf("expected WORKSPACE_NAME='test-workspace', got '%s'", env["WORKSPACE_NAME"])
	}
	if env["GIT_REPO_URL"] != "https://github.com/test/repo.git" {
		t.Errorf("expected GIT_REPO_URL, got '%s'", env["GIT_REPO_URL"])
	}

	// Test model vars
	if env["QUEEN_MODEL"] != "opus" {
		t.Errorf("expected QUEEN_MODEL='opus', got '%s'", env["QUEEN_MODEL"])
	}
	if env["WORKER_MODEL"] != "sonnet" {
		t.Errorf("expected WORKER_MODEL='sonnet', got '%s'", env["WORKER_MODEL"])
	}

	// Test worker mode
	if env["WORKER_MODE"] != "daemon" {
		t.Errorf("expected WORKER_MODE='daemon', got '%s'", env["WORKER_MODE"])
	}

	// Test poll interval
	if env["POLL_INTERVAL"] != "5" {
		t.Errorf("expected POLL_INTERVAL='5', got '%s'", env["POLL_INTERVAL"])
	}

	// Test redis port
	if env["REDIS_EXTERNAL_PORT"] != "6380" {
		t.Errorf("expected REDIS_EXTERNAL_PORT='6380', got '%s'", env["REDIS_EXTERNAL_PORT"])
	}

	// Test monitoring vars
	if env["QUEEN_MONITORING_ENABLED"] != "true" {
		t.Errorf("expected QUEEN_MONITORING_ENABLED='true', got '%s'", env["QUEEN_MONITORING_ENABLED"])
	}
	if env["WORKER_MONITORING_ENABLED"] != "false" {
		t.Errorf("expected WORKER_MONITORING_ENABLED='false', got '%s'", env["WORKER_MONITORING_ENABLED"])
	}
}

func TestGenerateEnvVars_MinimalConfig(t *testing.T) {
	cfg := Default()
	env := cfg.GenerateEnvVars()

	// Ensure essential defaults are present
	if env["WORKSPACE_NAME"] != "my-project" {
		t.Errorf("expected default WORKSPACE_NAME='my-project', got '%s'", env["WORKSPACE_NAME"])
	}

	// Check that optional values don't generate env vars when not set
	if _, exists := env["GIT_REPO_URL"]; exists && cfg.Workspace.GitURL == "" {
		t.Error("should not generate GIT_REPO_URL when empty")
	}
}

func TestWriteEnvGenerated(t *testing.T) {
	tmpDir := t.TempDir()

	cfg := &Config{
		Workspace: WorkspaceConfig{
			Name:   "write-test",
			GitURL: "https://github.com/test/repo.git",
		},
		Redis: RedisConfig{
			Port: 6381,
		},
		Agents: AgentsConfig{
			Queen: AgentConfig{
				Model:      "opus",
				Dockerfile: "docker/Dockerfile.node",
			},
			Workers: WorkersConfig{
				Count: 2,
				Model: "sonnet",
				Mode:  "interactive",
			},
		},
	}

	if err := cfg.WriteEnvGenerated(tmpDir); err != nil {
		t.Fatalf("WriteEnvGenerated failed: %v", err)
	}

	// Read the generated file
	content, err := os.ReadFile(filepath.Join(tmpDir, ".env.generated"))
	if err != nil {
		t.Fatalf("failed to read generated env file: %v", err)
	}

	// Check that content includes expected values
	contentStr := string(content)
	expectedVars := []string{
		"WORKSPACE_NAME=write-test",
		"GIT_REPO_URL=https://github.com/test/repo.git",
		"QUEEN_MODEL=opus",
		"WORKER_MODEL=sonnet",
		"WORKER_MODE=interactive",
		"REDIS_EXTERNAL_PORT=6381",
	}

	for _, expected := range expectedVars {
		if !contains(contentStr, expected) {
			t.Errorf("expected env file to contain '%s'", expected)
		}
	}

	// Check header comments
	if !contains(contentStr, "Auto-generated from hive.yaml") {
		t.Error("expected header comment in generated file")
	}
}

func TestWriteEnvGenerated_InvalidPath(t *testing.T) {
	cfg := Default()
	err := cfg.WriteEnvGenerated("/nonexistent/path/that/does/not/exist")
	if err == nil {
		t.Error("expected error when writing to invalid path")
	}
}

// contains checks if a string contains a substring
func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(substr) == 0 ||
		(len(s) > 0 && len(substr) > 0 && findSubstring(s, substr)))
}

func findSubstring(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}

func TestSanitizeProjectName(t *testing.T) {
	tests := []struct {
		input    string
		expected string
	}{
		{"my-project", "my-project"},
		{"MyProject", "myproject"},
		{"my project", "my-project"},
		{"My Cool Project", "my-cool-project"},
		{"project_name", "project-name"},
		{"project.name", "project-name"},
		{"Project123", "project123"},
		{"--project--", "project"},
		{"   spaces   ", "spaces"},
		{"UPPERCASE", "uppercase"},
		{"a-very-long-project-name-that-exceeds-twenty-chars", "a-very-long-project"},
		{"", "hive"},
		{"   ", "hive"},
		{"---", "hive"},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			result := SanitizeProjectName(tt.input)
			if result != tt.expected {
				t.Errorf("SanitizeProjectName(%q) = %q, expected %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestGetContainerPrefix(t *testing.T) {
	tests := []struct {
		name            string
		containerPrefix string
		workspaceName   string
		expected        string
	}{
		{"explicit prefix", "custom-prefix", "workspace", "custom-prefix"},
		{"from workspace name", "", "My Project", "my-project"},
		{"fallback to hive", "", "", "hive"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cfg := &Config{
				Workspace: WorkspaceConfig{
					Name:            tt.workspaceName,
					ContainerPrefix: tt.containerPrefix,
				},
			}
			result := cfg.GetContainerPrefix()
			if result != tt.expected {
				t.Errorf("GetContainerPrefix() = %q, expected %q", result, tt.expected)
			}
		})
	}
}
