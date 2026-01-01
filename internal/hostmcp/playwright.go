package hostmcp

import (
	"fmt"
	"os/exec"
	"time"
)

// StartPlaywright starts the Playwright MCP server on the host
func (m *Manager) StartPlaywright() error {
	port := m.cfg.HostMCPs.GetPlaywrightPort()

	// Ensure port is free (kill any existing process)
	if err := m.EnsurePortFree(MCPPlaywright, port); err != nil {
		return fmt.Errorf("failed to free port %d: %w", port, err)
	}

	// Build arguments for npx @playwright/mcp
	args := []string{
		"@playwright/mcp@latest",
		"--port", fmt.Sprintf("%d", port),
		"--host", "0.0.0.0",           // Bind to all interfaces so containers can access via host.docker.internal
		"--allowed-hosts", "*",        // Allow connections from any host (needed for Docker containers)
	}

	// Add browser type
	browser := m.cfg.HostMCPs.GetPlaywrightBrowser()
	if browser != "" {
		args = append(args, "--browser", browser)
	}

	// Add headless flag
	if m.cfg.HostMCPs.IsPlaywrightHeadless() {
		args = append(args, "--headless")
	}

	// Start the process
	pid, err := m.startProcess(MCPPlaywright, "npx", args...)
	if err != nil {
		return err
	}

	// Wait a moment and verify it's running
	time.Sleep(500 * time.Millisecond)
	if !m.isProcessRunning(pid) {
		logs, _ := m.GetLogs(MCPPlaywright, 20)
		return fmt.Errorf("playwright MCP failed to start. Check logs:\n%s", logs)
	}

	return nil
}

// StopPlaywright stops the Playwright MCP server
func (m *Manager) StopPlaywright() error {
	return m.StopMCP(MCPPlaywright)
}

// IsPlaywrightRunning checks if Playwright MCP is running
func (m *Manager) IsPlaywrightRunning() bool {
	pid, err := m.readPID(MCPPlaywright)
	if err != nil {
		return false
	}
	return m.isProcessRunning(pid)
}

// CheckPlaywrightInstalled verifies that npx and playwright are available
func CheckPlaywrightInstalled() error {
	// Check if npx is available
	_, err := exec.LookPath("npx")
	if err != nil {
		return fmt.Errorf("npx not found. Please install Node.js: https://nodejs.org/")
	}
	return nil
}
