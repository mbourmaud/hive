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
2. Check /hive-config/hive.yaml for required tools and MCPs:
   a. Read the 'tools:' list and verify EACH tool is installed:
      - Run: <tool> --version (e.g., glab --version, psql --version, jq --version)
      - If a tool is MISSING, report it clearly and explain how to fix:
        * For CLI tools: Add to hive.yaml hooks.init section, e.g.:
          hooks:
            init: |
              apt-get update && apt-get install -y <package>
        * For global npm packages: npm install -g <package>
   b. Read the 'mcps:' section and verify MCPs are configured:
      - Run: claude mcp list
      - For each MCP in hive.yaml, check if it appears in the list
      - If an MCP is MISSING, report it and explain the fix:
        * Check if required env vars are set (e.g., GITLAB_TOKEN, JIRA_TOKEN)
        * For env vars: Add them to .hive/.env or hive.yaml mcps.<name>.env
        * Example fix for GitLab MCP:
          mcps:
            gitlab:
              package: "@anthropic-ai/mcp-gitlab"
              env: [GITLAB_TOKEN]
   c. Report a SUMMARY with status of each tool/MCP:
      ✅ glab: installed (v1.2.3)
      ✅ playwright: configured
      ❌ psql: MISSING - add to hooks.init: apt-get install -y postgresql-client
      ❌ jira: MISSING ENV - set JIRA_TOKEN in .hive/.env
3. Run hive-status to see current HIVE state
4. Check monitoring configuration and start background monitoring if enabled
5. Report current HIVE state and await instructions

IMPORTANT: You can monitor drone activity in real-time via Redis streams (hive:logs:drone-1, hive:logs:all). Use this to track what drones are doing.`
		} else {
			initialPrompt = `Read your role and instructions from /home/agent/CLAUDE.md. Execute your mandatory startup sequence immediately:
1. Report your agent ID
2. Verify tools and MCPs from /hive-config/hive.yaml:
   a. Read the 'tools:' list and verify each tool: <tool> --version
   b. Read the 'mcps:' section and run: claude mcp list
   c. Report any MISSING tools/MCPs with a clear summary:
      ✅ tool: installed
      ❌ tool: MISSING - explain how to fix (add to hooks.init or set env var)
3. Run my-tasks
4. Take action based on what you find`
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
}
