package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"regexp"
	"strings"

	"github.com/spf13/cobra"
)

// validAgentIDPattern matches valid agent IDs (queen, q, 0, or numbers 1-99)
var validAgentIDPattern = regexp.MustCompile(`^(queen|q|0|[1-9][0-9]?)$`)

var connectCmd = &cobra.Command{
	Use:   "connect <id>",
	Short: "Connect to agent and launch Claude",
	Long:  "Connect to specified agent (queen, 1-10) and launch Claude",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		id := args[0]

		// Validate agent ID to prevent injection
		if !validAgentIDPattern.MatchString(id) {
			return fmt.Errorf("invalid agent ID: %q (must be 'queen', 'q', '0', or a number 1-99)", id)
		}

		// Map shortcuts
		containerName := mapAgentID(id)
		isQueen := (id == "queen" || id == "q" || id == "0")

		fmt.Printf("🔗 Connecting to %s...\n", containerName)

		// Create role-specific initial prompt
		var initialPrompt string
		if isQueen {
			initialPrompt = `Read your role and instructions from /home/agent/CLAUDE.md. You are the Queen (Orchestrator).

Execute your mandatory startup sequence:
1. Report your identity
2. Run hive-status to see current HIVE state
3. Check /hive-config/hive.yaml for monitoring configuration
4. If monitoring.queen.enabled is true, start background monitoring immediately (subscribe to drone activity logs via Redis)
5. Report current HIVE state and confirm monitoring is active

IMPORTANT: You can monitor drone activity in real-time via Redis streams (hive:logs:drone-1, hive:logs:all). Use this to track what drones are doing.`
		} else {
			initialPrompt = "Read your role and instructions from /home/agent/CLAUDE.md. Execute your mandatory startup sequence immediately: 1. Report your agent ID, 2. Run my-tasks, 3. Take action based on what you find."
		}

		// Launch Claude in the container with initial prompt
		// Workspace is at /workspace (worktree root)
		// Shell-escape the prompt to prevent injection
		escapedPrompt := shellEscape(initialPrompt)
		claudeCmd := fmt.Sprintf(
			`cd /workspace && exec claude --dangerously-skip-permissions --model "${CLAUDE_MODEL:-sonnet}" %s`,
			escapedPrompt,
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
		return "hive-queen"
	default:
		return fmt.Sprintf("hive-drone-%s", id)
	}
}

// shellEscape escapes a string for safe use in a shell command
// Uses single quotes and escapes any embedded single quotes
func shellEscape(s string) string {
	// Replace single quotes with '\'' (end quote, escaped quote, start quote)
	escaped := strings.ReplaceAll(s, "'", "'\\''")
	return "'" + escaped + "'"
}

func init() {
	rootCmd.AddCommand(connectCmd)
}
