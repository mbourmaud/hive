package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"time"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/mbourmaud/hive/internal/preflight"
	"github.com/spf13/cobra"
)

var (
	startSkipChecks bool
	startWaitReady  bool
)

var startCmd = &cobra.Command{
	Use:   "start [count]",
	Short: "Start queen + N workers",
	Long:  "Start the hive with queen and specified number of workers (default: 2)",
	Args:  cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		// Load config (from hive.yaml or defaults)
		cfg := config.LoadOrDefault()

		// Run preflight checks unless skipped
		if !startSkipChecks {
			results := preflight.RunAllChecks()
			if !preflight.PrintResults(results) {
				return fmt.Errorf("preflight checks failed. Use --skip-checks to bypass")
			}
		}

		// CLI argument takes precedence over config
		count := cfg.Agents.Workers.Count
		if len(args) > 0 {
			var err error
			count, err = strconv.Atoi(args[0])
			if err != nil {
				return fmt.Errorf("invalid count: %s", args[0])
			}
		}

		if count > 10 {
			return fmt.Errorf("maximum 10 workers allowed")
		}

		if count < 1 {
			return fmt.Errorf("minimum 1 worker required")
		}

		// Create history files to prevent Docker from creating them as directories
		// Docker bind mounts create directories when the source doesn't exist
		hiveDir := ".hive"
		workspacesDir := filepath.Join(hiveDir, "workspaces")
		agents := []string{"queen"}
		for i := 1; i <= count; i++ {
			agents = append(agents, fmt.Sprintf("drone-%d", i))
		}
		for _, agent := range agents {
			historyDir := filepath.Join(workspacesDir, ".history", agent)
			if err := os.MkdirAll(filepath.Join(historyDir, "session-env"), 0755); err != nil {
				return fmt.Errorf("failed to create history dir for %s: %w", agent, err)
			}
			historyFile := filepath.Join(historyDir, "history.jsonl")
			if _, err := os.Stat(historyFile); os.IsNotExist(err) {
				if err := os.WriteFile(historyFile, []byte{}, 0644); err != nil {
					return fmt.Errorf("failed to create history file for %s: %w", agent, err)
				}
			}
		}

		fmt.Printf("Starting hive: Queen + %d workers...\n", count)

		// Build services list (Redis must start first)
		services := []string{"redis", "queen"}
		for i := 1; i <= count; i++ {
			services = append(services, fmt.Sprintf("agent-%d", i))
		}

		// Start docker compose services from .hive directory
		cmdArgs := append([]string{"compose", "-f", filepath.Join(hiveDir, "docker-compose.yml"), "up", "-d"}, services...)
		dockerCmd := exec.Command("docker", cmdArgs...)
		dockerCmd.Stdout = os.Stdout
		dockerCmd.Stderr = os.Stderr
		// Set working directory to .hive for proper path resolution
		dockerCmd.Dir = hiveDir

		if err := dockerCmd.Run(); err != nil {
			return fmt.Errorf("failed to start containers: %w", err)
		}

		// Wait for containers to be healthy if requested
		if startWaitReady {
			fmt.Println()
			fmt.Println("Waiting for containers to be ready...")
			if err := waitForContainersReady(services, 60*time.Second); err != nil {
				return err
			}
		}

		fmt.Println()
		fmt.Printf("Hive started: %d containers\n", len(services))
		fmt.Println()
		fmt.Println("Next steps:")
		fmt.Println("  hive connect queen  # Connect to orchestrator")
		fmt.Println("  hive connect 1      # Connect to worker 1")
		fmt.Println("  hive status         # Check status")
		return nil
	},
}

// waitForContainersReady waits for all containers to be running
func waitForContainersReady(services []string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)

	for _, service := range services {
		var containerName string
		switch service {
		case "queen":
			containerName = "claude-queen"
		case "redis":
			containerName = "hive-redis"
		default:
			containerName = "claude-" + service
		}

		fmt.Printf("  Waiting for %s...", containerName)

		for {
			if time.Now().After(deadline) {
				fmt.Println(" TIMEOUT")
				return fmt.Errorf("timeout waiting for %s to be ready", containerName)
			}

			cmd := exec.Command("docker", "inspect", "-f", "{{.State.Running}}", containerName)
			output, err := cmd.Output()
			if err == nil && string(output) == "true\n" {
				fmt.Println(" OK")
				break
			}

			time.Sleep(1 * time.Second)
		}
	}

	return nil
}

func init() {
	rootCmd.AddCommand(startCmd)
	startCmd.Flags().BoolVar(&startSkipChecks, "skip-checks", false, "Skip preflight checks")
	startCmd.Flags().BoolVar(&startWaitReady, "wait", false, "Wait for containers to be ready")
}
