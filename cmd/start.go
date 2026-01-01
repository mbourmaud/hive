package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"time"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/mbourmaud/hive/internal/hostmcp"
	"github.com/mbourmaud/hive/internal/preflight"
	"github.com/mbourmaud/hive/internal/shell"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

var (
	startSkipChecks bool
	startWaitReady  bool
)

var startCmd = &cobra.Command{
	Use:   "start [count]",
	Short: "Start queen + N workers",
	Long:  "Start the hive with queen and specified number of workers (default: 2)",
	Args:  cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		// Load config (from hive.yaml or defaults)
		cfg := config.LoadOrDefault()

		// Check if .hive/ exists, if not, run initial setup
		if !fileExists(".hive/entrypoint.sh") {
			fmt.Printf("%s\n", ui.StyleCyan.Render("📦 First run - setting up .hive/..."))
			if err := runFirstTimeSetup(cfg); err != nil {
				return fmt.Errorf("first-time setup failed: %w", err)
			}
			fmt.Println()
		}

		// Run preflight checks unless skipped
		if !startSkipChecks {
			results := preflight.RunAllChecks()
			if !preflight.PrintResults(results) {
				return fmt.Errorf("preflight checks failed. Use --skip-checks to bypass")
			}
		}

		// CLI argument takes precedence over config
		count := cfg.Agents.Workers.Count
		if len(args) > 0 {
			var err error
			count, err = strconv.Atoi(args[0])
			if err != nil {
				return fmt.Errorf("invalid count: %s", args[0])
			}
		}

		if count > 10 {
			return fmt.Errorf("maximum 10 workers allowed")
		}

		if count < 1 {
			return fmt.Errorf("minimum 1 worker required")
		}

		// Create history files to prevent Docker from creating them as directories
		// Docker bind mounts create directories when the source doesn't exist
		hiveDir := ".hive"
		workspacesDir := filepath.Join(hiveDir, "workspaces")
		agents := []string{"queen"}
		for i := 1; i <= count; i++ {
			agents = append(agents, fmt.Sprintf("drone-%d", i))
		}
		for _, agent := range agents {
			// Create workspaces/<agent>/ with history.jsonl and session-env/
			// Matches docker-compose volume mounts: ./workspaces/<agent>/history.jsonl
			agentDir := filepath.Join(workspacesDir, agent)
			sessionEnvDir := filepath.Join(agentDir, "session-env")
			if err := os.MkdirAll(sessionEnvDir, 0755); err != nil {
				return fmt.Errorf("failed to create workspace dir for %s: %w", agent, err)
			}
			historyFile := filepath.Join(agentDir, "history.jsonl")
			if _, err := os.Stat(historyFile); os.IsNotExist(err) {
				if err := os.WriteFile(historyFile, []byte{}, 0644); err != nil {
					return fmt.Errorf("failed to create history file for %s: %w", agent, err)
				}
			}
		}

		// Generate .env.generated from hive.yaml config
		if err := cfg.WriteEnvGenerated(hiveDir); err != nil {
			return fmt.Errorf("failed to generate env vars: %w", err)
		}

		// Update worker count in config if overridden via CLI
		if count != cfg.Agents.Workers.Count {
			cfg.Agents.Workers.Count = count
		}

		// Regenerate docker-compose.yml with full config (prefix, ports, etc.)
		if err := generateDockerComposeFromConfig(cfg); err != nil {
			return fmt.Errorf("failed to generate docker-compose.yml: %w", err)
		}

		// Sync hive.yaml to .hive/ for container access
		if err := syncHiveYAML(); err != nil {
			fmt.Printf("%s\n", ui.Warning("hive.yaml sync: "+err.Error()))
		}

		// Sync host MCPs to .hive/ for container access
		if err := syncHostMCPs(); err != nil {
			fmt.Printf("%s\n", ui.Warning("host MCPs sync: "+err.Error()))
		}

		// Sync CLAUDE.md to .hive/ for container access
		if err := syncProjectCLAUDEmd(); err != nil {
			fmt.Printf("%s\n", ui.Warning("CLAUDE.md sync: "+err.Error()))
		}

		// Start host MCPs if configured
		if cfg.HostMCPs.IsPlaywrightEnabled() || cfg.HostMCPs.IsIOSEnabled() || cfg.HostMCPs.IsClipboardEnabled() {
			mcpManager := hostmcp.NewManager(hiveDir, cfg)
			if err := startHostMCPs(mcpManager, cfg); err != nil {
				fmt.Printf("%s\n", ui.Warning("Host MCPs: "+err.Error()))
			}
		}

		// Header
		fmt.Print(ui.Header("🚀", "Starting Hive"))
		fmt.Printf("%sQueen + %d worker%s%s\n\n", ui.StyleDim.Render(""), count, pluralize(count), "")

		// Build services list (Redis must start first)
		services := []string{"redis", "queen"}
		for i := 1; i <= count; i++ {
			services = append(services, fmt.Sprintf("drone-%d", i))
		}

		// Create shell runner
		runner := shell.NewRunner(DebugMode)

		// Start docker compose services from .hive directory
		cmdArgs := append([]string{"compose", "-f", "docker-compose.yml", "up", "-d"}, services...)
		dockerCmd := exec.Command("docker", cmdArgs...)
		dockerCmd.Dir = hiveDir

		if err := runner.RunWithTitle(dockerCmd, "Docker Compose Start"); err != nil {
			return fmt.Errorf("failed to start containers")
		}

		// Wait for containers to be healthy if requested
		if startWaitReady {
			fmt.Printf("%s\n", ui.StyleCyan.Render("⏳ Waiting for containers..."))
			if err := waitForContainersReady(runner, services, 60*time.Second); err != nil {
				return err
			}
			fmt.Println()
		}

		// Success message
		fmt.Printf("\n%s\n", ui.Success("Hive started successfully!"))
		fmt.Printf("%s\n\n", ui.StyleDim.Render(fmt.Sprintf("%d container%s running", len(services), pluralize(len(services)))))

		// Next steps
		steps := []ui.Step{
			{Command: "hive connect queen", Description: "Connect to orchestrator"},
			{Command: "hive connect 1", Description: "Connect to worker 1"},
			{Command: "hive status", Description: "Check status"},
		}
		fmt.Print(ui.NextSteps(steps))

		return nil
	},
}

