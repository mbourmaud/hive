package cmd

import (
	"fmt"
	"io"
	"os"
	"path/filepath"

	"github.com/mbourmaud/hive/internal/config"
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

	// Create shared directory for host MCP ↔ container communication (screenshots, files)
	if err := os.MkdirAll(filepath.Join(hiveDir, "shared"), 0755); err != nil {
		return fmt.Errorf("failed to create shared directory: %w", err)
	}

	return nil
}

// syncCustomDockerfiles copies custom Dockerfiles from project root to .hive/
// This is needed because hive update extracts default Dockerfiles, but users may
// have custom Dockerfiles configured in hive.yaml (e.g., docker/Dockerfile.millenium)
func syncCustomDockerfiles(cfg *config.Config) error {
	dockerfiles := []string{cfg.Agents.Queen.Dockerfile, cfg.Agents.Workers.Dockerfile}

	for _, dockerfile := range dockerfiles {
		if dockerfile == "" {
			continue
		}

		// Source is relative to project root
		srcPath := dockerfile
		// Destination is inside .hive/
		dstPath := filepath.Join(".hive", dockerfile)

		// Check if source exists
		if _, err := os.Stat(srcPath); os.IsNotExist(err) {
			// Source doesn't exist, skip (will use embedded default)
			continue
		}

		// Create destination directory
		dstDir := filepath.Dir(dstPath)
		if err := os.MkdirAll(dstDir, 0755); err != nil {
			return fmt.Errorf("failed to create directory %s: %w", dstDir, err)
		}

		// Copy the file
		src, err := os.Open(srcPath)
		if err != nil {
			return fmt.Errorf("failed to open %s: %w", srcPath, err)
		}
		defer src.Close()

		dst, err := os.Create(dstPath)
		if err != nil {
			return fmt.Errorf("failed to create %s: %w", dstPath, err)
		}
		defer dst.Close()

		if _, err := io.Copy(dst, src); err != nil {
			return fmt.Errorf("failed to copy %s: %w", dockerfile, err)
		}
	}

	return nil
}

// copyCACertificate copies the CA certificate to .hive/ for Docker build
// This is required for corporate proxies (like Zscaler) that intercept HTTPS traffic
func copyCACertificate(cfg *config.Config) error {
	if cfg.Network.CACert == "" {
		return nil // No CA cert configured
	}

	srcPath := cfg.Network.CACert
	dstPath := filepath.Join(".hive", "ca-cert.crt")

	// Open source file
	src, err := os.Open(srcPath)
	if err != nil {
		return fmt.Errorf("failed to open CA certificate %s: %w", srcPath, err)
	}
	defer src.Close()

	// Create destination file
	dst, err := os.Create(dstPath)
	if err != nil {
		return fmt.Errorf("failed to create CA certificate in .hive/: %w", err)
	}
	defer dst.Close()

	// Copy content
	if _, err := io.Copy(dst, src); err != nil {
		return fmt.Errorf("failed to copy CA certificate: %w", err)
	}

	return nil
}
