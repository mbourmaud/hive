package cmd

import (
	"context"
	"fmt"
	"os"
	"path/filepath"

	"github.com/mbourmaud/hive/internal/ui"
	"github.com/mbourmaud/hive/internal/worktree"
	"github.com/spf13/cobra"
)

var cleanCmd = &cobra.Command{
	Use:   "clean",
	Short: "Clean up all Hive agents and worktrees",
	Long: `Remove all Hive agents and their git worktrees.

This will:
  - Stop all running agents
  - Remove all agent worktrees
  - Clear the agent state file

Examples:
  hive clean              # Clean everything
  hive clean --force      # Skip confirmation`,
	RunE: runClean,
}

var cleanForce bool

func init() {
	rootCmd.AddCommand(cleanCmd)
	cleanCmd.Flags().BoolVarP(&cleanForce, "force", "f", false, "Skip confirmation")
}

func pluralize(n int) string {
	if n > 1 {
		return "s"
	}
	return ""
}

func runClean(cmd *cobra.Command, args []string) error {
	// Kill all agents first
	if err := killAllAgents(); err != nil {
		fmt.Printf("%s Failed to stop some agents: %v\n", ui.StyleYellow.Render("⚠️"), err)
	}

	// Clean worktrees
	cwd, err := os.Getwd()
	if err != nil {
		return err
	}

	if worktree.IsGitRepository(cwd) {
		repoRoot, err := worktree.GetRepoRoot(cwd)
		if err == nil {
			home, _ := os.UserHomeDir()
			mgr := worktree.NewGitManager(repoRoot, filepath.Join(home, "hive-worktrees"))

			worktrees, err := mgr.List(context.Background())
			if err == nil && len(worktrees) > 0 {
				fmt.Printf("%s Removing %d worktree(s)...\n", ui.StyleYellow.Render("🧹"), len(worktrees))
				for _, wt := range worktrees {
					if err := mgr.Delete(context.Background(), wt.Name); err != nil {
						fmt.Printf("  %s Failed to remove %s: %v\n", ui.StyleRed.Render("✗"), wt.Name, err)
					} else {
						fmt.Printf("  %s %s\n", ui.StyleGreen.Render("✓"), wt.Name)
					}
				}
			}

			// Prune orphaned worktrees
			mgr.Prune(context.Background())
		}
	}

	// Clear state file
	stateFile := getStateFilePath()
	if err := os.Remove(stateFile); err != nil && !os.IsNotExist(err) {
		fmt.Printf("%s Failed to remove state file: %v\n", ui.StyleYellow.Render("⚠️"), err)
	}

	fmt.Printf("\n%s Hive cleaned!\n", ui.StyleGreen.Render("✓"))
	return nil
}
