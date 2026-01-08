package agent

import (
	"context"
	_ "embed"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"syscall"
	"time"

	"github.com/google/uuid"
	"github.com/mbourmaud/hive/internal/worktree"
)

//go:embed hive-commands.sh
var hiveCommandsScript string

//go:embed sandbox-config.json
var sandboxConfigTemplate string

//go:embed agent-system-prompt.md
var agentSystemPromptTemplate string

//go:embed skills/ralph-loop.md
var ralphLoopSkill string

const (
	DefaultBasePort     = 7440
	DefaultReadyTimeout = 60 * time.Second
)

// Spawner manages the lifecycle of agent processes.
type Spawner interface {
	// Spawn creates and starts a new agent.
	Spawn(ctx context.Context, opts SpawnOptions) (*Agent, error)
	// Stop stops an agent process gracefully.
	Stop(ctx context.Context, agent *Agent) error
	// Destroy stops an agent and removes its worktree.
	Destroy(ctx context.Context, agent *Agent) error
}

// ProcessSpawner implements Spawner using OS processes.
type ProcessSpawner struct {
	worktreeMgr worktree.Manager
	client      Client
	basePort    int
	usedPorts   map[int]bool
	portMu      sync.Mutex
}

// NewProcessSpawner creates a new process-based spawner.
func NewProcessSpawner(worktreeMgr worktree.Manager, client Client) *ProcessSpawner {
	return &ProcessSpawner{
		worktreeMgr: worktreeMgr,
		client:      client,
		basePort:    DefaultBasePort,
		usedPorts:   make(map[int]bool),
	}
}

// SetBasePort sets the starting port for agent allocation.
func (s *ProcessSpawner) SetBasePort(port int) {
	s.portMu.Lock()
	defer s.portMu.Unlock()
	s.basePort = port
}

