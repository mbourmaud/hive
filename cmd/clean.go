package cmd

import (
	"bufio"
	"fmt"
	"os"
	"strings"

	"github.com/spf13/cobra"
)

var cleanCmd = &cobra.Command{
	Use:   "clean",
	Short: "Remove all hive files from the project",
	Long:  "Remove .hive/ directory, hive.yaml, and hive entries from .gitignore",
	RunE: func(cmd *cobra.Command, args []string) error {
		fmt.Println("🧹 Cleaning hive files...")

		// Remove .hive directory
		if _, err := os.Stat(".hive"); err == nil {
			if err := os.RemoveAll(".hive"); err != nil {
				return fmt.Errorf("failed to remove .hive/: %w", err)
			}
			fmt.Println("  ✓ Removed .hive/")
		}

		// Remove hive.yaml
		if _, err := os.Stat("hive.yaml"); err == nil {
			if err := os.Remove("hive.yaml"); err != nil {
				return fmt.Errorf("failed to remove hive.yaml: %w", err)
			}
			fmt.Println("  ✓ Removed hive.yaml")
		}

		// Clean .gitignore
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

func init() {
	rootCmd.AddCommand(cleanCmd)
}
