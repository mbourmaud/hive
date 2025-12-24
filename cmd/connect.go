package cmd

import (
	"fmt"
	"os"
	"os/exec"

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
		isQueen := (id == "queen" || id == "q" || id == "0")

		fmt.Printf("🔗 Connecting to %s...\n", containerName)

		// Create role-specific initial prompt
		var initialPrompt string
		if isQueen {
			initialPrompt = "Read your role and instructions from /home/agent/CLAUDE.md. You are the Queen (Orchestrator). Execute your mandatory startup sequence immediately: 1. Report your identity, 2. Run hive-status, 3. Report current HIVE state to me."
		} else {
			initialPrompt = "Read your role and instructions from /home/agent/CLAUDE.md. Execute your mandatory startup sequence immediately: 1. Report your agent ID, 2. Run my-tasks, 3. Take action based on what you find."
		}

		// Launch Claude in the container with initial prompt
		// The workspace name is read from WORKSPACE_NAME env var in container
		claudeCmd := fmt.Sprintf(
			`cd /workspace/${WORKSPACE_NAME:-my-project} && exec claude --dangerously-skip-permissions --model "${CLAUDE_MODEL:-sonnet}" "%s"`,
			initialPrompt,
		)

		command := []string{"exec", "-it", containerName, "bash", "-l", "-c", claudeCmd}

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

func init() {
	rootCmd.AddCommand(connectCmd)
}
