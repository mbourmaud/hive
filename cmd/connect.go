package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"regexp"
	"strings"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/spf13/cobra"
)

// validAgentIDPattern matches valid agent IDs (queen, q, 0, or numbers 1-99)
var validAgentIDPattern = regexp.MustCompile(`^(queen|q|0|[1-9][0-9]?)$`)

var resumeFlag bool

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
			initialPrompt = `You are the QUEEN (Orchestrator) in a HIVE multi-agent system.

FIRST: Read your instruction files:
1. /home/agent/CLAUDE-ROLE.md - Your role-specific instructions
2. /home/agent/HIVE-CAPABILITIES.md - All available MCP tools (Playwright, iOS, Clipboard)

Startup sequence:
1. Report your identity (Queen)
2. Check HIVE status using MCP tool: hive_status
3. List drones: hive_list_drones
4. Report summary and await instructions

You have these MCP tools (use them directly, not bash):
- hive MCP: hive_status, hive_list_drones, hive_assign_task, hive_get_drone_logs
- playwright MCP: browser_navigate, browser_click, browser_type, browser_screenshot
- ios MCP: ios_list_devices, ios_boot_device, ios_open_url, ios_screenshot
- clipboard MCP: clipboard_read_text, clipboard_write_text, clipboard_read_image

You can assign tasks to drones:
  hive_assign_task(drone="drone-1", title="Implement feature X", description="...")`
		} else {
			initialPrompt = `You are a WORKER DRONE in a HIVE multi-agent system.

FIRST: Read your instruction files:
1. /home/agent/CLAUDE-ROLE.md - Your role-specific instructions
2. /home/agent/HIVE-CAPABILITIES.md - All available MCP tools (Playwright, iOS, Clipboard, testing)

Startup sequence:
1. Report your agent ID (check $AGENT_NAME env var)
2. Check your tasks using MCP tool: hive_my_tasks
3. If you have a task, work on it
4. If no tasks, use hive_start_monitoring then poll with hive_get_monitoring_events

You have these MCP tools (use them directly, not bash):
- hive MCP: hive_my_tasks, hive_take_task, hive_complete_task, hive_log_activity
- playwright MCP: browser_navigate, browser_click, browser_type, browser_screenshot
- ios MCP: ios_list_devices, ios_boot_device, ios_open_url, ios_screenshot
- clipboard MCP: clipboard_read_text, clipboard_write_text, clipboard_read_image

For AUTONOMOUS TESTING of your work:
1. Start app: npm run dev (bind to 0.0.0.0)
2. Get URL: hive_get_test_url(port=3000) -> returns host-accessible URL
3. Test: browser_navigate(url), browser_screenshot()
4. Complete: hive_complete_task(result="tested with screenshots")`
		}

		// Launch Claude in the container
		// Workspace is at /workspace (worktree root)
		var claudeCmd string
		if resumeFlag {
			// Resume previous conversation using --continue
			claudeCmd = `cd /workspace && exec claude --dangerously-skip-permissions --model "${CLAUDE_MODEL:-sonnet}" --continue`
		} else {
			// New session with initial prompt
			// Shell-escape the prompt to prevent injection
			escapedPrompt := shellEscape(initialPrompt)
			claudeCmd = fmt.Sprintf(
				`cd /workspace && exec claude --dangerously-skip-permissions --model "${CLAUDE_MODEL:-sonnet}" %s`,
				escapedPrompt,
			)
		}

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

// mapAgentID converts a user-friendly agent ID to container name
// Uses the container prefix from the current project's config
func mapAgentID(id string) string {
	prefix := getContainerPrefix()
	return mapAgentIDWithPrefix(id, prefix)
}

// mapAgentIDWithPrefix converts a user-friendly agent ID to container name with explicit prefix
func mapAgentIDWithPrefix(id string, prefix string) string {
	switch id {
	case "queen", "q", "0":
		return fmt.Sprintf("%s-queen", prefix)
	default:
		return fmt.Sprintf("%s-drone-%s", prefix, id)
	}
}

// getContainerPrefix returns the container prefix for the current project
func getContainerPrefix() string {
	cfg := config.LoadOrDefault()
	return cfg.GetContainerPrefix()
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
	connectCmd.Flags().BoolVarP(&resumeFlag, "resume", "r", false, "Resume previous conversation with this agent")
}
