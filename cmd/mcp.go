package cmd

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"syscall"

	"github.com/mbourmaud/hive/internal/hub"
	"github.com/mbourmaud/hive/internal/mcp"
	"github.com/mbourmaud/hive/internal/worktree"
	"github.com/spf13/cobra"
)

var mcpCmd = &cobra.Command{
	Use:   "mcp",
	Short: "Start the MCP server for Queen integration",
	Long: `Start an MCP (Model Context Protocol) server that allows Claude to control the Hive.

The MCP server provides tools for:
  - Managing agents (spawn, stop, destroy)
  - Sending messages to agents
  - Managing tasks and plans
  - Responding to solicitations from agents
  - Managing port allocations

This command should be configured as an MCP server in Claude's settings.

Example Claude configuration:
  {
    "mcpServers": {
      "hive": {
        "command": "hive",
        "args": ["mcp"]
      }
    }
  }`,
	RunE: runMCP,
}

var (
	mcpHubPort int
)

func init() {
	rootCmd.AddCommand(mcpCmd)

	mcpCmd.Flags().IntVar(&mcpHubPort, "hub-port", 8080, "Hub server port to connect to")
}

func runMCP(cmd *cobra.Command, args []string) error {
	// Get repo root
	cwd, err := os.Getwd()
	if err != nil {
		return fmt.Errorf("failed to get current directory: %w", err)
	}

	repoRoot, err := worktree.GetRepoRoot(cwd)
	if err != nil {
		return fmt.Errorf("not in a git repository: %w", err)
	}

	// Create hub config
	home, _ := os.UserHomeDir()
	cfg := hub.Config{
		Port:         mcpHubPort,
		WorktreesDir: home + "/hive-worktrees",
		BasePort:     3284,
		RepoPath:     repoRoot,
		Sandbox:      true,
	}

	// Create hub
	h, err := hub.New(cfg)
	if err != nil {
		return fmt.Errorf("failed to create hub: %w", err)
	}

	// Create MCP adapter
	hubURL := fmt.Sprintf("http://localhost:%d", mcpHubPort)
	adapter := mcp.NewHubAdapter(h, repoRoot, hubURL)

	// Create MCP server
	server := mcp.NewServer(adapter)

	// Setup graceful shutdown
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		<-sigCh
		cancel()
	}()

	// Start hub in background
	go func() {
		_ = h.Start(ctx)
	}()

	// Run MCP server (reads from stdin, writes to stdout)
	return server.Run(ctx)
}
