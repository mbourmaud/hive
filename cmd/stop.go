package cmd

import (
	"fmt"
	"os"
	"os/exec"

	"github.com/spf13/cobra"
)

var stopCmd = &cobra.Command{
	Use:   "stop",
	Short: "Stop all hive containers",
	Long:  "Stop all running hive containers",
	RunE: func(cmd *cobra.Command, args []string) error {
		fmt.Println("🛑 Stopping hive...")

		dockerCmd := exec.Command("docker", "compose", "down")
		dockerCmd.Stdout = os.Stdout
		dockerCmd.Stderr = os.Stderr

		if err := dockerCmd.Run(); err != nil {
			return fmt.Errorf("failed to stop containers: %w", err)
		}

		fmt.Println("✅ Hive stopped")
		return nil
	},
}

func init() {
	rootCmd.AddCommand(stopCmd)
}
