package cmd

import (
	"os"

	"github.com/mbourmaud/hive/internal/compose"
)

// generateDockerCompose creates docker-compose.yml with the specified number of workers
func generateDockerCompose(workers int) error {
	return generateDockerComposeWithConfig(workers, 6379)
}

// generateDockerComposeWithConfig creates docker-compose.yml with full config options
func generateDockerComposeWithConfig(workers int, redisPort int) error {
	content := compose.GenerateWithOptions(compose.Options{
		WorkerCount: workers,
		RedisPort:   redisPort,
	})
	return os.WriteFile(".hive/docker-compose.yml", []byte(content), 0644)
}
