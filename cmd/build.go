package cmd

import (
	"fmt"
	"os"
	"os/exec"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/spf13/cobra"
)

var (
	buildNoCache bool
)

var buildCmd = &cobra.Command{
	Use:   "build",
	Short: "Build Docker images",
	Long: `Build or rebuild Docker images for Hive agents.

Examples:
  hive build             # Build images using cache
  hive build --no-cache  # Rebuild images from scratch`,
	RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.LoadOrDefault()

		fmt.Println("Building Hive Docker images...")
		fmt.Println()

		// Get dockerfile from config
		dockerfile := cfg.Agents.Queen.Dockerfile
		if dockerfile == "" {
			dockerfile = "docker/Dockerfile.node"
		}

		fmt.Printf("  Dockerfile: %s\n", dockerfile)
		fmt.Println()

		// Build docker-compose build command
		dockerArgs := []string{"compose", "build"}

		if buildNoCache {
			dockerArgs = append(dockerArgs, "--no-cache")
		}

		dockerCmd := exec.Command("docker", dockerArgs...)
		dockerCmd.Stdout = os.Stdout
		dockerCmd.Stderr = os.Stderr
		dockerCmd.Env = append(os.Environ(), fmt.Sprintf("HIVE_DOCKERFILE=%s", dockerfile))

		if err := dockerCmd.Run(); err != nil {
			return fmt.Errorf("failed to build images: %w", err)
		}

		fmt.Println()
		fmt.Println("Build complete!")
		return nil
	},
}

func init() {
	rootCmd.AddCommand(buildCmd)
	buildCmd.Flags().BoolVar(&buildNoCache, "no-cache", false, "Build without using cache")
}
