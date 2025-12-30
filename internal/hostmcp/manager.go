// Package hostmcp manages MCP servers that run on the host machine
// These servers provide browser and iOS automation capabilities to Docker containers
package hostmcp

import (
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/mbourmaud/hive/internal/config"
)

// Manager handles starting, stopping, and monitoring host MCP processes
type Manager struct {
	hiveDir string
	pidsDir string
	logsDir string
	cfg     *config.Config
}

// MCPType represents the type of host MCP
type MCPType string

const (
	MCPPlaywright MCPType = "playwright"
	MCPIOS        MCPType = "ios"
	MCPClipboard  MCPType = "clipboard"
)

// MCPStatus represents the status of a host MCP
type MCPStatus struct {
	Type    MCPType
	Running bool
	PID     int
	Port    int
	Error   string
}

// NewManager creates a new host MCP manager
func NewManager(hiveDir string, cfg *config.Config) *Manager {
	pidsDir := filepath.Join(hiveDir, "pids")
	logsDir := filepath.Join(hiveDir, "logs")
	return &Manager{
		hiveDir: hiveDir,
		pidsDir: pidsDir,
		logsDir: logsDir,
		cfg:     cfg,
	}
}

// EnsureDirs creates the required directories
func (m *Manager) EnsureDirs() error {
	if err := os.MkdirAll(m.pidsDir, 0755); err != nil {
		return fmt.Errorf("failed to create pids directory: %w", err)
	}
	if err := os.MkdirAll(m.logsDir, 0755); err != nil {
		return fmt.Errorf("failed to create logs directory: %w", err)
	}
	return nil
}

// StartAll starts all enabled host MCPs
func (m *Manager) StartAll() error {
	if err := m.EnsureDirs(); err != nil {
		return err
	}

	var errors []string

	if m.cfg.HostMCPs.IsPlaywrightEnabled() {
		if err := m.StartPlaywright(); err != nil {
			errors = append(errors, fmt.Sprintf("playwright: %v", err))
		}
	}

	if m.cfg.HostMCPs.IsIOSEnabled() {
		if err := m.StartIOS(); err != nil {
			errors = append(errors, fmt.Sprintf("ios: %v", err))
		}
	}

	if m.cfg.HostMCPs.IsClipboardEnabled() {
		if err := m.StartClipboard(); err != nil {
			errors = append(errors, fmt.Sprintf("clipboard: %v", err))
		}
	}

	if len(errors) > 0 {
		return fmt.Errorf("failed to start host MCPs: %s", strings.Join(errors, "; "))
	}
	return nil
}

// StopAll stops all running host MCPs
func (m *Manager) StopAll() error {
	var errors []string

	if err := m.StopMCP(MCPPlaywright); err != nil {
		errors = append(errors, fmt.Sprintf("playwright: %v", err))
	}

	if err := m.StopMCP(MCPIOS); err != nil {
		errors = append(errors, fmt.Sprintf("ios: %v", err))
	}

	if err := m.StopMCP(MCPClipboard); err != nil {
		errors = append(errors, fmt.Sprintf("clipboard: %v", err))
	}

	if len(errors) > 0 {
		return fmt.Errorf("failed to stop host MCPs: %s", strings.Join(errors, "; "))
	}
	return nil
}

// Status returns the status of all host MCPs
func (m *Manager) Status() []MCPStatus {
	var statuses []MCPStatus

	// Playwright status
	pwStatus := MCPStatus{
		Type: MCPPlaywright,
		Port: m.cfg.HostMCPs.GetPlaywrightPort(),
	}
	if pid, err := m.readPID(MCPPlaywright); err == nil && m.isProcessRunning(pid) {
		pwStatus.Running = true
		pwStatus.PID = pid
	}
	statuses = append(statuses, pwStatus)

	// iOS status
	iosStatus := MCPStatus{
		Type: MCPIOS,
		Port: m.cfg.HostMCPs.GetIOSPort(),
	}
	if pid, err := m.readPID(MCPIOS); err == nil && m.isProcessRunning(pid) {
		iosStatus.Running = true
		iosStatus.PID = pid
	}
	statuses = append(statuses, iosStatus)

	// Clipboard status
	clipStatus := MCPStatus{
		Type: MCPClipboard,
		Port: m.cfg.HostMCPs.GetClipboardPort(),
	}
	if pid, err := m.readPID(MCPClipboard); err == nil && m.isProcessRunning(pid) {
		clipStatus.Running = true
		clipStatus.PID = pid
	}
	statuses = append(statuses, clipStatus)

	return statuses
}

