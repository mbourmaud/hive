package cmd

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"time"

	"github.com/mbourmaud/hive/internal/agent"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/mbourmaud/hive/internal/worktree"
	"github.com/spf13/cobra"
)

var spawnCmd = &cobra.Command{
	Use:   "spawn <name>",
	Short: "Spawn a new agent",
	Long: `Spawn a new Claude Code agent with its own git worktree.

The agent runs locally using AgentAPI and Claude Code's native sandbox.

Examples:
  hive spawn front              # Spawn agent named "front"
  hive spawn back --branch dev  # Spawn on specific branch
  hive spawn api --no-sandbox   # Spawn without sandbox (not recommended)`,
	Args: cobra.ExactArgs(1),
	RunE: runSpawn,
}

var (
	spawnBranch     string
	spawnBaseBranch string
	spawnSpecialty  string
	spawnNoSandbox  bool
	spawnPort       int
)

func init() {
	rootCmd.AddCommand(spawnCmd)

	spawnCmd.Flags().StringVarP(&spawnBranch, "branch", "b", "", "Branch to work on (default: hive/<name>)")
	spawnCmd.Flags().StringVar(&spawnBaseBranch, "base", "", "Base branch to create from (default: current branch)")
	spawnCmd.Flags().StringVarP(&spawnSpecialty, "specialty", "s", "fullstack", "Agent specialty (front, back, infra, fullstack)")
	spawnCmd.Flags().BoolVar(&spawnNoSandbox, "no-sandbox", false, "Disable Claude Code sandbox")
	spawnCmd.Flags().IntVarP(&spawnPort, "port", "p", 3284, "Base port for AgentAPI")
}

func runSpawn(cmd *cobra.Command, args []string) error {
	name := args[0]

	// Get repo root
	cwd, err := os.Getwd()
	if err != nil {
		return fmt.Errorf("failed to get current directory: %w", err)
	}

	repoRoot, err := worktree.GetRepoRoot(cwd)
	if err != nil {
		return fmt.Errorf("not in a git repository: %w", err)
	}

	// Check if agentapi is installed
	if !isAgentAPIInstalled() {
		fmt.Println(ui.StyleYellow.Render("⚠️  agentapi not found"))
		fmt.Println()
		fmt.Println("Install it with:")
		fmt.Println(ui.StyleDim.Render(`  curl -fsSL "https://github.com/coder/agentapi/releases/latest/download/agentapi-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/')" -o /usr/local/bin/agentapi && chmod +x /usr/local/bin/agentapi`))
		return fmt.Errorf("agentapi not installed")
	}

	fmt.Printf("%s Spawning agent %s...\n", ui.StyleCyan.Render("🐝"), ui.StyleBold.Render(name))

	// Create worktree manager
	home, _ := os.UserHomeDir()
	workDir := home + "/hive-worktrees"
	worktreeMgr := worktree.NewGitManager(repoRoot, workDir)

	// Create agent client and spawner
	client := agent.NewHTTPClient()
	spawner := agent.NewProcessSpawner(worktreeMgr, client)
	spawner.SetBasePort(spawnPort)

	// Spawn the agent
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
	defer cancel()

	opts := agent.SpawnOptions{
		Name:       name,
		RepoPath:   repoRoot,
		Branch:     spawnBranch,
		BaseBranch: spawnBaseBranch,
		Specialty:  spawnSpecialty,
		Sandbox:    !spawnNoSandbox,
	}

	a, err := spawner.Spawn(ctx, opts)
	if err != nil {
		return fmt.Errorf("failed to spawn agent: %w", err)
	}

	// Save to state file
	if err := addAgentToState(a); err != nil {
		fmt.Printf("%s Failed to save agent state: %v\n", ui.StyleYellow.Render("⚠️"), err)
	}

	fmt.Println()
	fmt.Printf("%s Agent spawned successfully!\n", ui.StyleGreen.Render("✓"))
	fmt.Println()
	fmt.Printf("  %s %s\n", ui.StyleDim.Render("ID:"), a.ID)
	fmt.Printf("  %s %s\n", ui.StyleDim.Render("Name:"), a.Name)
	fmt.Printf("  %s %s\n", ui.StyleDim.Render("Branch:"), a.Branch)
	fmt.Printf("  %s %d\n", ui.StyleDim.Render("Port:"), a.Port)
	fmt.Printf("  %s %s\n", ui.StyleDim.Render("Worktree:"), a.WorktreePath)
	fmt.Println()
	fmt.Printf("Send a message: %s\n", ui.StyleCyan.Render(fmt.Sprintf("hive msg %s \"your message\"", name)))

	return nil
}

func isAgentAPIInstalled() bool {
	if _, err := exec.LookPath("agentapi"); err == nil {
		return true
	}
	home, _ := os.UserHomeDir()
	if _, err := os.Stat(filepath.Join(home, "go", "bin", "agentapi")); err == nil {
		return true
	}
	return false
}
