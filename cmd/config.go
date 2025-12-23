package cmd

import (
	"fmt"
	"os"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/spf13/cobra"
	"gopkg.in/yaml.v3"
)

var configCmd = &cobra.Command{
	Use:   "config",
	Short: "View or manage configuration",
	Long: `View and manage Hive configuration.

Examples:
  hive config show        # Display current configuration
  hive config validate    # Validate configuration files
  hive config path        # Show config file paths`,
}

var configShowCmd = &cobra.Command{
	Use:   "show",
	Short: "Display current configuration",
	RunE: func(cmd *cobra.Command, args []string) error {
		cfg := config.LoadOrDefault()

		fmt.Println("Current Hive Configuration:")
		fmt.Println("===========================")
		fmt.Println()

		// Marshal to YAML for display
		data, err := yaml.Marshal(cfg)
		if err != nil {
			return fmt.Errorf("failed to format config: %w", err)
		}

		fmt.Println(string(data))

		// Show config source
		if _, err := os.Stat("hive.yaml"); err == nil {
			fmt.Println("Source: hive.yaml")
		} else {
			fmt.Println("Source: defaults (no hive.yaml found)")
		}

		return nil
	},
}

var configValidateCmd = &cobra.Command{
	Use:   "validate",
	Short: "Validate configuration files",
	RunE: func(cmd *cobra.Command, args []string) error {
		fmt.Println("Validating configuration...")
		fmt.Println()

		hasErrors := false

		// Check hive.yaml
		if _, err := os.Stat("hive.yaml"); err == nil {
			cfg, err := config.Load("hive.yaml")
			if err != nil {
				fmt.Printf("  hive.yaml: INVALID - %s\n", err)
				hasErrors = true
			} else if err := cfg.Validate(); err != nil {
				fmt.Printf("  hive.yaml: INVALID - %s\n", err)
				hasErrors = true
			} else {
				fmt.Println("  hive.yaml: OK")
			}
		} else {
			fmt.Println("  hive.yaml: not found (using defaults)")
		}

		// Check .env
		if _, err := os.Stat(".env"); err == nil {
			fmt.Println("  .env: OK")
		} else {
			fmt.Println("  .env: not found (required for secrets)")
			hasErrors = true
		}

		// Check docker-compose.yml
		if _, err := os.Stat("docker-compose.yml"); err == nil {
			fmt.Println("  docker-compose.yml: OK")
		} else {
			fmt.Println("  docker-compose.yml: not found")
			hasErrors = true
		}

		fmt.Println()

		if hasErrors {
			return fmt.Errorf("configuration validation failed")
		}

		fmt.Println("All configuration files are valid!")
		return nil
	},
}

var configPathCmd = &cobra.Command{
	Use:   "path",
	Short: "Show configuration file paths",
	Run: func(cmd *cobra.Command, args []string) {
		cwd, _ := os.Getwd()
		fmt.Println("Configuration file paths:")
		fmt.Println()
		fmt.Printf("  Working directory: %s\n", cwd)
		fmt.Printf("  hive.yaml:         %s/hive.yaml\n", cwd)
		fmt.Printf("  .env:              %s/.env\n", cwd)
		fmt.Printf("  docker-compose:    %s/docker-compose.yml\n", cwd)
		fmt.Println()
	},
}

func init() {
	rootCmd.AddCommand(configCmd)
	configCmd.AddCommand(configShowCmd)
	configCmd.AddCommand(configValidateCmd)
	configCmd.AddCommand(configPathCmd)
}
