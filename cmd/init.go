package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"runtime"
	"strconv"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/mbourmaud/hive/internal/hostmcp"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

// Flag variables for init command
var (
	flagNonInteractive bool
	flagEmail          string
	flagName           string
	flagToken          string
	flagApiKey         string
	flagAuthBackend    string
	flagWorkspace      string
	flagWorkers        int
	flagWorkerMode     string
	flagGitURL         string
	flagSkipStart      bool
)

var initCmd = &cobra.Command{
	Use:   "init",
	Short: "Initialize Hive with interactive wizard",
	Long: `Initialize Hive configuration with an interactive wizard.

This command will:
  1. Gather your Git and Claude configuration
  2. Create .env file with your settings
  3. Build and install the CLI
  4. Start Hive with specified workers

For automation (Claude, CI/CD), use -y or --yes to accept defaults.`,
	RunE: runInit,
}

func init() {
	rootCmd.AddCommand(initCmd)

	initCmd.Flags().BoolVarP(&flagNonInteractive, "yes", "y", false, "Accept defaults, skip interactive prompts")
	initCmd.Flags().StringVar(&flagEmail, "email", "", "Git user email")
	initCmd.Flags().StringVar(&flagName, "name", "", "Git user name")
	initCmd.Flags().StringVar(&flagToken, "token", "", "Claude OAuth token (for cli backend)")
	initCmd.Flags().StringVar(&flagApiKey, "api-key", "", "Anthropic API key (for api backend)")
	initCmd.Flags().StringVar(&flagAuthBackend, "auth", "cli", "Auth backend: cli (OAuth), api (API key), bedrock (AWS)")
	initCmd.Flags().StringVar(&flagWorkspace, "workspace", "my-project", "Workspace name")
	initCmd.Flags().IntVar(&flagWorkers, "workers", 2, "Number of workers to start")
	initCmd.Flags().StringVar(&flagWorkerMode, "mode", "interactive", "Worker mode: interactive, daemon (autonomous)")
	initCmd.Flags().StringVar(&flagGitURL, "git-url", "", "Git repository URL to clone (optional)")
	initCmd.Flags().BoolVar(&flagSkipStart, "skip-start", false, "Skip starting Hive after initialization (for testing)")
}

