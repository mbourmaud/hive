package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"strconv"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/spf13/cobra"
)

var startCmd = &cobra.Command{
	Use:   "start [count]",
	Short: "Start queen + N workers",
	Long:  "Start the hive with queen and specified number of workers (default: 2)",
	Args:  cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		// Load config (from hive.yaml or defaults)
		cfg := config.LoadOrDefault()

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

		fmt.Printf("🐝 Starting hive: Queen + %d workers...\n", count)

		// Build services list (Redis must start first)
		services := []string{"redis", "queen"}
		for i := 1; i <= count; i++ {
			services = append(services, fmt.Sprintf("agent-%d", i))
		}

		// Start docker compose services
		cmdArgs := append([]string{"compose", "up", "-d"}, services...)
		dockerCmd := exec.Command("docker", cmdArgs...)
		dockerCmd.Stdout = os.Stdout
		dockerCmd.Stderr = os.Stderr

		if err := dockerCmd.Run(); err != nil {
			return fmt.Errorf("failed to start containers: %w", err)
		}

		fmt.Printf("✅ Hive started: %d containers\n", len(services))
		return nil
	},
}

func init() {
	rootCmd.AddCommand(startCmd)
}
