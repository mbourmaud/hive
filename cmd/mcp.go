package cmd

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"sort"
	"strings"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/mbourmaud/hive/internal/mcp"
	"github.com/spf13/cobra"
)

var mcpCmd = &cobra.Command{
	Use:   "mcp",
	Short: "Manage MCP (Model Context Protocol) servers",
	Long: `Configure and manage MCP servers for Hive agents.

MCPs extend Claude's capabilities with tools like Jira, GitHub, Playwright, etc.
Configuration is stored in hive.yaml, secrets in .env.project.

Examples:
  hive mcp list              # List available and configured MCPs
  hive mcp add jira          # Add Jira MCP (prompts for secrets)
  hive mcp add playwright    # Add Playwright MCP (no secrets needed)
  hive mcp remove jira       # Remove Jira MCP
  hive mcp sync              # Configure MCPs from hive.yaml`,
}

var mcpGlobal bool

var mcpAddCmd = &cobra.Command{
	Use:   "add <name> [package] [args...]",
	Short: "Add an MCP server",
	Long: `Add an MCP server to the project.

For known MCPs (jira, github, playwright, etc.), the package is auto-detected.
Secrets are prompted and stored in .env.project.

Examples:
  hive mcp add jira                    # Known MCP, auto-detects package
  hive mcp add playwright              # No secrets needed
  hive mcp add github --global         # Add to user scope (all projects)
  hive mcp add custom npx my-mcp       # Custom MCP with package`,
	Args: cobra.MinimumNArgs(1),
	RunE: runMCPAdd,
}

var mcpListCmd = &cobra.Command{
	Use:   "list",
	Short: "List MCP servers",
	Long: `List all available and configured MCP servers.

Shows:
- Known MCPs from registry (available to add)
- MCPs configured in hive.yaml
- MCPs configured via Claude CLI`,
	RunE: runMCPList,
}

var mcpRemoveCmd = &cobra.Command{
	Use:   "remove <name>",
	Short: "Remove an MCP server",
	Args:  cobra.ExactArgs(1),
	RunE:  runMCPRemove,
}

var mcpSyncCmd = &cobra.Command{
	Use:   "sync",
	Short: "Sync MCPs from hive.yaml",
	Long: `Configure all MCPs defined in hive.yaml.

Prompts for any missing secrets and configures Claude CLI.
Run this after cloning a project with MCPs defined.`,
	RunE: runMCPSync,
}

func init() {
	rootCmd.AddCommand(mcpCmd)
	mcpCmd.AddCommand(mcpAddCmd)
	mcpCmd.AddCommand(mcpListCmd)
	mcpCmd.AddCommand(mcpRemoveCmd)
	mcpCmd.AddCommand(mcpSyncCmd)

	mcpAddCmd.Flags().BoolVarP(&mcpGlobal, "global", "g", false, "Add to user scope (all projects)")
}

func runMCPAdd(cmd *cobra.Command, args []string) error {
	name := args[0]

	// Check if it's a known MCP
	info := mcp.Get(name)

	var packageName string
	var mcpArgs []string
	var envVars []string

	if info != nil {
		// Known MCP
		fmt.Printf("🔍 MCP '%s' found in registry\n", name)
		fmt.Printf("   Package: %s\n", info.Package)
		fmt.Printf("   Description: %s\n", info.Description)
		fmt.Println()

		packageName = info.Package
		envVars = info.Env

		// Prompt for required secrets
		if len(envVars) > 0 {
			fmt.Println("📋 Configuration required:")
			if err := promptAndSaveSecrets(envVars); err != nil {
				return err
			}
			fmt.Println()
		}
	} else {
		// Custom MCP
		if len(args) < 2 {
			return fmt.Errorf("unknown MCP '%s'. For custom MCPs, provide package: hive mcp add %s npx @your/mcp", name, name)
		}
		packageName = args[1]
		if len(args) > 2 {
			mcpArgs = args[2:]
		}
		fmt.Printf("📦 Adding custom MCP '%s' with package '%s'\n", name, packageName)
	}

	// Determine scope
	scope := "local"
	if mcpGlobal {
		scope = "user"
	}

	// Build claude mcp add command
	claudeArgs := []string{"mcp", "add", "--scope", scope, name}

	// For npm packages, use npx
	if strings.HasPrefix(packageName, "@") || strings.Contains(packageName, "/") {
		claudeArgs = append(claudeArgs, "npx", "-y", packageName)
	} else {
		claudeArgs = append(claudeArgs, packageName)
	}

	// Add additional args
	claudeArgs = append(claudeArgs, mcpArgs...)

	// Add env vars if any
	for _, env := range envVars {
		claudeArgs = append(claudeArgs, "-e", fmt.Sprintf("%s=${%s}", env, env))
	}

	// Run claude mcp add
	fmt.Printf("🚀 Running: claude %s\n", strings.Join(claudeArgs, " "))

	claudeCmd := exec.Command("claude", claudeArgs...)
	claudeCmd.Stdout = os.Stdout
	claudeCmd.Stderr = os.Stderr
	claudeCmd.Stdin = os.Stdin

	if err := claudeCmd.Run(); err != nil {
		return fmt.Errorf("failed to add MCP via Claude CLI: %w", err)
	}

	// Update hive.yaml
	if err := addMCPToConfig(name, packageName, envVars, mcpArgs); err != nil {
		fmt.Printf("⚠️  Warning: Could not update hive.yaml: %v\n", err)
	} else {
		fmt.Println("✓ MCP added to hive.yaml")
	}

	fmt.Println()
	fmt.Printf("✅ MCP '%s' configured successfully!\n", name)
	fmt.Println("🔄 Restart agents to apply: hive stop && hive start")

	return nil
}