func runInit(cmd *cobra.Command, args []string) error {
	// Check if already initialized
	if fileExists(".hive/.env") {
		return fmt.Errorf(".hive/ already exists. Use 'hive clean' to reset")
	}

	fmt.Print(ui.Header("🐝", "Welcome to HIVE"))
	fmt.Printf("%s\n\n", ui.StyleDim.Render("Multi-Agent Claude System"))

	// Detect project info
	projectType := detectProjectType()
	email, name, repoURL, workspaceName := detectGitConfig()
	claudeToken := detectClaudeToken()

	var cfg map[string]string

	if flagNonInteractive {
		// Non-interactive mode: use flags (with detected values as fallback)
		if flagEmail == "" {
			flagEmail = email
		}
		if flagName == "" {
			flagName = name
		}
		if flagToken == "" {
			flagToken = claudeToken
		}
		if flagApiKey == "" {
			flagApiKey = os.Getenv("ANTHROPIC_API_KEY")
		}
		if flagWorkspace == "my-project" && workspaceName != "" {
			flagWorkspace = workspaceName
		}
		if flagGitURL == "" {
			flagGitURL = repoURL
		}

		if err := validateFlags(); err != nil {
			return err
		}

		cfg = map[string]string{
			"GIT_USER_EMAIL":      flagEmail,
			"GIT_USER_NAME":       flagName,
			"WORKSPACE_NAME":      flagWorkspace,
			"GIT_REPO_URL":        flagGitURL,
			"PROJECT_TYPE":        projectType,
			"HIVE_CLAUDE_BACKEND": flagAuthBackend,
			"WORKER_MODE":         flagWorkerMode,
		}

		// Add auth credentials based on backend
		switch flagAuthBackend {
		case "cli":
			cfg["CLAUDE_CODE_OAUTH_TOKEN"] = flagToken
		case "api":
			cfg["ANTHROPIC_API_KEY"] = flagApiKey
		case "bedrock":
			cfg["AWS_PROFILE"] = os.Getenv("AWS_PROFILE")
			cfg["AWS_REGION"] = os.Getenv("AWS_REGION")
		}
	} else {
		// Interactive mode with auto-detection
		var err error
		cfg, err = interactiveWizardWithDetection(email, name, repoURL, workspaceName, claudeToken, projectType)
		if err != nil {
			return err
		}
	}

	// Extract hive files to .hive/ directory
	fmt.Printf("%s\n", ui.StyleCyan.Render("📦 Setting up project..."))
	if err := extractHiveFiles(cfg["PROJECT_TYPE"]); err != nil {
		return fmt.Errorf("failed to extract hive files: %w", err)
	}
	fmt.Print(ui.ProgressLine("Extracted .hive/", "✓"))

	// Detect Node version from package.json
	if nodeVersion := detectNodeVersion(); nodeVersion != "" {
		cfg["NODE_VERSION"] = nodeVersion
	}

	// Ask for workers count and mode before writing config files
	workers := flagWorkers
	if !flagNonInteractive {
		fmt.Println()
		workersStr, err := ui.PromptDefault("🚀 Workers to start", "2")
		if err == nil {
			if w, parseErr := strconv.Atoi(workersStr); parseErr == nil {
				workers = w
			}
		}

		// Ask for worker mode
		fmt.Println()
		fmt.Println("🤖 Worker mode:")
		fmt.Println("   1. Interactive (manual CLI control)")
		fmt.Println("   2. Autonomous (daemon mode, executes tasks automatically)")
		fmt.Println("   3. Hybrid (configure per-worker in .env)")
		modeChoice, err := ui.PromptDefault("Choose mode", "1")
		if err == nil {
			switch modeChoice {
			case "1":
				cfg["WORKER_MODE"] = "interactive"
			case "2":
				cfg["WORKER_MODE"] = "daemon"
			case "3":
				cfg["WORKER_MODE"] = "hybrid"
			default:
				cfg["WORKER_MODE"] = "interactive"
			}
		}
	}

	// Generate docker-compose.yml with the correct worker count
	if err := generateDockerCompose(workers); err != nil {
		return fmt.Errorf("failed to generate docker-compose.yml: %w", err)
	}
	fmt.Print(ui.ProgressLine("Generated docker-compose.yml", "✓"))

	// Write .env file
	if err := writeEnvFile(cfg, workers); err != nil {
		return fmt.Errorf("failed to write .hive/.env: %w", err)
	}
	fmt.Print(ui.ProgressLine("Created .hive/.env", "✓"))

	// Write hive.yaml file (contains all configuration)
	if err := writeHiveYAML(cfg, workers); err != nil {
		return fmt.Errorf("failed to write hive.yaml: %w", err)
	}
	fmt.Print(ui.ProgressLine("Created hive.yaml", "✓"))

	// Copy hive.yaml to .hive/ for container access
	if err := syncHiveYAML(); err != nil {
		return fmt.Errorf("failed to sync hive.yaml: %w", err)
	}
	fmt.Print(ui.ProgressLine("Synced hive.yaml to .hive/", "✓"))

	// Copy host MCPs to .hive/ for container access
	if err := syncHostMCPs(); err != nil {
		return fmt.Errorf("failed to sync host MCPs: %w", err)
	}
	fmt.Print(ui.ProgressLine("Synced host MCPs to .hive/", "✓"))

	// Copy CLAUDE.md to .hive/ for container access
	if err := syncProjectCLAUDEmd(); err != nil {
		return fmt.Errorf("failed to sync CLAUDE.md: %w", err)
	}
	fmt.Print(ui.ProgressLine("Synced CLAUDE.md to .hive/", "✓"))

	// Generate .env.generated from hive.yaml for docker-compose
	hiveCfg := config.LoadOrDefault()
	if err := hiveCfg.WriteEnvGenerated(".hive"); err != nil {
		return fmt.Errorf("failed to generate env vars: %w", err)
	}
	fmt.Print(ui.ProgressLine("Generated .hive/.env.generated", "✓"))

	// Check and install pngpaste if clipboard MCP is enabled
	if hiveCfg.HostMCPs.IsClipboardEnabled() {
		if err := ensurePngpaste(flagNonInteractive); err != nil {
			fmt.Printf("  %s\n", ui.Warning("pngpaste: "+err.Error()))
		}
	}

	// Sync custom Dockerfiles from project root to .hive/
	if hiveCfg.Agents.Queen.Dockerfile != "" || hiveCfg.Agents.Workers.Dockerfile != "" {
		if err := syncCustomDockerfiles(hiveCfg); err != nil {
			fmt.Printf("  %s\n", ui.Warning("Custom Dockerfiles: "+err.Error()))
		} else {
			fmt.Print(ui.ProgressLine("Synced custom Dockerfiles", "✓"))
		}
	}

	// Regenerate docker-compose.yml with full config (ports, volumes, dockerfile paths)
	if err := generateDockerComposeFromConfig(hiveCfg); err != nil {
		return fmt.Errorf("failed to regenerate docker-compose.yml: %w", err)
	}
	fmt.Print(ui.ProgressLine("Regenerated docker-compose.yml with config", "✓"))

	// Copy CA certificate if configured (for corporate proxy support)
	if hiveCfg.Network.CACert != "" {
		if err := copyCACertificate(hiveCfg); err != nil {
			return fmt.Errorf("failed to copy CA certificate: %w", err)
		}
		fmt.Print(ui.ProgressLine("Copied CA certificate for proxy", "✓"))
	}

	// Update .gitignore
	if err := updateGitignore(); err != nil {
		fmt.Printf("  %s\n", ui.Warning(".gitignore: "+err.Error()))
	} else {
		fmt.Print(ui.ProgressLine("Updated .gitignore", "✓"))
	}

	// Create git worktrees for each agent
	fmt.Println()
	if err := createWorktrees(workers); err != nil {
		fmt.Printf("%s\n", ui.StyleCyan.Render(fmt.Sprintf("🌳 Worktrees... ⚠️  %v", err)))
		fmt.Printf("%s\n", ui.StyleDim.Render("   Agents will use empty workspaces"))
	}

	// Skip starting if --skip-start flag is set
	if flagSkipStart {
		fmt.Printf("\n%s\n", ui.StyleDim.Render("Skipping hive start (--skip-start flag)"))
		fmt.Printf("%s\n", ui.StyleDim.Render("Run manually:"))
		fmt.Printf("  %s\n\n", ui.StyleCyan.Render(fmt.Sprintf("hive start %d", workers)))
		return nil
	}

	fmt.Print(ui.Header("🚀", "Starting Hive"))
	startCmd := exec.Command("hive", "start", strconv.Itoa(workers))
	startCmd.Stdout = os.Stdout
	startCmd.Stderr = os.Stderr

	if err := startCmd.Run(); err != nil {
		fmt.Printf("\n%s\n", ui.Warning("Failed to start Hive"))
		fmt.Printf("%s\n", ui.StyleDim.Render("Please check the error above and run manually:"))
		fmt.Printf("  %s\n\n", ui.StyleCyan.Render(fmt.Sprintf("hive start %d", workers)))
		return fmt.Errorf("hive start failed")
	}

	// Print success message
	printSuccessMessage(workers)

	return nil
}

