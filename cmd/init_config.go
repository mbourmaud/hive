package cmd

import (
	"os"
	"path/filepath"

	"github.com/mbourmaud/hive/internal/config"
)

// fileExists checks if a file exists at the given path
func fileExists(path string) bool {
	absPath, err := filepath.Abs(path)
	if err != nil {
		absPath = path
	}
	_, err = os.Stat(absPath)
	return err == nil
}

// writeHiveYAML writes or updates the hive.yaml configuration file
// If hive.yaml already exists, it preserves user's custom settings (network, volumes, ports, etc.)
// and only updates fields explicitly provided via CLI
func writeHiveYAML(cfgMap map[string]string, workers int) error {
	var cfg *config.Config

	// Check if hive.yaml already exists - preserve user's custom config
	if fileExists("hive.yaml") {
		existingCfg, err := config.Load("hive.yaml")
		if err == nil {
			cfg = existingCfg
		} else {
			// Existing file is invalid, use defaults
			cfg = config.Default()
		}
	} else {
		cfg = config.Default()
	}

	// Only update fields that are explicitly provided (don't overwrite with empty values)
	if ws := cfgMap["WORKSPACE_NAME"]; ws != "" {
		cfg.Workspace.Name = ws
	}
	if gitURL := cfgMap["GIT_REPO_URL"]; gitURL != "" {
		cfg.Workspace.GitURL = gitURL
	}

	// Update models only if explicitly set
	if queenModel := cfgMap["QUEEN_MODEL"]; queenModel != "" {
		cfg.Agents.Queen.Model = queenModel
	}
	if workerModel := cfgMap["WORKER_MODEL"]; workerModel != "" {
		cfg.Agents.Workers.Model = workerModel
	}

	// Update worker count (always update since it's a CLI argument)
	cfg.Agents.Workers.Count = workers

	// Update dockerfile only if explicitly set
	if dockerfile := cfgMap["HIVE_DOCKERFILE"]; dockerfile != "" {
		cfg.Agents.Queen.Dockerfile = dockerfile
		cfg.Agents.Workers.Dockerfile = dockerfile
	}

	return cfg.Save("hive.yaml")
}
