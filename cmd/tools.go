package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/spf13/cobra"
)

// ToolInfo represents a CLI tool in the registry
type ToolInfo struct {
	Name        string `json:"name"`
	Description string `json:"description"`
	Category    string `json:"category"`
	Install     string `json:"install"`
	Verify      string `json:"verify"`
}

// ToolsRegistry represents the tools-registry.json structure
type ToolsRegistry struct {
	Version    string              `json:"version"`
	Updated    string              `json:"updated"`
	Tools      map[string]ToolInfo `json:"tools"`
	Categories map[string]string   `json:"categories"`
}

var toolsRegistry *ToolsRegistry

var toolsCmd = &cobra.Command{
	Use:   "tools",
	Short: "Manage CLI tools in Hive containers",
	Long: `Configure CLI tools to install in Hive agent containers.

Tools are installed at container startup and cached for fast subsequent starts.
Configuration is stored in hive.yaml under the 'tools' section.

Examples:
  hive tools list              # List available and configured tools
  hive tools add psql          # Add PostgreSQL client
  hive tools add kubectl       # Add Kubernetes CLI
  hive tools remove psql       # Remove a tool
  hive tools reinstall         # Force reinstall all tools`,
}

var toolsAddCmd = &cobra.Command{
	Use:   "add <name>",
	Short: "Add a CLI tool",
	Long: `Add a CLI tool to be installed in Hive containers.

The tool will be installed when containers start.
Use 'hive tools list' to see available tools.

Examples:
  hive tools add psql          # PostgreSQL client
  hive tools add kubectl       # Kubernetes CLI
  hive tools add mongosh       # MongoDB shell`,
	Args: cobra.ExactArgs(1),
	RunE: runToolsAdd,
}

var toolsListCmd = &cobra.Command{
	Use:   "list",
	Short: "List CLI tools",
	Long: `List all available and configured CLI tools.

Shows:
- Tools configured in hive.yaml
- Available tools from registry`,
	RunE: runToolsList,
}

var toolsRemoveCmd = &cobra.Command{
	Use:   "remove <name>",
	Short: "Remove a CLI tool",
	Args:  cobra.ExactArgs(1),
	RunE:  runToolsRemove,
}

var toolsReinstallCmd = &cobra.Command{
	Use:   "reinstall",
	Short: "Force reinstall all tools",
	Long: `Clear the tools cache and force reinstallation.

This will remove the cached installation state, causing all
tools to be reinstalled on next container start.`,
	RunE: runToolsReinstall,
}

func init() {
	rootCmd.AddCommand(toolsCmd)
	toolsCmd.AddCommand(toolsAddCmd)
	toolsCmd.AddCommand(toolsListCmd)
	toolsCmd.AddCommand(toolsRemoveCmd)
	toolsCmd.AddCommand(toolsReinstallCmd)
}

func loadToolsRegistry() error {
	if toolsRegistry != nil {
		return nil
	}

	// Try to load from project root first
	registryPath := "tools-registry.json"
	if _, err := os.Stat(registryPath); os.IsNotExist(err) {
		// Try executable directory
		execPath, err := os.Executable()
		if err == nil {
			registryPath = filepath.Join(filepath.Dir(execPath), "tools-registry.json")
		}
	}

	data, err := os.ReadFile(registryPath)
	if err != nil {
		// Return embedded minimal registry
		toolsRegistry = &ToolsRegistry{
			Version: "1.0.0",
			Tools: map[string]ToolInfo{
				"glab":      {Name: "GitLab CLI", Description: "Command-line interface for GitLab", Category: "vcs"},
				"psql":      {Name: "PostgreSQL Client", Description: "Command-line interface for PostgreSQL", Category: "database"},
				"mongosh":   {Name: "MongoDB Shell", Description: "Modern MongoDB shell", Category: "database"},
				"mysql":     {Name: "MySQL Client", Description: "Command-line interface for MySQL", Category: "database"},
				"kubectl":   {Name: "Kubernetes CLI", Description: "Command-line tool for Kubernetes", Category: "cloud"},
				"helm":      {Name: "Helm", Description: "Kubernetes package manager", Category: "cloud"},
				"terraform": {Name: "Terraform", Description: "Infrastructure as Code tool", Category: "cloud"},
				"aws":       {Name: "AWS CLI", Description: "Amazon Web Services command-line interface", Category: "cloud"},
				"heroku":    {Name: "Heroku CLI", Description: "Command-line interface for Heroku", Category: "cloud"},
				"vercel":    {Name: "Vercel CLI", Description: "Command-line interface for Vercel", Category: "cloud"},
				"netlify":   {Name: "Netlify CLI", Description: "Command-line interface for Netlify", Category: "cloud"},
				"flyctl":    {Name: "Fly.io CLI", Description: "Command-line interface for Fly.io", Category: "cloud"},
			},
			Categories: map[string]string{
				"vcs":      "Version Control",
				"database": "Databases",
				"cloud":    "Cloud & Infrastructure",
			},
		}
		return nil
	}

	toolsRegistry = &ToolsRegistry{}
	if err := json.Unmarshal(data, toolsRegistry); err != nil {
		return fmt.Errorf("failed to parse tools registry: %w", err)
	}

	return nil
}