// ensurePngpaste checks if pngpaste is installed and offers to install it via Homebrew
func ensurePngpaste(nonInteractive bool) error {
	// Only works on macOS
	if runtime.GOOS != "darwin" {
		return nil
	}

	// Check if already installed
	if hostmcp.CheckPngpasteInstalled() {
		fmt.Print(ui.ProgressLine("pngpaste installed (clipboard image support)", "✓"))
		return nil
	}

	// Check if Homebrew is available
	_, err := exec.LookPath("brew")
	if err != nil {
		fmt.Print(ui.ProgressLine("pngpaste not installed (no Homebrew)", "⚠"))
		fmt.Printf("  %s\n", ui.StyleDim.Render("Install manually: brew install pngpaste"))
		return nil
	}

	// In non-interactive mode, auto-install
	if nonInteractive {
		fmt.Printf("%s Installing pngpaste for clipboard image support...\n", ui.StyleCyan.Render("📋"))
		cmd := exec.Command("brew", "install", "pngpaste")
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
		if err := cmd.Run(); err != nil {
			return fmt.Errorf("failed to install pngpaste: %w", err)
		}
		fmt.Print(ui.ProgressLine("Installed pngpaste", "✓"))
		return nil
	}

	// Interactive mode: ask user
	fmt.Println()
	fmt.Printf("%s pngpaste is required for clipboard image support\n", ui.StyleYellow.Render("📋"))
	response, err := ui.PromptDefault("Install pngpaste via Homebrew?", "y")
	if err != nil {
		return nil
	}

	if response == "y" || response == "Y" || response == "yes" {
		fmt.Printf("%s Installing pngpaste...\n", ui.StyleCyan.Render("📦"))
		cmd := exec.Command("brew", "install", "pngpaste")
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
		if err := cmd.Run(); err != nil {
			return fmt.Errorf("failed to install pngpaste: %w", err)
		}
		fmt.Print(ui.ProgressLine("Installed pngpaste", "✓"))
	} else {
		fmt.Print(ui.ProgressLine("Skipped pngpaste (text-only clipboard)", "⚠"))
	}

	return nil
}
