package cmd

import (
	"fmt"
	"os"
	"os/exec"

	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var DebugMode bool

var skipHealthcheck = map[string]bool{
	"setup":   true,
	"install": true,
	"help":    true,
	"version": true,
}

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
		if os.Getenv("HIVE_DEBUG") == "1" || os.Getenv("HIVE_DEBUG") == "true" {
			DebugMode = true
		}

		if !skipHealthcheck[cmd.Name()] && cmd.Name() != "hive" {
			runQuickHealthcheck()
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
	rootCmd.Version = Version
	rootCmd.SetVersionTemplate(GetVersionString())
	rootCmd.PersistentFlags().BoolVar(&DebugMode, "debug", false, "Enable debug mode (shows all commands)")
}

func runQuickHealthcheck() {
	var missing []string

	if _, err := exec.LookPath("agentapi"); err != nil {
		missing = append(missing, "agentapi")
	}

	if _, err := exec.LookPath("claude"); err != nil {
		missing = append(missing, "claude")
	}

	if len(missing) > 0 {
		fmt.Fprintf(os.Stderr, "%s Missing dependencies: %v\n", ui.StyleYellow.Render("⚠"), missing)
		fmt.Fprintf(os.Stderr, "  Run %s to install them.\n\n", ui.StyleCyan.Render("hive setup"))
	}
}