func runToolsAdd(cmd *cobra.Command, args []string) error {
	name := args[0]

	if err := loadToolsRegistry(); err != nil {
		return err
	}

	// Check if it's a known tool
	info, ok := toolsRegistry.Tools[name]
	if !ok {
		fmt.Printf("⚠️  Tool '%s' not found in registry\n", name)
		fmt.Println("   It will be added but may not install correctly.")
		fmt.Println("   Use 'hive tools list' to see available tools.")
		fmt.Println()
	} else {
		fmt.Printf("🔧 Tool '%s' found in registry\n", name)
		fmt.Printf("   Name: %s\n", info.Name)
		fmt.Printf("   Description: %s\n", info.Description)
		fmt.Println()
	}

	// Add to hive.yaml
	if err := addToolToConfig(name); err != nil {
		return err
	}

	fmt.Printf("✅ Tool '%s' added to hive.yaml\n", name)
	fmt.Println()
	fmt.Println("🔄 The tool will be installed when containers start.")
	fmt.Println("   Run 'hive stop && hive start' to apply now.")

	return nil
}

func runToolsList(cmd *cobra.Command, args []string) error {
	if err := loadToolsRegistry(); err != nil {
		return err
	}

	fmt.Println("🔧 CLI Tools")
	fmt.Println("============")
	fmt.Println()

	// Load config to show configured tools
	cfg := config.LoadOrDefault()

	// Show configured tools from hive.yaml
	if len(cfg.Tools) > 0 {
		fmt.Println("📦 Configured in hive.yaml:")
		for _, name := range cfg.Tools {
			info, ok := toolsRegistry.Tools[name]
			if ok {
				fmt.Printf("   ✓ %s - %s\n", name, info.Description)
			} else {
				fmt.Printf("   ✓ %s (custom)\n", name)
			}
		}
		fmt.Println()
	}

	// Show available tools from registry grouped by category
	fmt.Println("📋 Available tools (use 'hive tools add <name>'):")

	// Group tools by category
	byCategory := make(map[string][]string)
	for name := range toolsRegistry.Tools {
		info := toolsRegistry.Tools[name]
		byCategory[info.Category] = append(byCategory[info.Category], name)
	}

	// Sort categories
	categories := make([]string, 0, len(byCategory))
	for cat := range byCategory {
		categories = append(categories, cat)
	}
	sort.Strings(categories)

	// Track configured tools for marking
	configuredTools := make(map[string]bool)
	for _, t := range cfg.Tools {
		configuredTools[t] = true
	}

	for _, cat := range categories {
		catName := toolsRegistry.Categories[cat]
		if catName == "" {
			catName = cat
		}
		fmt.Printf("\n   %s:\n", catName)

		tools := byCategory[cat]
		sort.Strings(tools)

		for _, name := range tools {
			info := toolsRegistry.Tools[name]
			configured := ""
			if configuredTools[name] {
				configured = " [configured]"
			}
			fmt.Printf("     • %s - %s%s\n", name, info.Description, configured)
		}
	}

	fmt.Println()
	return nil
}

func runToolsRemove(cmd *cobra.Command, args []string) error {
	name := args[0]

	if err := removeToolFromConfig(name); err != nil {
		return err
	}

	fmt.Printf("✅ Tool '%s' removed from hive.yaml\n", name)
	fmt.Println()
	fmt.Println("💡 The tool will remain installed until containers are recreated.")
	fmt.Println("   Run 'hive clean && hive init' to fully remove.")

	return nil
}

func runToolsReinstall(cmd *cobra.Command, args []string) error {
	fmt.Println("🔄 Clearing tools cache in running containers...")

	// Find running containers and clear their tools cache
	containers := []string{"hive-queen", "hive-drone-1", "hive-drone-2", "hive-drone-3",
		"hive-drone-4", "hive-drone-5", "hive-drone-6", "hive-drone-7",
		"hive-drone-8", "hive-drone-9", "hive-drone-10"}

	clearedCount := 0
	for _, container := range containers {
		// Check if container is running
		output, err := runDockerCommand("inspect", "-f", "{{.State.Running}}", container)
		if err != nil || strings.TrimSpace(output) != "true" {
			continue
		}

		// Clear tools cache
		_, err = runDockerCommand("exec", container, "rm", "-rf", "/home/agent/.tools-cache/installed.txt")
		if err == nil {
			fmt.Printf("   ✓ %s cache cleared\n", container)
			clearedCount++
		}
	}

	if clearedCount == 0 {
		fmt.Println("   No running containers found")
		fmt.Println()
		fmt.Println("💡 Tools will be reinstalled on next container start.")
	} else {
		fmt.Println()
		fmt.Printf("✅ Tools cache cleared in %d container(s)\n", clearedCount)
		fmt.Println("💡 Restart containers to reinstall tools: hive stop && hive start")
	}

	return nil
}

func addToolToConfig(name string) error {
	configPath := "hive.yaml"

	cfg, err := config.Load(configPath)
	if err != nil {
		cfg = config.Default()
	}

	// Check if already configured
	for _, t := range cfg.Tools {
		if t == name {
			return fmt.Errorf("tool '%s' is already configured", name)
		}
	}

	cfg.Tools = append(cfg.Tools, name)
	return cfg.Save(configPath)
}

func removeToolFromConfig(name string) error {
	configPath := "hive.yaml"

	cfg, err := config.Load(configPath)
	if err != nil {
		return fmt.Errorf("hive.yaml not found: %w", err)
	}

	// Find and remove the tool
	found := false
	newTools := make([]string, 0, len(cfg.Tools))
	for _, t := range cfg.Tools {
		if t == name {
			found = true
		} else {
			newTools = append(newTools, t)
		}
	}

	if !found {
		return fmt.Errorf("tool '%s' is not configured", name)
	}

	cfg.Tools = newTools
	return cfg.Save(configPath)
}

// dockerCommandRunner allows dependency injection for testing
var dockerCommandRunner = defaultDockerCommand

func defaultDockerCommand(args ...string) (string, error) {
	cmd := exec.Command("docker", args...)
	output, err := cmd.Output()
	return string(output), err
}

func runDockerCommand(args ...string) (string, error) {
	return dockerCommandRunner(args...)
}
