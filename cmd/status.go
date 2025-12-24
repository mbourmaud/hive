package cmd

import (
	"fmt"
	"os/exec"

	"github.com/mbourmaud/hive/internal/shell"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var statusCmd = &cobra.Command{
	Use:     "status",
	Aliases: []string{"ps"},
	Short:   "Show hive status",
	Long:    "Display running containers",
	RunE: func(cmd *cobra.Command, args []string) error {
		// Header
		fmt.Print(ui.Header("🐝", "Hive Status"))

		// Create shell runner with debug mode
		runner := shell.NewRunner(DebugMode)

		// Show running containers
		dockerCmd := exec.Command("docker", "compose", "-f", ".hive/docker-compose.yml", "ps", "--format", "table")

		// Use runner to execute
		if err := runner.Run(dockerCmd); err != nil {
			fmt.Printf("%s\n\n", ui.Error("Failed to get status"))
			return err
		}

		return nil
	},
}

func init() {
	rootCmd.AddCommand(statusCmd)
}
