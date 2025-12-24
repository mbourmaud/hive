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
		fmt.Println("🧹 Cleaning hive (containers, images, and files)...")

		// Step 1: Stop and remove Docker containers
		if err := cleanDockerContainers(); err != nil {
			fmt.Printf("  ⚠ Could not clean Docker containers: %v\n", err)
		}

		// Step 2: Remove Docker images
		if err := cleanDockerImages(); err != nil {
			fmt.Printf("  ⚠ Could not clean Docker images: %v\n", err)
		}

		// Step 3: Remove .hive directory
		if _, err := os.Stat(".hive"); err == nil {
			if err := os.RemoveAll(".hive"); err != nil {
				return fmt.Errorf("failed to remove .hive/: %w", err)
			}
			fmt.Println("  ✓ Removed .hive/")
		}

		// Step 4: Remove hive.yaml
		if _, err := os.Stat("hive.yaml"); err == nil {
			if err := os.Remove("hive.yaml"); err != nil {
				return fmt.Errorf("failed to remove hive.yaml: %w", err)
			}
			fmt.Println("  ✓ Removed hive.yaml")
		}

		// Step 5: Clean .gitignore
		if err := cleanGitignore(); err != nil {
			fmt.Printf("  ⚠ Could not clean .gitignore: %v\n", err)
		}

		fmt.Println("\n✅ Hive cleaned from project")
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
		fmt.Println("  ✓ Cleaned .gitignore")
	}

	return nil
}

func cleanDockerContainers() error {
	// Try docker-compose down if .hive/docker-compose.yml exists
	composeFile := ".hive/docker-compose.yml"
	if _, err := os.Stat(composeFile); err == nil {
		fmt.Println("  🐳 Stopping Docker containers...")
		downCmd := exec.Command("docker", "compose", "-f", composeFile, "down", "-v", "--remove-orphans")
		downCmd.Stdout = os.Stdout
		downCmd.Stderr = os.Stderr
		if err := downCmd.Run(); err != nil {
			fmt.Printf("  ⚠ docker compose down failed: %v\n", err)
		} else {
			fmt.Println("  ✓ Stopped and removed containers")
		}
	}

	// Force remove any remaining hive containers
	fmt.Println("  🐳 Removing remaining hive containers...")
	psCmd := exec.Command("docker", "ps", "-aq", "--filter", "name=claude-")
	output, err := psCmd.Output()
	if err == nil && len(output) > 0 {
		containerIDs := strings.TrimSpace(string(output))
		if containerIDs != "" {
			rmCmd := exec.Command("docker", "rm", "-f")
			rmCmd.Args = append(rmCmd.Args, strings.Split(containerIDs, "\n")...)
			if err := rmCmd.Run(); err != nil {
				fmt.Printf("  ⚠ Could not force remove some containers: %v\n", err)
			} else {
				fmt.Println("  ✓ Removed remaining containers")
			}
		}
	}

	// Remove redis container if exists
	redisCmd := exec.Command("docker", "rm", "-f", "hive-redis")
	_ = redisCmd.Run() // Ignore errors, container might not exist

	return nil
}

func cleanDockerImages() error {
	fmt.Println("  🐳 Removing hive Docker images...")

	// List all hive-related images
	imagesCmd := exec.Command("docker", "images", "--filter", "reference=hive-*", "-q")
	output, err := imagesCmd.Output()
	if err != nil {
		return fmt.Errorf("failed to list images: %w", err)
	}

	imageIDs := strings.TrimSpace(string(output))
	if imageIDs == "" {
		fmt.Println("  ✓ No hive images to remove")
		return nil
	}

	// Remove images
	rmiCmd := exec.Command("docker", "rmi", "-f")
	rmiCmd.Args = append(rmiCmd.Args, strings.Split(imageIDs, "\n")...)
	rmiCmd.Stdout = os.Stdout
	rmiCmd.Stderr = os.Stderr
	if err := rmiCmd.Run(); err != nil {
		return fmt.Errorf("failed to remove images: %w", err)
	}

	fmt.Println("  ✓ Removed hive images")
	return nil
}

func init() {
	rootCmd.AddCommand(cleanCmd)
}
