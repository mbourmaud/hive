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
		fmt.Printf("\n%s%s🐝 Hive Status%s\n\n", colorBold, colorCyan, colorReset)

		// Show running containers
		dockerCmd := exec.Command("docker", "compose", "-f", ".hive/docker-compose.yml", "ps", "--format", "table")

		var out bytes.Buffer
		dockerCmd.Stdout = &out
		dockerCmd.Stderr = &out

		if err := dockerCmd.Run(); err != nil {
			fmt.Printf("%s❌ Failed to get status%s\n\n", colorYellow, colorReset)
			return err
		}

		fmt.Println(out.String())
		return nil
	},
}

func init() {
	rootCmd.AddCommand(statusCmd)
}
