package preflight

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/mbourmaud/hive/internal/ui"
)

// CheckResult represents the result of a preflight check
type CheckResult struct {
	Name    string
	Passed  bool
	Message string
}

// RunAllChecks runs all preflight checks and returns results
func RunAllChecks() []CheckResult {
	var results []CheckResult

	results = append(results, CheckDocker())
	results = append(results, CheckDockerCompose())
	results = append(results, CheckEnvFile())
	results = append(results, CheckDockerComposeFile())
	// Note: Docker socket check removed - if docker info works, socket is accessible
	results = append(results, CheckClaudeConfig())

	return results
}

// CheckDocker verifies Docker is installed and running
func CheckDocker() CheckResult {
	result := CheckResult{Name: "Docker daemon"}

	cmd := exec.Command("docker", "info")
	output, err := cmd.CombinedOutput()
	if err != nil {
		result.Passed = false
		switch {
		case strings.Contains(string(output), "Cannot connect"):
			result.Message = "Docker daemon is not running. Please start Docker."
		case strings.Contains(err.Error(), "executable file not found"):
			result.Message = "Docker is not installed. Please install Docker."
		default:
			result.Message = fmt.Sprintf("Docker check failed: %s", strings.TrimSpace(string(output)))
		}
		return result
	}

	result.Passed = true
	result.Message = "Docker is running"
	return result
}

// CheckDockerCompose verifies Docker Compose is available
func CheckDockerCompose() CheckResult {
	result := CheckResult{Name: "Docker Compose"}

	cmd := exec.Command("docker", "compose", "version")
	output, err := cmd.CombinedOutput()
	if err != nil {
		result.Passed = false
		result.Message = "Docker Compose is not available. Please install Docker Compose V2."
		return result
	}

	result.Passed = true
	result.Message = strings.TrimSpace(string(output))
	return result
}

// CheckEnvFile verifies .hive/.env file exists
func CheckEnvFile() CheckResult {
	result := CheckResult{Name: ".hive/.env file"}

	if _, err := os.Stat(".hive/.env"); os.IsNotExist(err) {
		result.Passed = false
		result.Message = ".hive/.env file not found. Run 'hive init' to create it."
		return result
	}

	result.Passed = true
	result.Message = ".hive/.env file exists"
	return result
}

// CheckDockerComposeFile verifies .hive/docker-compose.yml exists
func CheckDockerComposeFile() CheckResult {
	result := CheckResult{Name: ".hive/docker-compose.yml"}

	if _, err := os.Stat(".hive/docker-compose.yml"); os.IsNotExist(err) {
		result.Passed = false
		result.Message = ".hive/docker-compose.yml not found. Run 'hive init' to create it."
		return result
	}

	result.Passed = true
	result.Message = ".hive/docker-compose.yml exists"
	return result
}

// CheckDockerSocket verifies Docker socket is accessible
func CheckDockerSocket() CheckResult {
	result := CheckResult{Name: "Docker socket"}

	socketPath := "/var/run/docker.sock"
	if _, err := os.Stat(socketPath); os.IsNotExist(err) {
		result.Passed = false
		result.Message = "Docker socket not found at /var/run/docker.sock"
		return result
	}

	// Check if readable
	file, err := os.Open(socketPath)
	if err != nil {
		result.Passed = false
		result.Message = fmt.Sprintf("Cannot access Docker socket: %v", err)
		return result
	}
	_ = file.Close() // nolint:errcheck

	result.Passed = true
	result.Message = "Docker socket is accessible"
	return result
}

// CheckClaudeConfig verifies Claude configuration exists
func CheckClaudeConfig() CheckResult {
	result := CheckResult{Name: "Claude config"}

	homeDir, err := os.UserHomeDir()
	if err != nil {
		result.Passed = false
		result.Message = "Cannot determine home directory"
		return result
	}

	claudeDir := filepath.Join(homeDir, ".claude")
	if _, err := os.Stat(claudeDir); os.IsNotExist(err) {
		result.Passed = false
		result.Message = "~/.claude directory not found. Run 'claude' first to initialize."
		return result
	}

	result.Passed = true
	result.Message = "~/.claude directory exists"
	return result
}

// CheckWorkspaceDir verifies workspace directory is writable
func CheckWorkspaceDir(workspacePath string) CheckResult {
	result := CheckResult{Name: "Workspace directory"}

	// Create if doesn't exist
	if err := os.MkdirAll(workspacePath, 0750); err != nil {
		result.Passed = false
		result.Message = fmt.Sprintf("Cannot create workspace directory: %v", err)
		return result
	}

	// Check if writable
	testFile := filepath.Join(workspacePath, ".hive-test")
	if err := os.WriteFile(testFile, []byte("test"), 0600); err != nil {
		result.Passed = false
		result.Message = fmt.Sprintf("Workspace directory is not writable: %v", err)
		return result
	}
	_ = os.Remove(testFile) // nolint:errcheck

	result.Passed = true
	result.Message = fmt.Sprintf("Workspace directory is ready: %s", workspacePath)
	return result
}

// PrintResults displays check results to the console
func PrintResults(results []CheckResult) bool {
	allPassed := true

	fmt.Print(ui.Header("🔍", "Preflight Checks"))

	for _, r := range results {
		var status string
		if r.Passed {
			status = ui.StyleGreen.Render("✓")
		} else {
			status = ui.StyleRed.Render("✗")
			allPassed = false
		}

		// Format: "  ✓ Docker daemon: Docker is running"
		name := ui.StyleBold.Render(r.Name)
		message := ui.StyleDim.Render(r.Message)
		fmt.Printf("  %s %s: %s\n", status, name, message)
	}

	fmt.Println()
	return allPassed
}
