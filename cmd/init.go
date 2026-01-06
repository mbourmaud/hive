package cmd

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/mbourmaud/hive/internal/ui"
	"github.com/mbourmaud/hive/internal/worktree"
	"github.com/spf13/cobra"
)

var initCmd = &cobra.Command{
	Use:   "init",
	Short: "Initialize Hive in the current project",
	Long: `Initialize Hive v2 in the current git repository.

This creates the necessary configuration for running agents with git worktree isolation.

Examples:
  hive init                    # Initialize with defaults
  hive init --worktrees-dir ~  # Custom worktrees directory`,
	RunE: runInit,
}

var (
	initWorktreesDir string
)

func init() {
	rootCmd.AddCommand(initCmd)
	initCmd.Flags().StringVar(&initWorktreesDir, "worktrees-dir", "", "Directory for agent worktrees (default: ~/hive-worktrees)")
}

func runInit(cmd *cobra.Command, args []string) error {
	cwd, err := os.Getwd()
	if err != nil {
		return fmt.Errorf("failed to get current directory: %w", err)
	}

	fmt.Printf("%s Initializing Hive v2...\n\n", ui.StyleCyan.Render("🐝"))

	// Check if in a git repo
	if !worktree.IsGitRepository(cwd) {
		return fmt.Errorf("not in a git repository - run 'git init' first")
	}

	repoRoot, err := worktree.GetRepoRoot(cwd)
	if err != nil {
		return fmt.Errorf("failed to get repository root: %w", err)
	}

	fmt.Printf("  %s %s\n", ui.StyleDim.Render("Repository:"), repoRoot)

	// Detect project type
	projectType := detectProjectType(cwd)
	fmt.Printf("  %s %s\n", ui.StyleDim.Render("Project type:"), projectType)

	// Setup worktrees directory
	home, _ := os.UserHomeDir()
	worktreesDir := initWorktreesDir
	if worktreesDir == "" {
		worktreesDir = filepath.Join(home, "hive-worktrees")
	}

	if err := os.MkdirAll(worktreesDir, 0755); err != nil {
		return fmt.Errorf("failed to create worktrees directory: %w", err)
	}

	fmt.Printf("  %s %s\n", ui.StyleDim.Render("Worktrees dir:"), worktreesDir)

	// Create .hive directory for state
	hiveDir := filepath.Join(home, ".hive")
	if err := os.MkdirAll(hiveDir, 0755); err != nil {
		return fmt.Errorf("failed to create .hive directory: %w", err)
	}

	// Check agentapi
	if !isAgentAPIInstalled() {
		fmt.Println()
		fmt.Printf("%s agentapi not found\n", ui.StyleYellow.Render("⚠️"))
		fmt.Println()
		fmt.Println("Install it with:")
		fmt.Println(ui.StyleDim.Render(`  curl -fsSL "https://github.com/coder/agentapi/releases/latest/download/agentapi-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/')" -o /usr/local/bin/agentapi && chmod +x /usr/local/bin/agentapi`))
		fmt.Println()
	}

	fmt.Println()
	fmt.Printf("%s Hive initialized!\n", ui.StyleGreen.Render("✓"))
	fmt.Println()
	fmt.Println("Next steps:")
	fmt.Printf("  %s\n", ui.StyleCyan.Render("hive spawn front"))
	fmt.Printf("  %s\n", ui.StyleCyan.Render("hive spawn back"))
	fmt.Printf("  %s\n", ui.StyleCyan.Render("hive agents"))

	return nil
}