func runMCPList(cmd *cobra.Command, args []string) error {
	fmt.Println("🔌 MCP Servers")
	fmt.Println("==============")
	fmt.Println()

	// Load config to show configured MCPs
	cfg := config.LoadOrDefault()

	// Show configured MCPs from hive.yaml
	if len(cfg.MCPs) > 0 {
		fmt.Println("📦 Configured in hive.yaml:")
		for name, mcpCfg := range cfg.MCPs {
			pkg := mcpCfg.Package
			if pkg == "" {
				pkg = mcpCfg.Command
			}
			status := "✓"
			if len(mcpCfg.Env) > 0 {
				// Check if secrets are configured
				missing := checkMissingSecrets(mcpCfg.Env)
				if len(missing) > 0 {
					status = fmt.Sprintf("⚠️  (missing: %s)", strings.Join(missing, ", "))
				}
			}
			fmt.Printf("   %s %s - %s\n", status, name, pkg)
		}
		fmt.Println()
	}

	// Show available MCPs from registry
	fmt.Println("📋 Available MCPs (use 'hive mcp add <name>'):")

	// Sort by name for consistent output
	names := mcp.List()
	sort.Strings(names)

	// Group by category
	categories := map[string][]string{
		"Browser":      {"playwright"},
		"Project Mgmt": {"jira", "linear", "asana"},
		"Code":         {"github", "gitlab"},
		"Docs":         {"notion", "confluence"},
		"Communication": {"slack"},
		"Cloud":        {"aws"},
		"Database":     {"postgres"},
		"AI":           {"memory", "sequential-thinking", "context7"},
		"Files":        {"filesystem", "gdrive"},
		"Monitoring":   {"sentry"},
	}

	for category, mcpNames := range categories {
		fmt.Printf("\n   %s:\n", category)
		for _, name := range mcpNames {
			info := mcp.Get(name)
			if info != nil {
				configured := ""
				if _, ok := cfg.MCPs[name]; ok {
					configured = " [configured]"
				}
				fmt.Printf("     • %s - %s%s\n", name, info.Description, configured)
			}
		}
	}

	fmt.Println()
	return nil
}

func runMCPRemove(cmd *cobra.Command, args []string) error {
	name := args[0]

	// Remove from Claude CLI
	scope := "local"
	if mcpGlobal {
		scope = "user"
	}

	claudeCmd := exec.Command("claude", "mcp", "remove", "--scope", scope, name)
	claudeCmd.Stdout = os.Stdout
	claudeCmd.Stderr = os.Stderr

	if err := claudeCmd.Run(); err != nil {
		fmt.Printf("⚠️  Warning: Could not remove from Claude CLI: %v\n", err)
	}

	// Remove from hive.yaml
	if err := removeMCPFromConfig(name); err != nil {
		fmt.Printf("⚠️  Warning: Could not update hive.yaml: %v\n", err)
	} else {
		fmt.Println("✓ MCP removed from hive.yaml")
	}

	fmt.Printf("✅ MCP '%s' removed\n", name)
	return nil
}

func runMCPSync(cmd *cobra.Command, args []string) error {
	cfg := config.LoadOrDefault()

	if len(cfg.MCPs) == 0 {
		fmt.Println("No MCPs defined in hive.yaml")
		return nil
	}

	fmt.Println("🔄 Syncing MCPs from hive.yaml...")
	fmt.Println()

	for name, mcpCfg := range cfg.MCPs {
		fmt.Printf("📦 Configuring %s...\n", name)

		// Check for missing secrets
		if len(mcpCfg.Env) > 0 {
			missing := checkMissingSecrets(mcpCfg.Env)
			if len(missing) > 0 {
				fmt.Printf("   Missing secrets: %s\n", strings.Join(missing, ", "))
				if err := promptAndSaveSecrets(missing); err != nil {
					return err
				}
			}
		}

		// Configure via Claude CLI
		pkg := mcpCfg.Package
		if pkg == "" {
			pkg = mcpCfg.Command
		}

		claudeArgs := []string{"mcp", "add", "--scope", "local", name}

		if strings.HasPrefix(pkg, "@") || strings.Contains(pkg, "/") {
			claudeArgs = append(claudeArgs, "npx", "-y", pkg)
		} else {
			claudeArgs = append(claudeArgs, pkg)
		}

		claudeArgs = append(claudeArgs, mcpCfg.Args...)

		for _, env := range mcpCfg.Env {
			claudeArgs = append(claudeArgs, "-e", fmt.Sprintf("%s=${%s}", env, env))
		}

		claudeCmd := exec.Command("claude", claudeArgs...)
		if err := claudeCmd.Run(); err != nil {
			fmt.Printf("   ⚠️  Warning: %v\n", err)
		} else {
			fmt.Printf("   ✓ Configured\n")
		}
	}

	fmt.Println()
	fmt.Println("✅ MCPs synced successfully!")
	return nil
}

