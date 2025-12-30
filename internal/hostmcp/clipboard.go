package hostmcp

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"time"
)

// StartClipboard starts the Clipboard MCP server on the host
func (m *Manager) StartClipboard() error {
	// Clipboard only works on macOS
	if runtime.GOOS != "darwin" {
		return fmt.Errorf("clipboard MCP is only available on macOS")
	}

	port := m.cfg.HostMCPs.GetClipboardPort()

	// Ensure port is free (kill any existing process)
	if err := m.EnsurePortFree(MCPClipboard, port); err != nil {
		return fmt.Errorf("failed to free port %d: %w", port, err)
	}

	// Find the hive-clipboard-mcp.js script
	scriptPath, err := findClipboardMCPScript()
	if err != nil {
		return err
	}

	// Build arguments for node hive-clipboard-mcp.js
	args := []string{
		scriptPath,
		"--port", fmt.Sprintf("%d", port),
	}

	// Start the process
	pid, err := m.startProcess(MCPClipboard, "node", args...)
	if err != nil {
		return err
	}

	// Wait a moment and verify it's running
	time.Sleep(500 * time.Millisecond)
	if !m.isProcessRunning(pid) {
		logs, _ := m.GetLogs(MCPClipboard, 20)
		return fmt.Errorf("clipboard MCP failed to start. Check logs:\n%s", logs)
	}

	return nil
}

// StopClipboard stops the Clipboard MCP server
func (m *Manager) StopClipboard() error {
	return m.StopMCP(MCPClipboard)
}

// IsClipboardRunning checks if Clipboard MCP is running
func (m *Manager) IsClipboardRunning() bool {
	pid, err := m.readPID(MCPClipboard)
	if err != nil {
		return false
	}
	return m.isProcessRunning(pid)
}

// CheckPngpasteInstalled verifies that pngpaste is available (optional but required for image support)
func CheckPngpasteInstalled() bool {
	_, err := exec.LookPath("pngpaste")
	return err == nil
}

// findClipboardMCPScript locates the hive-clipboard-mcp.js script
// It checks multiple locations in order of preference:
// 1. Project .hive/ directory (extracted by hive init)
// 2. Development: relative to project root
// 3. User's home directory (~/.hive/scripts/)
// 4. Relative to hive binary
func findClipboardMCPScript() (string, error) {
	// Check common locations
	locations := []string{
		// Project .hive/ directory (most common - extracted by hive init)
		filepath.Join(".hive", "scripts", "mcp", "hive-clipboard-mcp.js"),
		// Development: relative to project root
		filepath.Join("scripts", "mcp", "hive-clipboard-mcp.js"),
		// User's home directory
		filepath.Join(getHiveDir(), "scripts", "mcp", "hive-clipboard-mcp.js"),
	}

	// Also check relative to the executable
	if execPath, err := os.Executable(); err == nil {
		execDir := filepath.Dir(execPath)
		locations = append(locations,
			filepath.Join(execDir, "scripts", "mcp", "hive-clipboard-mcp.js"),
			filepath.Join(execDir, "..", "share", "hive", "hive-clipboard-mcp.js"),
		)
	}

	for _, loc := range locations {
		if _, err := os.Stat(loc); err == nil {
			absPath, err := filepath.Abs(loc)
			if err != nil {
				return loc, nil
			}
			return absPath, nil
		}
	}

	return "", fmt.Errorf("hive-clipboard-mcp.js not found. Please ensure hive is installed correctly")
}
