package cmd

import (
	"fmt"
	"os/exec"

	"github.com/mbourmaud/hive/internal/shell"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var stopCmd = &cobra.Command{
	Use:   "stop",
	Short: "Stop all hive containers",
	Long:  "Stop all running hive containers",
	RunE: func(cmd *cobra.Command, args []string) error {
		// Header
		fmt.Print(ui.Header("🛑", "Stopping Hive"))

		// Create shell runner with debug mode
		runner := shell.NewRunner(DebugMode)

		// Stop containers
		dockerCmd := exec.Command("docker", "compose", "-f", ".hive/docker-compose.yml", "down")
		if err := runner.RunWithTitle(dockerCmd, "Docker Compose Stop"); err != nil {
			return fmt.Errorf("failed to stop containers: %w", err)
		}

		// Success message
		fmt.Printf("\n%s\n\n", ui.Success("Hive stopped successfully!"))
		return nil
	},
}

func init() {
	rootCmd.AddCommand(stopCmd)
}
