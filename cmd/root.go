package cmd

import (
	"os"

	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

// DebugMode is a global flag for debug output
var DebugMode bool

var rootCmd = &cobra.Command{
	Use:   "hive",
	Short: "HIVE - Claude Multi-Agent System",
	Long: `HIVE - Run multiple Claude Code agents with git worktree isolation.

Commands:
  spawn <name>     Spawn a new agent with git worktree
  agents           List running agents
  msg <agent> <m>  Send message to an agent
  conv <agent>     Show conversation history
  kill <agent>     Stop an agent
  destroy <agent>  Stop agent and remove worktree
  clean            Remove all agents and worktrees

Server:
  hub              Start the Hive API server`,
	PersistentPreRun: func(cmd *cobra.Command, args []string) {
		// Check for HIVE_DEBUG environment variable
		if os.Getenv("HIVE_DEBUG") == "1" || os.Getenv("HIVE_DEBUG") == "true" {
			DebugMode = true
		}
	},
}

func Execute() error {
	return rootCmd.Execute()
}

// GetVersionString returns the formatted version string
func GetVersionString() string {
	title := ui.StyleBold.Render(ui.StyleCyan.Render("hive")) + " " + Version + "\n"
	commit := ui.StyleDim.Render("Commit: ") + GitCommit + "\n"
	built := ui.StyleDim.Render("Built: ") + BuildDate + "\n"
	return title + commit + built
}

func init() {
	rootCmd.CompletionOptions.DisableDefaultCmd = true
	// Use a PersistentPreRun to set version dynamically
	rootCmd.Version = Version
	rootCmd.SetVersionTemplate(GetVersionString())

	// Add global --debug flag
	rootCmd.PersistentFlags().BoolVar(&DebugMode, "debug", false, "Enable debug mode (shows all commands)")
}