// StopMCP stops a specific host MCP by type
func (m *Manager) StopMCP(mcpType MCPType) error {
	pid, err := m.readPID(mcpType)
	if err != nil {
		// No PID file - try to kill by port as fallback
		port := m.getPortForType(mcpType)
		if port > 0 {
			return m.KillProcessOnPort(port)
		}
		return nil
	}

	if !m.isProcessRunning(pid) {
		// Process not running, clean up PID file
		m.removePIDFile(mcpType)
		return nil
	}

	// Try graceful shutdown first (SIGTERM)
	process, err := os.FindProcess(pid)
	if err != nil {
		m.removePIDFile(mcpType)
		return nil
	}

	if err := process.Signal(syscall.SIGTERM); err != nil {
		// Process might have died, try SIGKILL
		_ = process.Signal(syscall.SIGKILL)
	}

	// Wait for process to exit (with timeout)
	done := make(chan struct{})
	go func() {
		for i := 0; i < 50; i++ { // 5 second timeout
			if !m.isProcessRunning(pid) {
				close(done)
				return
			}
			time.Sleep(100 * time.Millisecond)
		}
		// Force kill if still running
		_ = process.Signal(syscall.SIGKILL)
		close(done)
	}()
	<-done

	m.removePIDFile(mcpType)
	return nil
}

// IsPortAvailable checks if a port is available for use
func (m *Manager) IsPortAvailable(port int) bool {
	// Try to listen on all interfaces
	listener, err := net.Listen("tcp", fmt.Sprintf(":%d", port))
	if err != nil {
		return false
	}
	listener.Close()

	// Also try localhost specifically (some processes only bind to localhost)
	listener, err = net.Listen("tcp", fmt.Sprintf("127.0.0.1:%d", port))
	if err != nil {
		return false
	}
	listener.Close()

	return true
}

// KillProcessOnPort kills any process listening on the given port
// This is used to clean up orphan processes from previous sessions
func (m *Manager) KillProcessOnPort(port int) error {
	// Use lsof to find the process
	cmd := exec.Command("lsof", "-i", fmt.Sprintf(":%d", port), "-t")
	output, err := cmd.Output()
	if err != nil {
		// No process found, port is free
		return nil
	}

	// Parse PIDs and kill each one
	pids := strings.Split(strings.TrimSpace(string(output)), "\n")
	for _, pidStr := range pids {
		if pidStr == "" {
			continue
		}
		pid, err := strconv.Atoi(pidStr)
		if err != nil {
			continue
		}

		process, err := os.FindProcess(pid)
		if err != nil {
			continue
		}

		// Kill the process
		_ = process.Signal(syscall.SIGTERM)
		time.Sleep(100 * time.Millisecond)

		// Force kill if still running
		if m.isProcessRunning(pid) {
			_ = process.Signal(syscall.SIGKILL)
		}
	}

	// Wait a bit for ports to be released
	time.Sleep(200 * time.Millisecond)
	return nil
}

// EnsurePortFree ensures the port is free, killing any existing process if needed
func (m *Manager) EnsurePortFree(mcpType MCPType, port int) error {
	// First try to stop via our tracked PID
	m.StopMCP(mcpType)

	// If port is still in use, kill whatever is on it
	if !m.IsPortAvailable(port) {
		if err := m.KillProcessOnPort(port); err != nil {
			return err
		}

		// Verify port is now free
		if !m.IsPortAvailable(port) {
			return fmt.Errorf("port %d is still in use after cleanup", port)
		}
	}

	return nil
}

