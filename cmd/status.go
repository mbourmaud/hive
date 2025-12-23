package cmd

import (
	"bytes"
	"fmt"
	"os/exec"

	"github.com/spf13/cobra"
)

var statusCmd = &cobra.Command{
	Use:     "status",
	Aliases: []string{"ps"},
	Short:   "Show hive status",
	Long:    "Display running containers",
	RunE: func(cmd *cobra.Command, args []string) error {
		fmt.Println("🐝 HIVE Status")

		// Show running containers
		dockerCmd := exec.Command("docker", "compose", "ps", "--format", "table")

		var out bytes.Buffer
		dockerCmd.Stdout = &out
		dockerCmd.Stderr = &out

		if err := dockerCmd.Run(); err != nil {
			fmt.Println("❌ Failed to get status:", err)
			return err
		}

		fmt.Println(out.String())
		return nil
	},
}

func init() {
	rootCmd.AddCommand(statusCmd)
}
