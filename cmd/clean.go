package cmd

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"strings"

	"github.com/spf13/cobra"
)

var cleanCmd = &cobra.Command{
	Use:   "clean",
	Short: "Remove all hive files from the project",
	Long:  "Remove .hive/ directory, hive.yaml, and hive entries from .gitignore",
	RunE: func(cmd *cobra.Command, args []string) error {
		fmt.Printf("\n%s%s🧹 Cleaning hive...%s\n\n", colorBold, colorCyan, colorReset)

		// Step 1: Stop and remove Docker containers
		if err := cleanDockerContainers(); err != nil {
			fmt.Printf("  %s⚠️  Docker containers: %v%s\n", colorYellow, err, colorReset)
		}

		// Step 2: Remove Docker images
		if err := cleanDockerImages(); err != nil {
			fmt.Printf("  %s⚠️  Docker images: %v%s\n", colorYellow, err, colorReset)
		}

		// Step 3: Remove git worktrees
		if err := cleanWorktrees(); err != nil {
			fmt.Printf("  %s⚠️  Git worktrees: %v%s\n", colorYellow, err, colorReset)
		}

		// Step 4: Remove .hive directory
		if _, err := os.Stat(".hive"); err == nil {
			if err := os.RemoveAll(".hive"); err != nil {
				return fmt.Errorf("failed to remove .hive/: %w", err)
			}
			fmt.Printf("  %s✓ Removed .hive/%s\n", colorGreen, colorReset)
		}

		// Step 5: Remove hive.yaml
		if _, err := os.Stat("hive.yaml"); err == nil {
			if err := os.Remove("hive.yaml"); err != nil {
				return fmt.Errorf("failed to remove hive.yaml: %w", err)
			}
			fmt.Printf("  %s✓ Removed hive.yaml%s\n", colorGreen, colorReset)
		}

		// Step 6: Clean .gitignore
		if err := cleanGitignore(); err != nil {
			fmt.Printf("  %s⚠️  .gitignore: %v%s\n", colorYellow, err, colorReset)
		}

		fmt.Printf("\n%s%s✨ Hive cleaned successfully!%s\n\n", colorBold, colorGreen, colorReset)
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
		fmt.Printf("  %s✓ Cleaned .gitignore%s\n", colorGreen, colorReset)
	}

	return nil
}

func cleanDockerContainers() error {
	// Try docker-compose down if .hive/docker-compose.yml exists
	composeFile := ".hive/docker-compose.yml"
	if _, err := os.Stat(composeFile); err == nil {
		fmt.Printf("  %s🐳 Stopping containers...%s", colorCyan, colorReset)
		downCmd := exec.Command("docker", "compose", "-f", composeFile, "down", "-v", "--remove-orphans")
		downCmd.Stdout = nil
		downCmd.Stderr = nil
		if err := downCmd.Run(); err != nil {
			fmt.Printf(" %s⚠️%s\n", colorYellow, colorReset)
		} else {
			fmt.Printf(" %s✓%s\n", colorGreen, colorReset)
		}
	}

	// Force remove any remaining hive containers
	psCmd := exec.Command("docker", "ps", "-aq", "--filter", "name=claude-")
	output, err := psCmd.Output()
	if err == nil && len(output) > 0 {
		containerIDs := strings.TrimSpace(string(output))
		if containerIDs != "" {
			fmt.Printf("  %s🐳 Removing remaining containers...%s", colorCyan, colorReset)
			rmCmd := exec.Command("docker", "rm", "-f")
			rmCmd.Args = append(rmCmd.Args, strings.Split(containerIDs, "\n")...)
			rmCmd.Stdout = nil
			rmCmd.Stderr = nil
			if err := rmCmd.Run(); err != nil {
				fmt.Printf(" %s⚠️%s\n", colorYellow, colorReset)
			} else {
				fmt.Printf(" %s✓%s\n", colorGreen, colorReset)
			}
		}
	}

	// Remove redis container if exists
	redisCmd := exec.Command("docker", "rm", "-f", "hive-redis")
	redisCmd.Stdout = nil
	redisCmd.Stderr = nil
	_ = redisCmd.Run() // Ignore errors, container might not exist

	return nil
}

func cleanDockerImages() error {
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
	fmt.Printf("  %s🗑️  Removing Docker images...%s", colorCyan, colorReset)
	rmiCmd := exec.Command("docker", "rmi", "-f")
	rmiCmd.Args = append(rmiCmd.Args, strings.Split(imageIDs, "\n")...)
	rmiCmd.Stdout = nil
	rmiCmd.Stderr = nil
	if err := rmiCmd.Run(); err != nil {
		fmt.Printf(" %s⚠️%s\n", colorYellow, colorReset)
	} else {
		fmt.Printf(" %s✓%s\n", colorGreen, colorReset)
	}

	return nil
}

func cleanWorktrees() error {
	// Check if we're in a git repository
	gitCmd := exec.Command("git", "rev-parse", "--is-inside-work-tree")
	gitCmd.Stdout = nil
	gitCmd.Stderr = nil
	if err := gitCmd.Run(); err != nil {
		// Not a git repo, skip worktree cleanup
		return nil
	}

	// Prune orphaned worktrees first
	pruneCmd := exec.Command("git", "worktree", "prune")
	pruneCmd.Stdout = nil
	pruneCmd.Stderr = nil
	_ = pruneCmd.Run() // Silent prune

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

	fmt.Printf("  %s🌳 Removing git worktrees...%s", colorCyan, colorReset)
	removed := 0
	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}

		worktreePath := fmt.Sprintf("%s/%s", worktreesDir, entry.Name())

		// Remove worktree using git worktree remove
		removeCmd := exec.Command("git", "worktree", "remove", worktreePath, "--force")
		removeCmd.Stdout = nil
		removeCmd.Stderr = nil
		if err := removeCmd.Run(); err == nil {
			removed++
		}
	}

	if removed > 0 {
		fmt.Printf(" %s✓%s (%d worktree%s)\n", colorGreen, colorReset, removed, pluralize(removed))
	} else {
		fmt.Printf(" %s✓%s\n", colorGreen, colorReset)
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
}