// getPortForType returns the configured port for an MCP type
func (m *Manager) getPortForType(mcpType MCPType) int {
	switch mcpType {
	case MCPPlaywright:
		return m.cfg.HostMCPs.GetPlaywrightPort()
	case MCPIOS:
		return m.cfg.HostMCPs.GetIOSPort()
	case MCPClipboard:
		return m.cfg.HostMCPs.GetClipboardPort()
	default:
		return 0
	}
}

// pidFile returns the path to the PID file for an MCP type
func (m *Manager) pidFile(mcpType MCPType) string {
	return filepath.Join(m.pidsDir, fmt.Sprintf("%s.pid", mcpType))
}

// logFile returns the path to the log file for an MCP type
func (m *Manager) logFile(mcpType MCPType) string {
	return filepath.Join(m.logsDir, fmt.Sprintf("%s.log", mcpType))
}

// writePID writes a PID to the PID file
func (m *Manager) writePID(mcpType MCPType, pid int) error {
	return os.WriteFile(m.pidFile(mcpType), []byte(strconv.Itoa(pid)), 0644)
}

// readPID reads the PID from the PID file
func (m *Manager) readPID(mcpType MCPType) (int, error) {
	data, err := os.ReadFile(m.pidFile(mcpType))
	if err != nil {
		return 0, err
	}
	return strconv.Atoi(strings.TrimSpace(string(data)))
}

// removePIDFile removes the PID file
func (m *Manager) removePIDFile(mcpType MCPType) {
	os.Remove(m.pidFile(mcpType))
}

// isProcessRunning checks if a process with the given PID is running
func (m *Manager) isProcessRunning(pid int) bool {
	process, err := os.FindProcess(pid)
	if err != nil {
		return false
	}
	// On Unix, FindProcess always succeeds, we need to check with Signal(0)
	err = process.Signal(syscall.Signal(0))
	return err == nil
}

// startProcess starts a process and returns its PID
func (m *Manager) startProcess(mcpType MCPType, name string, args ...string) (int, error) {
	// Check if already running
	if pid, err := m.readPID(mcpType); err == nil && m.isProcessRunning(pid) {
		return pid, fmt.Errorf("%s is already running (PID: %d)", mcpType, pid)
	}

	// Open log file
	logFile, err := os.OpenFile(m.logFile(mcpType), os.O_CREATE|os.O_WRONLY|os.O_TRUNC, 0644)
	if err != nil {
		return 0, fmt.Errorf("failed to create log file: %w", err)
	}

	cmd := exec.Command(name, args...)
	cmd.Stdout = logFile
	cmd.Stderr = logFile
	cmd.SysProcAttr = &syscall.SysProcAttr{
		Setpgid: true, // Run in its own process group
	}

	if err := cmd.Start(); err != nil {
		logFile.Close()
		return 0, fmt.Errorf("failed to start %s: %w", mcpType, err)
	}

	pid := cmd.Process.Pid
	if err := m.writePID(mcpType, pid); err != nil {
		// Try to kill the process if we can't write PID
		cmd.Process.Kill()
		logFile.Close()
		return 0, fmt.Errorf("failed to write PID file: %w", err)
	}

	// Don't wait for the process, let it run in background
	go func() {
		cmd.Wait()
		logFile.Close()
	}()

	return pid, nil
}

// GetLogs returns the last n lines of logs for an MCP
func (m *Manager) GetLogs(mcpType MCPType, lines int) (string, error) {
	logPath := m.logFile(mcpType)
	data, err := os.ReadFile(logPath)
	if err != nil {
		return "", fmt.Errorf("failed to read log file: %w", err)
	}

	allLines := strings.Split(string(data), "\n")
	if len(allLines) <= lines {
		return string(data), nil
	}

	return strings.Join(allLines[len(allLines)-lines:], "\n"), nil
}
