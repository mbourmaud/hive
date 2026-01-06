package cmd

import (
	"context"
	"fmt"
	"os"
	"syscall"
	"time"

	"github.com/mbourmaud/hive/internal/ui"
	"github.com/mbourmaud/hive/internal/worktree"
	"github.com/spf13/cobra"
)

var killCmd = &cobra.Command{
	Use:   "kill <agent>",
	Short: "Stop a running agent",
	Long: `Stop a running agent process.

Examples:
  hive kill front         # Stop agent named "front"
  hive kill --all         # Stop all agents`,
	Args: cobra.MaximumNArgs(1),
	RunE: runKill,
}

var killAll bool

func init() {
	rootCmd.AddCommand(killCmd)
	killCmd.Flags().BoolVarP(&killAll, "all", "a", false, "Stop all agents")
}

func runKill(cmd *cobra.Command, args []string) error {
	if killAll {
		return killAllAgents()
	}

	if len(args) == 0 {
		return fmt.Errorf("agent name required (or use --all)")
	}

	return killAgent(args[0])
}

func killAgent(nameOrID string) error {
	a, err := getAgentFromState(nameOrID)
	if err != nil {
		return fmt.Errorf("agent not found: %w", err)
	}

	fmt.Printf("%s Stopping agent %s...\n", ui.StyleYellow.Render("⚠️"), ui.StyleBold.Render(a.Name))

	// Kill the process
	if a.PID > 0 {
		proc, err := os.FindProcess(a.PID)
		if err == nil {
			proc.Signal(syscall.SIGTERM)
			// Wait a bit for graceful shutdown
			time.Sleep(500 * time.Millisecond)
			proc.Kill()
		}
	}

	// Remove from state
	if err := removeAgentFromState(a.Name); err != nil {
		fmt.Printf("%s Failed to update state: %v\n", ui.StyleYellow.Render("⚠️"), err)
	}

	fmt.Printf("%s Agent %s stopped\n", ui.StyleGreen.Render("✓"), a.Name)
	return nil
}

func killAllAgents() error {
	agents, err := loadAgentState()
	if err != nil {
		if os.IsNotExist(err) {
			fmt.Println(ui.StyleDim.Render("No agents running"))
			return nil
		}
		return err
	}

	if len(agents) == 0 {
		fmt.Println(ui.StyleDim.Render("No agents running"))
		return nil
	}

	fmt.Printf("%s Stopping %d agent(s)...\n", ui.StyleYellow.Render("⚠️"), len(agents))

	for _, a := range agents {
		if a.PID > 0 {
			proc, err := os.FindProcess(a.PID)
			if err == nil {
				proc.Signal(syscall.SIGTERM)
			}
		}
		fmt.Printf("  %s %s\n", ui.StyleRed.Render("●"), a.Name)
	}

	// Wait for graceful shutdown
	time.Sleep(500 * time.Millisecond)

	// Force kill remaining
	for _, a := range agents {
		if a.PID > 0 {
			if proc, err := os.FindProcess(a.PID); err == nil {
				proc.Kill()
			}
		}
	}

	// Clear state
	if err := saveAgentState(nil); err != nil {
		return fmt.Errorf("failed to clear state: %w", err)
	}

	fmt.Printf("%s All agents stopped\n", ui.StyleGreen.Render("✓"))
	return nil
}

var destroyCmd = &cobra.Command{
	Use:   "destroy <agent>",
	Short: "Stop agent and remove its worktree",
	Long: `Stop an agent and delete its git worktree.

Examples:
  hive destroy front      # Stop and remove worktree for "front"
  hive destroy --all      # Destroy all agents`,
	Args: cobra.MaximumNArgs(1),
	RunE: runDestroy,
}

var destroyAll bool

func init() {
	rootCmd.AddCommand(destroyCmd)
	destroyCmd.Flags().BoolVarP(&destroyAll, "all", "a", false, "Destroy all agents")
}

func runDestroy(cmd *cobra.Command, args []string) error {
	if destroyAll {
		return destroyAllAgents()
	}

	if len(args) == 0 {
		return fmt.Errorf("agent name required (or use --all)")
	}

	return destroyAgent(args[0])
}

func destroyAgent(nameOrID string) error {
	a, err := getAgentFromState(nameOrID)
	if err != nil {
		return fmt.Errorf("agent not found: %w", err)
	}

	fmt.Printf("%s Destroying agent %s...\n", ui.StyleYellow.Render("⚠️"), ui.StyleBold.Render(a.Name))

	// Kill the process
	if a.PID > 0 {
		proc, err := os.FindProcess(a.PID)
		if err == nil {
			proc.Signal(syscall.SIGTERM)
			time.Sleep(500 * time.Millisecond)
			proc.Kill()
		}
	}

	// Remove worktree
	cwd, _ := os.Getwd()
	repoRoot, err := worktree.GetRepoRoot(cwd)
	if err == nil {
		home, _ := os.UserHomeDir()
		mgr := worktree.NewGitManager(repoRoot, home+"/hive-worktrees")
		if err := mgr.Delete(context.Background(), a.Name); err != nil {
			fmt.Printf("%s Failed to remove worktree: %v\n", ui.StyleYellow.Render("⚠️"), err)
		}
	}

	// Remove from state
	if err := removeAgentFromState(a.Name); err != nil {
		fmt.Printf("%s Failed to update state: %v\n", ui.StyleYellow.Render("⚠️"), err)
	}

	fmt.Printf("%s Agent %s destroyed\n", ui.StyleGreen.Render("✓"), a.Name)
	return nil
}

func destroyAllAgents() error {
	agents, err := loadAgentState()
	if err != nil {
		if os.IsNotExist(err) {
			fmt.Println(ui.StyleDim.Render("No agents to destroy"))
			return nil
		}
		return err
	}

	if len(agents) == 0 {
		fmt.Println(ui.StyleDim.Render("No agents to destroy"))
		return nil
	}

	fmt.Printf("%s Destroying %d agent(s)...\n", ui.StyleYellow.Render("⚠️"), len(agents))

	cwd, _ := os.Getwd()
	repoRoot, _ := worktree.GetRepoRoot(cwd)
	home, _ := os.UserHomeDir()
	var mgr worktree.Manager
	if repoRoot != "" {
		mgr = worktree.NewGitManager(repoRoot, home+"/hive-worktrees")
	}

	for _, a := range agents {
		// Kill process
		if a.PID > 0 {
			if proc, err := os.FindProcess(a.PID); err == nil {
				proc.Signal(syscall.SIGTERM)
			}
		}

		// Remove worktree
		if mgr != nil {
			mgr.Delete(context.Background(), a.Name)
		}

		fmt.Printf("  %s %s\n", ui.StyleRed.Render("●"), a.Name)
	}

	// Wait and force kill
	time.Sleep(500 * time.Millisecond)
	for _, a := range agents {
		if a.PID > 0 {
			if proc, err := os.FindProcess(a.PID); err == nil {
				proc.Kill()
			}
		}
	}

	// Clear state
	if err := saveAgentState(nil); err != nil {
		return fmt.Errorf("failed to clear state: %w", err)
	}

	fmt.Printf("%s All agents destroyed\n", ui.StyleGreen.Render("✓"))
	return nil
}
