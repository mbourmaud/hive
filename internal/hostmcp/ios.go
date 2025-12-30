package hostmcp

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"time"
)

// StartIOS starts the iOS MCP server on the host
func (m *Manager) StartIOS() error {
	// iOS only works on macOS
	if runtime.GOOS != "darwin" {
		return fmt.Errorf("iOS MCP is only available on macOS")
	}

	// Check for Xcode/simctl
	if err := CheckXcodeInstalled(); err != nil {
		return err
	}

	port := m.cfg.HostMCPs.GetIOSPort()

	// Ensure port is free (kill any existing process)
	if err := m.EnsurePortFree(MCPIOS, port); err != nil {
		return fmt.Errorf("failed to free port %d: %w", port, err)
	}

	// Find the hive-ios-mcp.js script
	scriptPath, err := findIOSMCPScript()
	if err != nil {
		return err
	}

	// Build arguments for node hive-ios-mcp.js
	args := []string{
		scriptPath,
		"--port", fmt.Sprintf("%d", port),
	}

	// Start the process
	pid, err := m.startProcess(MCPIOS, "node", args...)
	if err != nil {
		return err
	}

	// Wait a moment and verify it's running
	time.Sleep(500 * time.Millisecond)
	if !m.isProcessRunning(pid) {
		logs, _ := m.GetLogs(MCPIOS, 20)
		return fmt.Errorf("iOS MCP failed to start. Check logs:\n%s", logs)
	}

	return nil
}

// StopIOS stops the iOS MCP server
func (m *Manager) StopIOS() error {
	return m.StopMCP(MCPIOS)
}

// IsIOSRunning checks if iOS MCP is running
func (m *Manager) IsIOSRunning() bool {
	pid, err := m.readPID(MCPIOS)
	if err != nil {
		return false
	}
	return m.isProcessRunning(pid)
}

// CheckXcodeInstalled verifies that Xcode and simctl are available
func CheckXcodeInstalled() error {
	// Check if xcrun is available
	_, err := exec.LookPath("xcrun")
	if err != nil {
		return fmt.Errorf("xcrun not found. Please install Xcode Command Line Tools: xcode-select --install")
	}

	// Check if simctl is available
	cmd := exec.Command("xcrun", "simctl", "help")
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("simctl not available. Please install Xcode from the App Store")
	}

	return nil
}

// findIOSMCPScript locates the hive-ios-mcp.js script
// It checks multiple locations in order of preference:
// 1. Project .hive/ directory (extracted by hive init)
// 2. Development: relative to project root
// 3. User's home directory (~/.hive/scripts/)
// 4. Relative to hive binary
func findIOSMCPScript() (string, error) {
	// Check common locations
	locations := []string{
		// Project .hive/ directory (most common - extracted by hive init)
		filepath.Join(".hive", "scripts", "mcp", "hive-ios-mcp.js"),
		// Development: relative to project root
		filepath.Join("scripts", "mcp", "hive-ios-mcp.js"),
		// User's home directory
		filepath.Join(getHiveDir(), "scripts", "mcp", "hive-ios-mcp.js"),
	}

	// Also check relative to the executable
	if execPath, err := os.Executable(); err == nil {
		execDir := filepath.Dir(execPath)
		locations = append(locations,
			filepath.Join(execDir, "scripts", "mcp", "hive-ios-mcp.js"),
			filepath.Join(execDir, "..", "share", "hive", "hive-ios-mcp.js"),
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

	return "", fmt.Errorf("hive-ios-mcp.js not found. Please ensure hive is installed correctly")
}

// getHiveDir returns the path to the user's hive config directory
func getHiveDir() string {
	home, err := os.UserHomeDir()
	if err != nil {
		return ""
	}
	return filepath.Join(home, ".hive")
}