// Helper functions

func promptAndSaveSecrets(envVars []string) error {
	reader := bufio.NewReader(os.Stdin)
	secrets := make(map[string]string)

	for _, env := range envVars {
		// Check if already set
		if val := os.Getenv(env); val != "" {
			fmt.Printf("   %s: [already set]\n", env)
			continue
		}

		fmt.Printf("   %s: ", env)
		value, err := reader.ReadString('\n')
		if err != nil {
			return err
		}
		value = strings.TrimSpace(value)
		if value != "" {
			secrets[env] = value
		}
	}

	// Save to .env.project
	if len(secrets) > 0 {
		if err := appendToEnvProject(secrets); err != nil {
			return err
		}
		fmt.Println("   💾 Secrets saved to .env.project")
	}

	return nil
}

func appendToEnvProject(secrets map[string]string) error {
	envFile := ".env.project"

	// Read existing content
	existing := make(map[string]bool)
	if data, err := os.ReadFile(envFile); err == nil {
		for _, line := range strings.Split(string(data), "\n") {
			if idx := strings.Index(line, "="); idx > 0 {
				existing[line[:idx]] = true
			}
		}
	}

	// Open for append
	f, err := os.OpenFile(envFile, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0600)
	if err != nil {
		return err
	}
	defer f.Close()

	for key, value := range secrets {
		if !existing[key] {
			if _, err := fmt.Fprintf(f, "%s=%s\n", key, value); err != nil {
				return err
			}
		}
	}

	return nil
}

func checkMissingSecrets(envVars []string) []string {
	var missing []string

	// Check environment
	for _, env := range envVars {
		if os.Getenv(env) != "" {
			continue
		}

		// Check .env.project
		if data, err := os.ReadFile(".env.project"); err == nil {
			if strings.Contains(string(data), env+"=") {
				continue
			}
		}

		// Check .env
		if data, err := os.ReadFile(".env"); err == nil {
			if strings.Contains(string(data), env+"=") {
				continue
			}
		}

		missing = append(missing, env)
	}

	return missing
}

func addMCPToConfig(name, pkg string, env, args []string) error {
	configPath := "hive.yaml"

	cfg, err := config.Load(configPath)
	if err != nil {
		cfg = config.Default()
	}

	if cfg.MCPs == nil {
		cfg.MCPs = make(map[string]config.MCPConfig)
	}

	cfg.MCPs[name] = config.MCPConfig{
		Package: pkg,
		Env:     env,
		Args:    args,
	}

	return cfg.Save(configPath)
}

func removeMCPFromConfig(name string) error {
	configPath := "hive.yaml"

	cfg, err := config.Load(configPath)
	if err != nil {
		return err
	}

	if cfg.MCPs == nil {
		return nil
	}

	delete(cfg.MCPs, name)
	return cfg.Save(configPath)
}

// SyncMCPsFromConfig is exported for use in init command
func SyncMCPsFromConfig(cfg *config.Config) error {
	if len(cfg.MCPs) == 0 {
		return nil
	}

	fmt.Println()
	fmt.Println("🔌 Configuring MCPs from hive.yaml...")

	for name, mcpCfg := range cfg.MCPs {
		// Check for missing secrets
		if len(mcpCfg.Env) > 0 {
			missing := checkMissingSecrets(mcpCfg.Env)
			if len(missing) > 0 {
				fmt.Printf("\n📦 MCP '%s' requires configuration:\n", name)
				if err := promptAndSaveSecrets(missing); err != nil {
					return err
				}
			}
		}

		// Configure via Claude CLI
		pkg := mcpCfg.Package
		if pkg == "" {
			pkg = mcpCfg.Command
		}

		claudeArgs := []string{"mcp", "add", "--scope", "local", name}

		if strings.HasPrefix(pkg, "@") || strings.Contains(pkg, "/") {
			claudeArgs = append(claudeArgs, "npx", "-y", pkg)
		} else {
			claudeArgs = append(claudeArgs, pkg)
		}

		claudeArgs = append(claudeArgs, mcpCfg.Args...)

		for _, env := range mcpCfg.Env {
			claudeArgs = append(claudeArgs, "-e", fmt.Sprintf("%s=${%s}", env, env))
		}

		claudeCmd := exec.Command("claude", claudeArgs...)
		if err := claudeCmd.Run(); err != nil {
			fmt.Printf("   ⚠️  %s: %v\n", name, err)
		} else {
			fmt.Printf("   ✓ %s configured\n", name)
		}
	}

	return nil
}
