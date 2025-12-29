package cmd

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"strings"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/mbourmaud/hive/internal/shell"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var forceClean bool

// commandExecutor is the interface for executing shell commands (allows mocking in tests)
var commandExecutor shell.CommandExecutor

var cleanCmd = &cobra.Command{
	Use:   "clean",
	Short: "Remove all hive files from the project",
	Long:  "Remove .hive/ directory, hive.yaml, and hive entries from .gitignore",
	RunE: func(cmd *cobra.Command, args []string) error {
		// Header
		fmt.Print(ui.Header("🧹", "Cleaning hive..."))

		// Create shell runner and executor
		runner := shell.NewRunner(DebugMode)
		if commandExecutor == nil {
			commandExecutor = shell.NewRealExecutor(DebugMode)
		}

		// Step 1: Stop and remove Docker containers
		if err := cleanDockerContainers(commandExecutor); err != nil {
			fmt.Printf("  %s\n", ui.Warning("Docker containers: "+err.Error()))
		}

		// Step 2: Remove Docker images
		if err := cleanDockerImages(commandExecutor); err != nil {
			fmt.Printf("  %s\n", ui.Warning("Docker images: "+err.Error()))
		}

		// Step 3: Remove git worktrees
		if err := cleanWorktrees(runner); err != nil {
			fmt.Printf("  %s\n", ui.Warning("Git worktrees: "+err.Error()))
		}

		// Step 4: Remove .hive directory
		if _, err := os.Stat(".hive"); err == nil {
			if err := os.RemoveAll(".hive"); err != nil {
				return fmt.Errorf("failed to remove .hive/: %w", err)
			}
			fmt.Print(ui.ProgressLine("Removed .hive/", "✓"))
		}

		// Step 5: Remove hive.yaml
		if _, err := os.Stat("hive.yaml"); err == nil {
			if err := os.Remove("hive.yaml"); err != nil {
				return fmt.Errorf("failed to remove hive.yaml: %w", err)
			}
			fmt.Print(ui.ProgressLine("Removed hive.yaml", "✓"))
		}

		// Step 6: Clean .gitignore
		if err := cleanGitignore(); err != nil {
			fmt.Printf("  %s\n", ui.Warning(".gitignore: "+err.Error()))
		}

		fmt.Printf("\n%s\n\n", ui.Success("Hive cleaned successfully!"))
		return nil
	},
}

func cleanGitignore() error {
	data, err := os.ReadFile(".gitignore")
	if err != nil {
		if os.IsNotExist(err) {
			return nil // No .gitignore, nothing to clean
		}
		return err
	}

	var newLines []string
	scanner := bufio.NewScanner(strings.NewReader(string(data)))
	inHiveSection := false
	removed := false

	for scanner.Scan() {
		line := scanner.Text()

		// Detect hive section start
		if strings.Contains(line, "Hive") && strings.HasPrefix(line, "#") {
			inHiveSection = true
			removed = true
			continue
		}

		// Skip hive-related entries
		if inHiveSection {
			trimmed := strings.TrimSpace(line)
			if trimmed == "" {
				inHiveSection = false
				continue
			}
			if trimmed == ".hive/" || trimmed == ".hive" {
				continue
			}
			// Non-empty line that's not hive-related, end of section
			inHiveSection = false
		}

		newLines = append(newLines, line)
	}

	if removed {
		// Write back cleaned .gitignore
		content := strings.Join(newLines, "\n")
		if !strings.HasSuffix(content, "\n") && len(content) > 0 {
			content += "\n"
		}
		if err := os.WriteFile(".gitignore", []byte(content), 0644); err != nil {
			return err
		}
		fmt.Print(ui.ProgressLine("Cleaned .gitignore", "✓"))
	}

	return nil
}

func cleanDockerContainers(executor shell.CommandExecutor) error {
	// Get container prefix for this project
	cfg := config.LoadOrDefault()
	prefix := cfg.GetContainerPrefix()

	// Try docker-compose down if .hive/docker-compose.yml exists
	composeFile := ".hive/docker-compose.yml"
	if _, err := os.Stat(composeFile); err == nil {
		fmt.Printf("  %s ", ui.StyleDim.Render("🐳 Stopping containers..."))
		if err := executor.RunQuietCommand("docker", "compose", "-f", composeFile, "down", "-v", "--remove-orphans"); err != nil {
			fmt.Printf("%s\n", ui.StyleYellow.Render("⚠️"))
		} else {
			fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))
		}
	}

	// Force remove any remaining containers for this project
	output, _, err := executor.RunCommand("docker", "ps", "-aq", "--filter", "name="+prefix+"-")
	if err == nil && len(output) > 0 {
		containerIDs := strings.TrimSpace(output)
		if containerIDs != "" {
			fmt.Printf("  %s ", ui.StyleDim.Render("🐳 Removing remaining containers..."))
			args := append([]string{"rm", "-f"}, strings.Split(containerIDs, "\n")...)
			if err := executor.RunQuietCommand("docker", args...); err != nil {
				fmt.Printf("%s\n", ui.StyleYellow.Render("⚠️"))
			} else {
				fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))
			}
		}
	}

	// Remove redis container if exists
	_ = executor.RunQuietCommand("docker", "rm", "-f", prefix+"-redis") // Ignore errors, container might not exist

	return nil
}