// waitForContainersReady waits for all containers to be running
func waitForContainersReady(runner *shell.Runner, services []string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	prefix := getContainerPrefix()

	for _, service := range services {
		var containerName string
		switch service {
		case "queen":
			containerName = prefix + "-queen"
		case "redis":
			containerName = prefix + "-redis"
		default:
			containerName = prefix + "-" + service
		}

		fmt.Printf("  %s", ui.StyleDim.Render(containerName+"..."))

		for {
			if time.Now().After(deadline) {
				fmt.Printf(" %s\n", ui.StyleYellow.Render("TIMEOUT"))
				return fmt.Errorf("timeout waiting for %s to be ready", containerName)
			}

			cmd := exec.Command("docker", "inspect", "-f", "{{.State.Running}}", containerName)
			stdout, _, err := runner.RunCapture(cmd)
			if err == nil && stdout == "true\n" {
				fmt.Printf(" %s\n", ui.StyleGreen.Render("✓"))
				break
			}

			time.Sleep(1 * time.Second)
		}
	}

	return nil
}

// startHostMCPs starts configured host MCP servers
func startHostMCPs(manager *hostmcp.Manager, cfg *config.Config) error {
	var started []string
	var errors []string

	// Start Playwright MCP if enabled
	if cfg.HostMCPs.IsPlaywrightEnabled() {
		// Check prerequisites
		if err := hostmcp.CheckPlaywrightInstalled(); err != nil {
			errors = append(errors, fmt.Sprintf("playwright: %v", err))
		} else {
			fmt.Printf("%s Starting Playwright MCP on port %d...\n",
				ui.StyleCyan.Render("🌐"),
				cfg.HostMCPs.GetPlaywrightPort())

			if err := manager.StartPlaywright(); err != nil {
				errors = append(errors, fmt.Sprintf("playwright: %v", err))
			} else {
				started = append(started, fmt.Sprintf("playwright (port %d)", cfg.HostMCPs.GetPlaywrightPort()))
			}
		}
	}

	// Start iOS MCP if enabled
	if cfg.HostMCPs.IsIOSEnabled() {
		// Check prerequisites
		if err := hostmcp.CheckXcodeInstalled(); err != nil {
			errors = append(errors, fmt.Sprintf("ios: %v", err))
		} else {
			fmt.Printf("%s Starting iOS MCP on port %d...\n",
				ui.StyleCyan.Render("📱"),
				cfg.HostMCPs.GetIOSPort())

			if err := manager.StartIOS(); err != nil {
				errors = append(errors, fmt.Sprintf("ios: %v", err))
			} else {
				started = append(started, fmt.Sprintf("ios (port %d)", cfg.HostMCPs.GetIOSPort()))
			}
		}
	}

	// Start Clipboard MCP if enabled
	if cfg.HostMCPs.IsClipboardEnabled() {
		fmt.Printf("%s Starting Clipboard MCP on port %d...\n",
			ui.StyleCyan.Render("📋"),
			cfg.HostMCPs.GetClipboardPort())

		if !hostmcp.CheckPngpasteInstalled() {
			fmt.Printf("%s pngpaste not installed - image clipboard disabled (brew install pngpaste)\n",
				ui.StyleYellow.Render("⚠"))
		}

		if err := manager.StartClipboard(); err != nil {
			errors = append(errors, fmt.Sprintf("clipboard: %v", err))
		} else {
			started = append(started, fmt.Sprintf("clipboard (port %d)", cfg.HostMCPs.GetClipboardPort()))
		}
	}

	// Print summary
	if len(started) > 0 {
		fmt.Printf("%s Host MCPs running: %v\n\n", ui.StyleGreen.Render("✓"), started)
	}

	if len(errors) > 0 {
		return fmt.Errorf("some host MCPs failed: %v", errors)
	}

	return nil
}

