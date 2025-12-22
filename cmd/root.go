package cmd

import (
	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "hive",
	Short: "🐝 HIVE - Claude Multi-Agent System",
	Long: `HIVE - Run multiple Claude Code agents in isolated Docker containers.

Core Commands:
  start [N]        Start queen + N workers (default: 2)
  stop [N|all]     Stop containers (default: all)
  rm [N|all]       Remove containers (default: all)
  status           Show running containers

Direct Access:
  queen            Start Queen and launch Claude
  connect <id>     Connect to agent (queen, 1, 2, etc.)`,
}

func Execute() error {
	return rootCmd.Execute()
}

func init() {
	rootCmd.CompletionOptions.DisableDefaultCmd = true
}