func cleanDockerImages(executor shell.CommandExecutor) error {
	// Get container prefix for this project
	cfg := config.LoadOrDefault()
	prefix := cfg.GetContainerPrefix()

	// List project-related images (e.g., hive-queen, hive-drone-1, etc.)
	output, _, err := executor.RunCommand("docker", "images", "--filter", "reference="+prefix+"-*", "-q")
	if err != nil {
		return err
	}

	imageIDs := strings.TrimSpace(output)
	if imageIDs == "" {
		return nil
	}

	// Remove images
	fmt.Printf("  %s ", ui.StyleDim.Render("🗑️ Removing Docker images..."))
	args := append([]string{"rmi", "-f"}, strings.Split(imageIDs, "\n")...)
	if err := executor.RunQuietCommand("docker", args...); err != nil {
		fmt.Printf("%s\n", ui.StyleYellow.Render("⚠️"))
	} else {
		fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))
	}

	return nil
}

// cleanWorktrees removes all hive worktrees and prunes orphaned entries
func cleanWorktrees(runner *shell.Runner) error {
	// Check if we're in a git repository
	gitCmd := exec.Command("git", "rev-parse", "--is-inside-work-tree")
	if err := runner.RunQuiet(gitCmd); err != nil {
		// Not a git repo, skip worktree cleanup
		return nil
	}

	// Prune orphaned worktrees first (clean up git metadata)
	fmt.Printf("  %s ", ui.StyleDim.Render("🌲 Pruning orphaned worktrees..."))
	pruneCmd := exec.Command("git", "worktree", "prune")
	if err := runner.RunQuiet(pruneCmd); err != nil {
		fmt.Printf("%s\n", ui.StyleYellow.Render("⚠️"))
	} else {
		fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))
	}

	// Check if worktrees directory exists
	worktreesDir := ".hive/workspaces"
	if _, err := os.Stat(worktreesDir); os.IsNotExist(err) {
		return nil
	}

	// List all worktrees in the directory
	entries, err := os.ReadDir(worktreesDir)
	if err != nil {
		return err
	}

	fmt.Printf("  %s ", ui.StyleDim.Render("🌳 Removing git worktrees..."))
	removed := 0
	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}

		worktreePath := fmt.Sprintf("%s/%s", worktreesDir, entry.Name())

		// Remove worktree using git worktree remove
		removeCmd := exec.Command("git", "worktree", "remove", worktreePath, "--force")
		if err := runner.RunQuiet(removeCmd); err == nil {
			removed++
		}
	}

	if removed > 0 {
		fmt.Printf("%s (%d worktree%s)\n", ui.StyleGreen.Render("✓"), removed, pluralize(removed))
	} else {
		fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))
	}

	// Delete hive branches (hive/queen, hive/drone-*)
	fmt.Printf("  %s ", ui.StyleDim.Render("🌿 Deleting hive branches..."))
	branchesDeleted := 0

	// Get list of hive branches
	listCmd := exec.Command("git", "branch", "--list", "hive/*")
	output, err := listCmd.Output()
	if err == nil && len(output) > 0 {
		branches := strings.Split(strings.TrimSpace(string(output)), "\n")
		for _, branch := range branches {
			branch = strings.TrimSpace(branch)
			branch = strings.TrimPrefix(branch, "* ") // Remove current branch marker
			if branch == "" {
				continue
			}

			// Delete the branch
			deleteCmd := exec.Command("git", "branch", "-D", branch)
			if err := runner.RunQuiet(deleteCmd); err == nil {
				branchesDeleted++
			}
		}
	}

	if branchesDeleted > 0 {
		fmt.Printf("%s (%d branch%s)\n", ui.StyleGreen.Render("✓"), branchesDeleted, pluralizeEs(branchesDeleted))
	} else {
		fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))
	}

	return nil
}

func pluralize(count int) string {
	if count > 1 {
		return "s"
	}
	return ""
}

func pluralizeEs(count int) string {
	if count > 1 {
		return "es"
	}
	return ""
}

func init() {
	rootCmd.AddCommand(cleanCmd)
	cleanCmd.Flags().BoolVar(&forceClean, "force", false, "Force clean without confirmation")
}