// runFirstTimeSetup extracts hive files and sets up .hive/ directory
// This is called automatically by 'hive start' when .hive/ doesn't exist
// but hive.yaml is present, enabling a simpler workflow where users just
// copy a project with hive.yaml and run 'hive start' without 'hive init'
func runFirstTimeSetup(cfg *config.Config) error {
	hiveDir := ".hive"

	// Extract hive files to .hive/ directory
	if err := extractHiveFiles(""); err != nil {
		return fmt.Errorf("failed to extract hive files: %w", err)
	}
	fmt.Print(ui.ProgressLine("Extracted .hive/", "✓"))

	// Create logs and pids directories for host MCPs
	if err := os.MkdirAll(filepath.Join(hiveDir, "logs"), 0755); err != nil {
		return fmt.Errorf("failed to create logs directory: %w", err)
	}
	if err := os.MkdirAll(filepath.Join(hiveDir, "pids"), 0755); err != nil {
		return fmt.Errorf("failed to create pids directory: %w", err)
	}

	// Detect Claude OAuth token from host
	claudeToken := detectClaudeToken()

	// Generate .env file with defaults from config
	envConfig := map[string]string{
		"WORKSPACE_NAME":        cfg.Workspace.Name,
		"GIT_REPO_URL":          cfg.Workspace.GitURL,
		"HIVE_CLAUDE_BACKEND":   "cli",
		"WORKER_MODE":           "interactive",
		"CLAUDE_CODE_OAUTH_TOKEN": claudeToken,
	}
	if err := writeEnvFile(envConfig, cfg.Agents.Workers.Count); err != nil {
		return fmt.Errorf("failed to write .env: %w", err)
	}
	fmt.Print(ui.ProgressLine("Created .hive/.env", "✓"))

	// Generate .env.generated from hive.yaml
	if err := cfg.WriteEnvGenerated(hiveDir); err != nil {
		return fmt.Errorf("failed to generate .env.generated: %w", err)
	}
	fmt.Print(ui.ProgressLine("Generated .hive/.env.generated", "✓"))

	// Generate docker-compose.yml
	if err := generateDockerComposeFromConfig(cfg); err != nil {
		return fmt.Errorf("failed to generate docker-compose.yml: %w", err)
	}
	fmt.Print(ui.ProgressLine("Generated docker-compose.yml", "✓"))

	// Sync hive.yaml to .hive/
	if err := syncHiveYAML(); err != nil {
		fmt.Printf("  %s\n", ui.Warning("hive.yaml sync: "+err.Error()))
	}

	// Sync host MCPs to .hive/
	if err := syncHostMCPs(); err != nil {
		fmt.Printf("  %s\n", ui.Warning("host MCPs sync: "+err.Error()))
	}

	// Sync custom Dockerfiles if configured
	if cfg.Agents.Queen.Dockerfile != "" || cfg.Agents.Workers.Dockerfile != "" {
		if err := syncCustomDockerfiles(cfg); err != nil {
			fmt.Printf("  %s\n", ui.Warning("Custom Dockerfiles: "+err.Error()))
		}
	}

	// Copy CA certificate if configured
	if cfg.Network.CACert != "" {
		if err := copyCACertificate(cfg); err != nil {
			return fmt.Errorf("failed to copy CA certificate: %w", err)
		}
		fmt.Print(ui.ProgressLine("Copied CA certificate", "✓"))
	}

	// Create git worktrees for each agent (ignore errors - may already exist or not in git repo)
	if err := createWorktrees(cfg.Agents.Workers.Count); err != nil {
		fmt.Printf("  %s\n", ui.StyleDim.Render("Worktrees: skipped ("+err.Error()+")"))
	} else {
		fmt.Print(ui.ProgressLine("Created git worktrees", "✓"))
	}

	// Update .gitignore
	if err := updateGitignore(); err != nil {
		fmt.Printf("  %s\n", ui.Warning(".gitignore: "+err.Error()))
	}

	return nil
}

func init() {
	rootCmd.AddCommand(startCmd)
	startCmd.Flags().BoolVar(&startSkipChecks, "skip-checks", false, "Skip preflight checks")
	startCmd.Flags().BoolVar(&startWaitReady, "wait", false, "Wait for containers to be ready")
}