// Spawn creates a worktree and starts an AgentAPI + Claude process.
func (s *ProcessSpawner) Spawn(ctx context.Context, opts SpawnOptions) (*Agent, error) {
	id := uuid.New().String()[:8]

	wt, err := s.worktreeMgr.Create(ctx, worktree.CreateOptions{
		Name:       opts.Name,
		RepoPath:   opts.RepoPath,
		Branch:     opts.Branch,
		BaseBranch: opts.BaseBranch,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to create worktree: %w", err)
	}

	if err := s.setupHiveCommands(wt.Path); err != nil {
		return nil, fmt.Errorf("failed to setup hive commands: %w", err)
	}

	hubURL := opts.HubURL
	if hubURL == "" {
		hubURL = "http://localhost:7433"
	}

	if err := s.setupSystemPrompt(wt.Path, systemPromptData{
		AgentID:   id,
		AgentName: opts.Name,
		RepoPath:  opts.RepoPath,
		Branch:    wt.Branch,
		Specialty: opts.Specialty,
		HubURL:    hubURL,
	}); err != nil {
		return nil, fmt.Errorf("failed to setup system prompt: %w", err)
	}

	if err := s.setupSkills(wt.Path); err != nil {
		return nil, fmt.Errorf("failed to setup skills: %w", err)
	}

	port, err := s.findAvailablePort()
	if err != nil {
		return nil, fmt.Errorf("failed to find available port: %w", err)
	}

	agent := &Agent{
		ID:           id,
		Name:         opts.Name,
		WorktreePath: wt.Path,
		Branch:       wt.Branch,
		Port:         port,
		Status:       StatusStarting,
		Specialty:    opts.Specialty,
		CreatedAt:    time.Now(),
	}

	// Find absolute paths for binaries (needed for sandbox)
	agentapiBin, err := exec.LookPath("agentapi")
	if err != nil {
		// Try common locations
		home := os.Getenv("HOME")
		agentapiBin = filepath.Join(home, "go", "bin", "agentapi")
		if _, statErr := os.Stat(agentapiBin); statErr != nil {
			s.releasePort(port)
			return nil, fmt.Errorf("agentapi not found: %w", err)
		}
	}

	claudeBin, err := exec.LookPath("claude")
	if err != nil {
		s.releasePort(port)
		return nil, fmt.Errorf("claude not found: %w", err)
	}

	claudeCmd := fmt.Sprintf("%s --dangerously-skip-permissions", claudeBin)
	if opts.Model != "" {
		claudeCmd = fmt.Sprintf("%s --dangerously-skip-permissions --model %s", claudeBin, opts.Model)
	}
	agentapiCmd := fmt.Sprintf("%s server --port %d -- %s", agentapiBin, port, claudeCmd)

	var cmd *exec.Cmd
	if opts.Sandbox {
		sandboxConfigPath, err := s.setupSandboxConfig(wt.Path)
		if err != nil {
			s.releasePort(port)
			return nil, fmt.Errorf("failed to setup sandbox config: %w", err)
		}
		cmd = exec.Command("srt", "-s", sandboxConfigPath, "-c", agentapiCmd)
	} else {
		cmd = exec.Command("sh", "-c", agentapiCmd)
	}

	cmd.Dir = wt.Path
	cmd.Env = append(os.Environ(),
		fmt.Sprintf("HOME=%s", os.Getenv("HOME")),
		fmt.Sprintf("PATH=%s", os.Getenv("PATH")),
		fmt.Sprintf("HIVE_HUB_URL=%s", hubURL),
		fmt.Sprintf("HIVE_AGENT_ID=%s", id),
		fmt.Sprintf("HIVE_AGENT_NAME=%s", opts.Name),
		fmt.Sprintf("HIVE_WORKTREE_PATH=%s", wt.Path),
		fmt.Sprintf("HIVE_COMMANDS_PATH=%s", filepath.Join(wt.Path, ".hive", "hive-commands.sh")),
	)

	cmd.SysProcAttr = &syscall.SysProcAttr{
		Setpgid: true,
	}

	if err := cmd.Start(); err != nil {
		s.releasePort(port)
		return nil, fmt.Errorf("failed to start agent: %w", err)
	}

	agent.Process = cmd.Process
	agent.PID = cmd.Process.Pid

	readyCtx, cancel := context.WithTimeout(context.Background(), DefaultReadyTimeout)
	defer cancel()

	if err := s.client.WaitReady(readyCtx, port, DefaultReadyTimeout); err != nil {
		cmd.Process.Kill()
		s.releasePort(port)
		return nil, fmt.Errorf("agent failed to become ready: %w", err)
	}

	agent.Status = StatusReady
	return agent, nil
}

// Stop gracefully stops an agent process.
func (s *ProcessSpawner) Stop(ctx context.Context, agent *Agent) error {
	if agent.Process == nil {
		return nil
	}

	// Send SIGTERM for graceful shutdown
	if err := agent.Process.Signal(syscall.SIGTERM); err != nil {
		// Process might already be dead
		if err.Error() != "os: process already finished" {
			return fmt.Errorf("failed to send SIGTERM: %w", err)
		}
	}

	// Wait for process to exit with timeout
	done := make(chan error, 1)
	go func() {
		_, err := agent.Process.Wait()
		done <- err
	}()

	select {
	case <-done:
		// Process exited
	case <-time.After(5 * time.Second):
		// Force kill after timeout
		agent.Process.Kill()
	case <-ctx.Done():
		agent.Process.Kill()
	}

	s.releasePort(agent.Port)
	agent.Status = StatusStopped
	return nil
}

// Destroy stops the agent and removes its worktree.
func (s *ProcessSpawner) Destroy(ctx context.Context, agent *Agent) error {
	// Stop the process first
	if err := s.Stop(ctx, agent); err != nil {
		// Continue with worktree removal even if stop fails
	}

	// Remove worktree
	if err := s.worktreeMgr.Delete(ctx, agent.Name); err != nil {
		return fmt.Errorf("failed to delete worktree: %w", err)
	}

	return nil
}

// findAvailablePort finds the next available port starting from basePort.
func (s *ProcessSpawner) findAvailablePort() (int, error) {
	s.portMu.Lock()
	defer s.portMu.Unlock()

	for port := s.basePort; port < s.basePort+100; port++ {
		if s.usedPorts[port] {
			continue
		}

		// Check if port is actually available
		ln, err := net.Listen("tcp", fmt.Sprintf(":%d", port))
		if err != nil {
			continue
		}
		ln.Close()

		s.usedPorts[port] = true
		return port, nil
	}

	return 0, fmt.Errorf("no available ports in range %d-%d", s.basePort, s.basePort+100)
}

// releasePort marks a port as available again.
func (s *ProcessSpawner) releasePort(port int) {
	s.portMu.Lock()
	defer s.portMu.Unlock()
	delete(s.usedPorts, port)
}

func (s *ProcessSpawner) setupHiveCommands(worktreePath string) error {
	hiveDir := filepath.Join(worktreePath, ".hive")
	if err := os.MkdirAll(hiveDir, 0755); err != nil {
		return fmt.Errorf("failed to create .hive directory: %w", err)
	}

	commandsPath := filepath.Join(hiveDir, "hive-commands.sh")
	if err := os.WriteFile(commandsPath, []byte(hiveCommandsScript), 0755); err != nil {
		return fmt.Errorf("failed to write hive-commands.sh: %w", err)
	}

	return nil
}

func (s *ProcessSpawner) setupSandboxConfig(worktreePath string) (string, error) {
	home := os.Getenv("HOME")

	config := sandboxConfigTemplate
	config = strings.ReplaceAll(config, "{{HOME}}", home)
	config = strings.ReplaceAll(config, "{{WORKTREE}}", worktreePath)

	configPath := filepath.Join(worktreePath, ".hive", "sandbox-config.json")
	if err := os.WriteFile(configPath, []byte(config), 0644); err != nil {
		return "", fmt.Errorf("failed to write sandbox config: %w", err)
	}

	return configPath, nil
}

type systemPromptData struct {
	AgentID   string
	AgentName string
	RepoPath  string
	Branch    string
	Specialty string
	HubURL    string
}

func (s *ProcessSpawner) setupSystemPrompt(worktreePath string, data systemPromptData) error {
	prompt := agentSystemPromptTemplate
	prompt = strings.ReplaceAll(prompt, "{{.AgentID}}", data.AgentID)
	prompt = strings.ReplaceAll(prompt, "{{.AgentName}}", data.AgentName)
	prompt = strings.ReplaceAll(prompt, "{{.RepoPath}}", data.RepoPath)
	prompt = strings.ReplaceAll(prompt, "{{.Branch}}", data.Branch)
	prompt = strings.ReplaceAll(prompt, "{{.Specialty}}", data.Specialty)
	prompt = strings.ReplaceAll(prompt, "{{.HubURL}}", data.HubURL)

	claudeMdPath := filepath.Join(worktreePath, "CLAUDE.md")
	if err := os.WriteFile(claudeMdPath, []byte(prompt), 0644); err != nil {
		return fmt.Errorf("failed to write CLAUDE.md: %w", err)
	}

	return nil
}

func (s *ProcessSpawner) setupSkills(worktreePath string) error {
	skillsDir := filepath.Join(worktreePath, ".claude", "skills")
	if err := os.MkdirAll(skillsDir, 0755); err != nil {
		return fmt.Errorf("failed to create .claude/skills directory: %w", err)
	}

	ralphLoopPath := filepath.Join(skillsDir, "ralph-loop.md")
	if err := os.WriteFile(ralphLoopPath, []byte(ralphLoopSkill), 0644); err != nil {
		return fmt.Errorf("failed to write ralph-loop.md skill: %w", err)
	}

	return nil
}

// MockSpawner is a mock implementation for testing.
type MockSpawner struct {
	Agents     map[string]*Agent
	SpawnError error
	StopError  error
	mu         sync.Mutex
	nextPort   int
}

// NewMockSpawner creates a new mock spawner.
func NewMockSpawner() *MockSpawner {
	return &MockSpawner{
		Agents:   make(map[string]*Agent),
		nextPort: DefaultBasePort,
	}
}

// Spawn mock implementation.
func (m *MockSpawner) Spawn(_ context.Context, opts SpawnOptions) (*Agent, error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	if m.SpawnError != nil {
		return nil, m.SpawnError
	}

	agent := &Agent{
		ID:           uuid.New().String()[:8],
		Name:         opts.Name,
		WorktreePath: fmt.Sprintf("/tmp/hive-worktrees/%s", opts.Name),
		Branch:       opts.Branch,
		Port:         m.nextPort,
		Status:       StatusReady,
		Specialty:    opts.Specialty,
		CreatedAt:    time.Now(),
	}
	m.nextPort++
	m.Agents[agent.ID] = agent
	return agent, nil
}

// Stop mock implementation.
func (m *MockSpawner) Stop(_ context.Context, agent *Agent) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	if m.StopError != nil {
		return m.StopError
	}

	if a, ok := m.Agents[agent.ID]; ok {
		a.Status = StatusStopped
	}
	return nil
}

// Destroy mock implementation.
func (m *MockSpawner) Destroy(_ context.Context, agent *Agent) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	if m.StopError != nil {
		return m.StopError
	}

	delete(m.Agents, agent.ID)
	return nil
}
