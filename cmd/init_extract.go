package cmd

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/mbourmaud/hive/internal/embed"
)

// extractHiveFiles copies all necessary hive files to .hive/ directory
// Note: docker-compose.yml is generated dynamically by generateDockerCompose()
func extractHiveFiles(projectType string) error {
	hiveDir := ".hive"

	// Create .hive directory
	if err := os.MkdirAll(hiveDir, 0755); err != nil {
		return fmt.Errorf("failed to create .hive directory: %w", err)
	}

	// docker-compose.yml is generated dynamically after worker count is known
	// See generateDockerCompose()

	// Extract entrypoint.sh
	if err := embed.ExtractFile("entrypoint.sh", filepath.Join(hiveDir, "entrypoint.sh")); err != nil {
		return fmt.Errorf("failed to extract entrypoint.sh: %w", err)
	}

	// Extract worker daemon files for autonomous mode
	if err := embed.ExtractFile("start-worker.sh", filepath.Join(hiveDir, "start-worker.sh")); err != nil {
		return fmt.Errorf("failed to extract start-worker.sh: %w", err)
	}
	if err := embed.ExtractFile("worker-daemon.py", filepath.Join(hiveDir, "worker-daemon.py")); err != nil {
		return fmt.Errorf("failed to extract worker-daemon.py: %w", err)
	}
	if err := embed.ExtractFile("backends.py", filepath.Join(hiveDir, "backends.py")); err != nil {
		return fmt.Errorf("failed to extract backends.py: %w", err)
	}
	if err := embed.ExtractFile("tools.py", filepath.Join(hiveDir, "tools.py")); err != nil {
		return fmt.Errorf("failed to extract tools.py: %w", err)
	}

	// Extract docker directory
	if err := embed.ExtractDir("docker", filepath.Join(hiveDir, "docker")); err != nil {
		return fmt.Errorf("failed to extract docker/: %w", err)
	}

	// Extract scripts directory
	if err := embed.ExtractDir("scripts", filepath.Join(hiveDir, "scripts")); err != nil {
		return fmt.Errorf("failed to extract scripts/: %w", err)
	}

	// Extract templates directory
	if err := embed.ExtractDir("templates", filepath.Join(hiveDir, "templates")); err != nil {
		return fmt.Errorf("failed to extract templates/: %w", err)
	}

	// Create workspaces directory inside .hive
	if err := os.MkdirAll(filepath.Join(hiveDir, "workspaces"), 0755); err != nil {
		return fmt.Errorf("failed to create workspaces directory: %w", err)
	}

	return nil
}
