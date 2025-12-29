package cmd

import (
	"os"

	"github.com/mbourmaud/hive/internal/compose"
	"github.com/mbourmaud/hive/internal/config"
)

// generateDockerCompose creates docker-compose.yml with the specified number of workers
func generateDockerCompose(workers int) error {
	return generateDockerComposeWithConfig(workers, 6379)
}

// generateDockerComposeWithConfig creates docker-compose.yml with full config options
func generateDockerComposeWithConfig(workers int, redisPort int) error {
	content := compose.GenerateWithOptions(compose.Options{
		WorkerCount:     workers,
		RedisPort:       redisPort,
		ContainerPrefix: "hive",
	})
	return os.WriteFile(".hive/docker-compose.yml", []byte(content), 0644)
}

// generateDockerComposeFromConfig creates docker-compose.yml from a full Config object
func generateDockerComposeFromConfig(cfg *config.Config) error {
	opts := compose.Options{
		WorkerCount:      cfg.Agents.Workers.Count,
		RedisPort:        cfg.Redis.Port,
		ContainerPrefix:  cfg.GetContainerPrefix(),
		QueenDockerfile:  cfg.Agents.Queen.Dockerfile,
		WorkerDockerfile: cfg.Agents.Workers.Dockerfile,
		QueenPorts:       cfg.Agents.Queen.Ports,
		WorkerPorts:      cfg.Agents.Workers.Ports,
		PortsPerDrone:    cfg.Agents.Workers.PortsPerDrone,
		ExtraVolumes:     cfg.Volumes,
		PlaywrightMode:   cfg.Playwright.Mode,
		BrowserEndpoint:  cfg.Playwright.BrowserEndpoint,
		CACertPath:       cfg.Network.CACert,
		ExtraHosts:       cfg.Network.ExtraHosts,
		NetworkEnv:       cfg.Network.Env,
	}
	content := compose.GenerateWithOptions(opts)
	return os.WriteFile(".hive/docker-compose.yml", []byte(content), 0644)
}
