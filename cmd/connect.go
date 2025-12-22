package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"syscall"

	"github.com/spf13/cobra"
)

var connectCmd = &cobra.Command{
	Use:   "connect <id>",
	Short: "Connect to agent and launch Claude",
	Long:  "Connect to specified agent (queen, 1-10) and launch Claude",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		id := args[0]

		// Map shortcuts
		containerName := mapAgentID(id)

		fmt.Printf("🔗 Connecting to %s...\n", containerName)

		// Launch Claude in the container
		// The workspace name is read from WORKSPACE_NAME env var in container
		command := []string{"exec", "-it", containerName, "bash", "-l", "-c",
			`cd /workspace/${WORKSPACE_NAME:-my-project} && exec claude --dangerously-skip-permissions --model "${CLAUDE_MODEL:-sonnet}"`}

		dockerCmd := exec.Command("docker", command...)
		dockerCmd.Stdin = os.Stdin
		dockerCmd.Stdout = os.Stdout
		dockerCmd.Stderr = os.Stderr

		// Use exec syscall to replace process for proper signal handling
		if err := dockerCmd.Run(); err != nil {
			if exitErr, ok := err.(*exec.ExitError); ok {
				os.Exit(exitErr.ExitCode())
			}
			return fmt.Errorf("failed to connect: %w", err)
		}

		return nil
	},
}

func mapAgentID(id string) string {
	switch id {
	case "queen", "q", "0":
		return "claude-queen"
	default:
		return fmt.Sprintf("claude-agent-%s", id)
	}
}

// For proper signal handling
func execReplaceProcess(cmd *exec.Cmd) error {
	argv := append([]string{cmd.Path}, cmd.Args[1:]...)
	return syscall.Exec(cmd.Path, argv, os.Environ())
}

func init() {
	rootCmd.AddCommand(connectCmd)
}
