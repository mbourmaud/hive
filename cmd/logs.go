package cmd

import (
	"fmt"
	"os"
	"os/exec"

	"github.com/spf13/cobra"
)

var (
	logsFollow bool
	logsTail   int
)

var logsCmd = &cobra.Command{
	Use:   "logs <id>",
	Short: "View container logs",
	Long: `View logs for a specific agent container.

Examples:
  hive logs queen         # View queen logs
  hive logs 1             # View worker 1 logs
  hive logs queen -f      # Follow logs in real-time
  hive logs 1 --tail 50   # Show last 50 lines`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		id := args[0]
		containerName := mapAgentID(id)

		fmt.Printf("Logs for %s:\n\n", containerName)

		// Build docker logs command
		dockerArgs := []string{"logs"}

		if logsFollow {
			dockerArgs = append(dockerArgs, "-f")
		}

		if logsTail > 0 {
			dockerArgs = append(dockerArgs, "--tail", fmt.Sprintf("%d", logsTail))
		}

		dockerArgs = append(dockerArgs, containerName)

		dockerCmd := exec.Command("docker", dockerArgs...)
		dockerCmd.Stdout = os.Stdout
		dockerCmd.Stderr = os.Stderr

		if err := dockerCmd.Run(); err != nil {
			return fmt.Errorf("failed to get logs for %s: %w", containerName, err)
		}

		return nil
	},
}

func init() {
	rootCmd.AddCommand(logsCmd)
	logsCmd.Flags().BoolVarP(&logsFollow, "follow", "f", false, "Follow log output")
	logsCmd.Flags().IntVar(&logsTail, "tail", 100, "Number of lines to show from the end of the logs")
}
