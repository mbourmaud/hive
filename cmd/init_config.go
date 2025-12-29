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

// writeHiveYAML writes the hive.yaml configuration file
func writeHiveYAML(cfgMap map[string]string, workers int) error {
	cfg := config.Default()

	// Update workspace
	if ws := cfgMap["WORKSPACE_NAME"]; ws != "" {
		cfg.Workspace.Name = ws
	}
	if gitURL := cfgMap["GIT_REPO_URL"]; gitURL != "" {
		cfg.Workspace.GitURL = gitURL
	}

	// Update models
	if queenModel := cfgMap["QUEEN_MODEL"]; queenModel != "" {
		cfg.Agents.Queen.Model = queenModel
	}
	if workerModel := cfgMap["WORKER_MODEL"]; workerModel != "" {
		cfg.Agents.Workers.Model = workerModel
	}

	// Update worker count
	cfg.Agents.Workers.Count = workers

	// Update dockerfile
	if dockerfile := cfgMap["HIVE_DOCKERFILE"]; dockerfile != "" {
		cfg.Agents.Queen.Dockerfile = dockerfile
		cfg.Agents.Workers.Dockerfile = dockerfile
	}

	return cfg.Save("hive.yaml")
}
