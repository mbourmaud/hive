package cmd

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"syscall"

	"github.com/mbourmaud/hive/internal/hub"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/mbourmaud/hive/internal/worktree"
	"github.com/spf13/cobra"
)

var hubCmd = &cobra.Command{
	Use:   "hub",
	Short: "Start the Hive hub server",
	Long: `Start the Hive hub server for managing agents via HTTP API.

The hub provides a REST API for:
  - Spawning and managing agents
  - Sending messages to agents
  - Real-time events via Server-Sent Events

Examples:
  hive hub                    # Start on default port 8080
  hive hub --port 3000        # Start on custom port
  hive hub --no-sandbox       # Disable sandbox for agents`,
	RunE: runHub,
}

var (
	hubPort      int
	hubNoSandbox bool
	hubBasePort  int
)

func init() {
	rootCmd.AddCommand(hubCmd)

	hubCmd.Flags().IntVarP(&hubPort, "port", "p", 8080, "Hub server port")
	hubCmd.Flags().BoolVar(&hubNoSandbox, "no-sandbox", false, "Disable sandbox for spawned agents")
	hubCmd.Flags().IntVar(&hubBasePort, "agent-port", 3284, "Base port for agent AgentAPI instances")
}

func runHub(cmd *cobra.Command, args []string) error {
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
		Port:         hubPort,
		WorktreesDir: home + "/hive-worktrees",
		BasePort:     hubBasePort,
		RepoPath:     repoRoot,
		Sandbox:      !hubNoSandbox,
	}

	// Create hub
	h, err := hub.New(cfg)
	if err != nil {
		return fmt.Errorf("failed to create hub: %w", err)
	}

	// Setup graceful shutdown
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		<-sigCh
		fmt.Println()
		fmt.Printf("%s Shutting down...\n", ui.StyleYellow.Render("⚠️"))
		cancel()
		h.Stop(context.Background())
	}()

	fmt.Printf("%s Hive Hub starting on port %d\n", ui.StyleCyan.Render("🐝"), hubPort)
	fmt.Println()
	fmt.Printf("  %s http://localhost:%d\n", ui.StyleDim.Render("API:"), hubPort)
	fmt.Printf("  %s http://localhost:%d/health\n", ui.StyleDim.Render("Health:"), hubPort)
	fmt.Printf("  %s http://localhost:%d/ws\n", ui.StyleDim.Render("Events:"), hubPort)
	fmt.Println()
	fmt.Println(ui.StyleDim.Render("Press Ctrl+C to stop"))
	fmt.Println()

	// Start hub
	if err := h.Start(ctx); err != nil {
		if err.Error() != "http: Server closed" {
			return fmt.Errorf("hub error: %w", err)
		}
	}

	fmt.Printf("%s Hub stopped\n", ui.StyleGreen.Render("✓"))
	return nil
}
