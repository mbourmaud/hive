package cmd

import (
	"fmt"
	"os/exec"

	"github.com/spf13/cobra"
)

var stopCmd = &cobra.Command{
	Use:   "stop",
	Short: "Stop all hive containers",
	Long:  "Stop all running hive containers",
	RunE: func(cmd *cobra.Command, args []string) error {
		fmt.Printf("\n%s%s🛑 Stopping Hive%s\n\n", colorBold, colorCyan, colorReset)

		dockerCmd := exec.Command("docker", "compose", "-f", ".hive/docker-compose.yml", "down")
		dockerCmd.Stdout = nil
		dockerCmd.Stderr = nil

		if err := dockerCmd.Run(); err != nil {
			return fmt.Errorf("failed to stop containers: %w", err)
		}

		fmt.Printf("%s%s✨ Hive stopped successfully!%s\n\n", colorBold, colorGreen, colorReset)
		return nil
	},
}

func init() {
	rootCmd.AddCommand(stopCmd)
}
