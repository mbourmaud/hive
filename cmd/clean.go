package cmd

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"strings"

	"github.com/mbourmaud/hive/internal/shell"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var forceClean bool

var cleanCmd = &cobra.Command{
	Use:   "clean",
	Short: "Remove all hive files from the project",
	Long:  "Remove .hive/ directory, hive.yaml, and hive entries from .gitignore",
	RunE: func(cmd *cobra.Command, args []string) error {
		// Header
		fmt.Print(ui.Header("🧹", "Cleaning hive..."))

		// Create shell runner
		runner := shell.NewRunner(DebugMode)

		// Step 1: Stop and remove Docker containers
		if err := cleanDockerContainers(runner); err != nil {
			fmt.Printf("  %s\n", ui.Warning("Docker containers: "+err.Error()))
		}

		// Step 2: Remove Docker images
		if err := cleanDockerImages(runner); err != nil {
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

func cleanDockerContainers(runner *shell.Runner) error {
	// Try docker-compose down if .hive/docker-compose.yml exists
	composeFile := ".hive/docker-compose.yml"
	if _, err := os.Stat(composeFile); err == nil {
		fmt.Printf("  %s ", ui.StyleDim.Render("🐳 Stopping containers..."))
		downCmd := exec.Command("docker", "compose", "-f", composeFile, "down", "-v", "--remove-orphans")
		if err := runner.RunQuiet(downCmd); err != nil {
			fmt.Printf("%s\n", ui.StyleYellow.Render("⚠️"))
		} else {
			fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))
		}
	}

	// Force remove any remaining hive containers
	psCmd := exec.Command("docker", "ps", "-aq", "--filter", "name=claude-")
	output, err := psCmd.Output()
	if err == nil && len(output) > 0 {
		containerIDs := strings.TrimSpace(string(output))
		if containerIDs != "" {
			fmt.Printf("  %s ", ui.StyleDim.Render("🐳 Removing remaining containers..."))
			rmCmd := exec.Command("docker", "rm", "-f")
			rmCmd.Args = append(rmCmd.Args, strings.Split(containerIDs, "\n")...)
			if err := runner.RunQuiet(rmCmd); err != nil {
				fmt.Printf("%s\n", ui.StyleYellow.Render("⚠️"))
			} else {
				fmt.Printf("%s\n", ui.StyleGreen.Render("✓"))
			}
		}
	}

	// Remove redis container if exists
	redisCmd := exec.Command("docker", "rm", "-f", "hive-redis")
	_ = runner.RunQuiet(redisCmd) // Ignore errors, container might not exist

	return nil
}

func cleanDockerImages(runner *shell.Runner) error {
	// List all hive-related images
	imagesCmd := exec.Command("docker", "images", "--filter", "reference=hive-*", "-q")
	output, err := imagesCmd.Output()
	if err != nil {
		return err
	}

	imageIDs := strings.TrimSpace(string(output))
	if imageIDs == "" {
		return nil
	}

	// Remove images
	fmt.Printf("  %s ", ui.StyleDim.Render("🗑️ Removing Docker images..."))
	rmiCmd := exec.Command("docker", "rmi", "-f")
	rmiCmd.Args = append(rmiCmd.Args, strings.Split(imageIDs, "\n")...)
	if err := runner.RunQuiet(rmiCmd); err != nil {
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

	return nil
}

func pluralize(count int) string {
	if count > 1 {
		return "s"
	}
	return ""
}

func init() {
	rootCmd.AddCommand(cleanCmd)
	cleanCmd.Flags().BoolVar(&forceClean, "force", false, "Force clean without confirmation")
}
