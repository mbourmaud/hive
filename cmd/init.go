package cmd

import (
	"bytes"
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/mbourmaud/hive/internal/compose"
	"github.com/mbourmaud/hive/internal/config"
	"github.com/mbourmaud/hive/internal/embed"
	"github.com/mbourmaud/hive/internal/ui"
	"github.com/spf13/cobra"
)

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

For automation (Claude, CI/CD), use --no-interactive with flags.`,
	RunE: runInit,
}

func init() {
	rootCmd.AddCommand(initCmd)

	initCmd.Flags().BoolVar(&flagNonInteractive, "no-interactive", false, "Skip interactive prompts, use flags only")
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
			"GIT_USER_EMAIL":       flagEmail,
			"GIT_USER_NAME":        flagName,
			"WORKSPACE_NAME":       flagWorkspace,
			"GIT_REPO_URL":         flagGitURL,
			"PROJECT_TYPE":         projectType,
			"HIVE_CLAUDE_BACKEND":  flagAuthBackend,
			"WORKER_MODE":          flagWorkerMode,
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

// interactiveWizardWithDetection shows detected values and only asks for missing info
func interactiveWizardWithDetection(email, name, repoURL, workspaceName, claudeToken, projectType string) (map[string]string, error) {
	cfg := make(map[string]string)

	// Show detected project info
	projectTypeDisplay := map[string]string{
		"node":    "Node.js",
		"go":      "Go",
		"python":  "Python",
		"rust":    "Rust",
		"minimal": "Generic",
	}
	fmt.Printf("📂 Project detected: %s (%s)\n", workspaceName, projectTypeDisplay[projectType])

	if email != "" {
		fmt.Printf("   Git email: %s\n", email)
	}
	if name != "" {
		fmt.Printf("   Git name: %s\n", name)
	}
	if repoURL != "" {
		fmt.Printf("   Remote: %s\n", repoURL)
	}
	fmt.Println()

	// Use detected values as defaults
	cfg["PROJECT_TYPE"] = projectType

	// =============================================
	// 1. Git Configuration (editable even if detected)
	// =============================================
	fmt.Println("📧 Git Configuration")
	emailInput, err := ui.PromptDefault("  Email", email)
	if err != nil {
		return nil, err
	}
	if err := validateEmail(emailInput); err != nil {
		return nil, fmt.Errorf("invalid email: %w", err)
	}
	cfg["GIT_USER_EMAIL"] = emailInput

	nameInput, err := ui.PromptDefault("  Name", name)
	if err != nil {
		return nil, err
	}
	cfg["GIT_USER_NAME"] = nameInput
	fmt.Println()

	// =============================================
	// 2. Workspace Configuration
	// =============================================
	fmt.Println("📂 Workspace")
	wsName, err := ui.PromptDefault("  Name", workspaceName)
	if err != nil {
		return nil, err
	}
	cfg["WORKSPACE_NAME"] = wsName

	gitURL, err := ui.PromptDefault("  Git URL", repoURL)
	if err != nil {
		return nil, err
	}
	cfg["GIT_REPO_URL"] = gitURL
	fmt.Println()

	// =============================================
	// 3. Claude Authentication
	// =============================================
	apiKey := detectAnthropicApiKey()

	fmt.Println("🔑 Claude Authentication")
	fmt.Println("   1. OAuth (Claude Max/Pro - FREE)")
	fmt.Println("   2. API Key (Pay-as-you-go)")
	fmt.Println("   3. AWS Bedrock (Enterprise)")

	// Set default based on what's detected
	defaultChoice := "1"
	if claudeToken != "" {
		fmt.Printf("   %s\n", ui.StyleDim.Render("OAuth token detected"))
	}
	if apiKey != "" {
		fmt.Printf("   %s\n", ui.StyleDim.Render("API key detected"))
		if claudeToken == "" {
			defaultChoice = "2"
		}
	}

	authChoice, err := ui.PromptDefault("Choose auth method", defaultChoice)
	if err != nil {
		return nil, err
	}

	switch authChoice {
	case "1": // OAuth
		cfg["HIVE_CLAUDE_BACKEND"] = "cli"
		if claudeToken != "" {
			cfg["CLAUDE_CODE_OAUTH_TOKEN"] = claudeToken
		} else {
			fmt.Println("   Get your token: claude /auth")
			tokenInput, err := ui.PromptSecret("   OAuth Token")
			if err != nil {
				return nil, err
			}
			cfg["CLAUDE_CODE_OAUTH_TOKEN"] = tokenInput
		}

	case "2": // API Key
		cfg["HIVE_CLAUDE_BACKEND"] = "api"
		if apiKey != "" {
			cfg["ANTHROPIC_API_KEY"] = apiKey
		} else {
			fmt.Println("   Get your key: https://console.anthropic.com/settings/keys")
			keyInput, err := ui.PromptSecret("   API Key")
			if err != nil {
				return nil, err
			}
			cfg["ANTHROPIC_API_KEY"] = keyInput
		}

	case "3": // Bedrock
		cfg["HIVE_CLAUDE_BACKEND"] = "bedrock"
		profile, err := ui.PromptDefault("   AWS Profile", "default")
		if err != nil {
			return nil, err
		}
		cfg["AWS_PROFILE"] = profile
		region, err := ui.PromptDefault("   AWS Region", "us-east-1")
		if err != nil {
			return nil, err
		}
		cfg["AWS_REGION"] = region

	default:
		cfg["HIVE_CLAUDE_BACKEND"] = "cli"
		if claudeToken != "" {
			cfg["CLAUDE_CODE_OAUTH_TOKEN"] = claudeToken
		}
	}
	fmt.Println()

	// =============================================
	// 4. Agent Models
	// =============================================
	fmt.Println("🤖 Agent Models")
	fmt.Println("   Available: opus, sonnet, haiku")

	queenModel, err := ui.PromptDefault("  Queen model", "opus")
	if err != nil {
		return nil, err
	}
	cfg["QUEEN_MODEL"] = queenModel

	workerModel, err := ui.PromptDefault("  Worker model", "sonnet")
	if err != nil {
		return nil, err
	}
	cfg["WORKER_MODEL"] = workerModel
	fmt.Println()

	// =============================================
	// 5. Docker Image
	// =============================================
	fmt.Println("🐳 Docker Image")
	defaultDockerfile := "docker/Dockerfile." + projectType
	fmt.Printf("   Available: node, go, python, rust, minimal\n")
	dockerfile, err := ui.PromptDefault("  Dockerfile", defaultDockerfile)
	if err != nil {
		return nil, err
	}
	cfg["HIVE_DOCKERFILE"] = dockerfile
	fmt.Println()

	return cfg, nil
}

// detectAnthropicApiKey attempts to find Anthropic API key from environment
func detectAnthropicApiKey() string {
	return os.Getenv("ANTHROPIC_API_KEY")
}

func validateFlags() error {
	if flagEmail == "" {
		return fmt.Errorf("--email is required in non-interactive mode")
	}
	if flagName == "" {
		return fmt.Errorf("--name is required in non-interactive mode")
	}
	if err := validateEmail(flagEmail); err != nil {
		return err
	}

	// Validate auth based on backend
	switch flagAuthBackend {
	case "cli":
		if flagToken == "" {
			return fmt.Errorf("--token is required for cli backend (or set CLAUDE_CODE_OAUTH_TOKEN)")
		}
	case "api":
		if flagApiKey == "" {
			return fmt.Errorf("--api-key is required for api backend (or set ANTHROPIC_API_KEY)")
		}
	case "bedrock":
		// Bedrock uses AWS credentials from environment
	default:
		return fmt.Errorf("--auth must be one of: cli, api, bedrock")
	}

	return nil
}

func validateEmail(email string) error {
	// Basic email validation: user@domain.tld
	if email == "" {
		return fmt.Errorf("email cannot be empty")
	}

	parts := strings.Split(email, "@")
	if len(parts) != 2 {
		return fmt.Errorf("invalid email format: must contain exactly one @")
	}

	user, domain := parts[0], parts[1]
	if user == "" {
		return fmt.Errorf("invalid email format: missing user part")
	}
	if domain == "" || !strings.Contains(domain, ".") {
		return fmt.Errorf("invalid email format: invalid domain")
	}

	// Check that domain doesn't start with a dot
	if strings.HasPrefix(domain, ".") {
		return fmt.Errorf("invalid email format: domain cannot start with a dot")
	}

	return nil
}

// generateSecurePassword generates a cryptographically secure password
func generateSecurePassword(length int) (string, error) {
	bytes := make([]byte, length)
	if _, err := rand.Read(bytes); err != nil {
		return "", fmt.Errorf("failed to generate secure password: %w", err)
	}
	// Use URL-safe base64 and trim to exact length
	return base64.URLEncoding.EncodeToString(bytes)[:length], nil
}

func writeEnvFile(cfg map[string]string, workers int) error {
	// .env now contains ONLY secrets - configuration is in hive.yaml
	var content strings.Builder

	content.WriteString("# ===========================================\n")
	content.WriteString("# Hive Secrets (generated by hive init)\n")
	content.WriteString("# ===========================================\n")
	content.WriteString("# This file contains SECRETS ONLY.\n")
	content.WriteString("# All configuration is in hive.yaml\n")
	content.WriteString("# DO NOT commit this file to git!\n")
	content.WriteString("# ===========================================\n\n")

	// Claude Authentication - only write what's needed based on backend
	backend := cfg["HIVE_CLAUDE_BACKEND"]
	if backend == "" {
		backend = "cli" // Default to CLI/OAuth
	}
	content.WriteString(fmt.Sprintf("HIVE_CLAUDE_BACKEND=%s\n", backend))

	switch backend {
	case "cli":
		if token := cfg["CLAUDE_CODE_OAUTH_TOKEN"]; token != "" {
			content.WriteString(fmt.Sprintf("CLAUDE_CODE_OAUTH_TOKEN=%s\n", token))
		}
	case "api":
		if key := cfg["ANTHROPIC_API_KEY"]; key != "" {
			content.WriteString(fmt.Sprintf("ANTHROPIC_API_KEY=%s\n", key))
		}
	case "bedrock":
		if profile := cfg["AWS_PROFILE"]; profile != "" {
			content.WriteString(fmt.Sprintf("AWS_PROFILE=%s\n", profile))
		}
		if region := cfg["AWS_REGION"]; region != "" {
			content.WriteString(fmt.Sprintf("AWS_REGION=%s\n", region))
		}
	}
	content.WriteString("\n")

	// Redis authentication
	redisPassword, err := generateSecurePassword(32)
	if err != nil {
		return fmt.Errorf("failed to generate Redis password: %w", err)
	}
	content.WriteString("# Redis Authentication\n")
	content.WriteString(fmt.Sprintf("REDIS_PASSWORD=%s\n", redisPassword))
	content.WriteString("\n")

	// Optional VCS tokens (user may add these)
	content.WriteString("# VCS Tokens (optional)\n")
	content.WriteString("# GITHUB_TOKEN=\n")
	content.WriteString("# GITLAB_TOKEN=\n")
	content.WriteString("\n")

	// Worker modes
	if workerMode := cfg["WORKER_MODE"]; workerMode != "" && workerMode != "hybrid" {
		content.WriteString("# Worker modes\n")
		for i := 1; i <= workers; i++ {
			content.WriteString(fmt.Sprintf("WORKER_%d_MODE=%s\n", i, workerMode))
		}
		content.WriteString("\n")
	}

	// Write to .hive/.env
	return os.WriteFile(".hive/.env", []byte(content.String()), 0600)
}

// writeMinimalEnvFile generates a minimal .env file without template
func writeMinimalEnvFile(cfg map[string]string) error {
	dockerfile := "docker/Dockerfile.node"
	if cfg["PROJECT_TYPE"] != "" {
		dockerfile = "docker/Dockerfile." + cfg["PROJECT_TYPE"]
	}

	// Generate Redis password
	redisPassword, err := generateSecurePassword(32)
	if err != nil {
		return fmt.Errorf("failed to generate Redis password: %w", err)
	}

	content := fmt.Sprintf(`# Hive Configuration
GIT_USER_EMAIL=%s
GIT_USER_NAME=%s
CLAUDE_CODE_OAUTH_TOKEN=%s
WORKSPACE_NAME=%s
GIT_REPO_URL=%s
HIVE_DOCKERFILE=%s
QUEEN_MODEL=opus
WORKER_MODEL=sonnet
AUTO_INSTALL_DEPS=true

# Redis Authentication
REDIS_PASSWORD=%s
`,
		cfg["GIT_USER_EMAIL"],
		cfg["GIT_USER_NAME"],
		cfg["CLAUDE_CODE_OAUTH_TOKEN"],
		cfg["WORKSPACE_NAME"],
		cfg["GIT_REPO_URL"],
		dockerfile,
		redisPassword,
	)

	// Write to .hive/.env
	return os.WriteFile(".hive/.env", []byte(content), 0600)
}

func fileExists(path string) bool {
	absPath, err := filepath.Abs(path)
	if err != nil {
		absPath = path
	}
	_, err = os.Stat(absPath)
	return err == nil
}

func writeHiveYAML(cfgMap map[string]string, workers int) error {
	cfg := config.Default()

	// Update workspace
	if ws := cfgMap["WORKSPACE_NAME"]; ws != "" {
		cfg.Workspace.Name = ws
	}
	if gitURL := cfgMap["GIT_REPO_URL"]; gitURL != "" {
		cfg.Workspace.GitURL = gitURL
	}

	// Update git config
	if email := cfgMap["GIT_USER_EMAIL"]; email != "" {
		cfg.Git.UserEmail = email
	}
	if name := cfgMap["GIT_USER_NAME"]; name != "" {
		cfg.Git.UserName = name
	}

	// Update agents config
	if queenModel := cfgMap["QUEEN_MODEL"]; queenModel != "" {
		cfg.Agents.Queen.Model = queenModel
	}
	if dockerfile := cfgMap["HIVE_DOCKERFILE"]; dockerfile != "" {
		cfg.Agents.Queen.Dockerfile = dockerfile
		cfg.Agents.Workers.Dockerfile = dockerfile
	}
	if workerModel := cfgMap["WORKER_MODEL"]; workerModel != "" {
		cfg.Agents.Workers.Model = workerModel
	}
	if workerMode := cfgMap["WORKER_MODE"]; workerMode != "" {
		cfg.Agents.Workers.Mode = workerMode
	}
	cfg.Agents.Workers.Count = workers

	return cfg.Save("hive.yaml")
}

// syncHiveYAML copies hive.yaml to .hive/hive.yaml for container access
// This is called during init, start, and update to keep the copy in sync
func syncHiveYAML() error {
	src := "hive.yaml"
	dst := filepath.Join(".hive", "hive.yaml")

	// Read source file
	data, err := os.ReadFile(src)
	if err != nil {
		return fmt.Errorf("failed to read %s: %w", src, err)
	}

	// Write to destination
	if err := os.WriteFile(dst, data, 0644); err != nil {
		return fmt.Errorf("failed to write %s: %w", dst, err)
	}

	return nil
}

// syncHostMCPs copies ~/.claude/settings.json to .hive/host-mcps.json
// This allows containers to access host MCPs without individual file mounts
func syncHostMCPs() error {
	home, err := os.UserHomeDir()
	if err != nil {
		return fmt.Errorf("failed to get home directory: %w", err)
	}

	src := filepath.Join(home, ".claude", "settings.json")
	dst := filepath.Join(".hive", "host-mcps.json")

	// Read source file (may not exist, that's OK)
	data, err := os.ReadFile(src)
	if err != nil {
		if os.IsNotExist(err) {
			// No host settings, create empty JSON
			data = []byte("{}")
		} else {
			return fmt.Errorf("failed to read %s: %w", src, err)
		}
	}

	// Write to destination
	if err := os.WriteFile(dst, data, 0644); err != nil {
		return fmt.Errorf("failed to write %s: %w", dst, err)
	}

	return nil
}

// syncProjectCLAUDEmd copies CLAUDE.md to .hive/CLAUDE.md
// This allows containers to access project guidelines
func syncProjectCLAUDEmd() error {
	src := "CLAUDE.md"
	dst := filepath.Join(".hive", "CLAUDE.md")

	// Read source file (may not exist, that's OK)
	data, err := os.ReadFile(src)
	if err != nil {
		if os.IsNotExist(err) {
			// No CLAUDE.md in project, skip
			return nil
		}
		return fmt.Errorf("failed to read %s: %w", src, err)
	}

	// Write to destination
	if err := os.WriteFile(dst, data, 0644); err != nil {
		return fmt.Errorf("failed to write %s: %w", dst, err)
	}

	return nil
}

func printSuccessMessage(workers int) {
	fmt.Printf("\n%s\n", ui.Success("Setup complete!"))
	fmt.Printf("%s\n\n", ui.StyleDim.Render(fmt.Sprintf("Hive is running with %d worker%s", workers, pluralize(workers))))

	steps := []ui.Step{
		{Command: "hive connect queen", Description: "Connect to orchestrator"},
		{Command: "hive connect 1", Description: "Connect to worker 1"},
		{Command: "hive status", Description: "Check status"},
	}
	fmt.Print(ui.NextSteps(steps))
}

// detectGitConfig retrieves git configuration from the current repository
func detectGitConfig() (email, name, repoURL, workspaceName string) {
	// git config user.email
	if out, err := exec.Command("git", "config", "user.email").Output(); err == nil {
		email = strings.TrimSpace(string(out))
	}

	// git config user.name
	if out, err := exec.Command("git", "config", "user.name").Output(); err == nil {
		name = strings.TrimSpace(string(out))
	}

	// git remote get-url origin
	if out, err := exec.Command("git", "remote", "get-url", "origin").Output(); err == nil {
		repoURL = strings.TrimSpace(string(out))
	}

	// Workspace name from current directory
	if cwd, err := os.Getwd(); err == nil {
		workspaceName = filepath.Base(cwd)
	}

	return
}

// detectProjectType detects the project type based on config files
func detectProjectType() string {
	if fileExists("package.json") {
		return "node"
	}
	if fileExists("go.mod") {
		return "go"
	}
	if fileExists("pyproject.toml") || fileExists("requirements.txt") {
		return "python"
	}
	if fileExists("Cargo.toml") {
		return "rust"
	}
	return "minimal"
}

// detectClaudeToken attempts to find Claude OAuth token from existing config
func detectClaudeToken() string {
	home, err := os.UserHomeDir()
	if err != nil {
		return ""
	}

	// Try reading from ~/.claude/settings.json
	settingsPath := filepath.Join(home, ".claude", "settings.json")
	if data, err := os.ReadFile(settingsPath); err == nil {
		var settings map[string]interface{}
		if err := json.Unmarshal(data, &settings); err == nil {
			if oauth, ok := settings["oauthAccount"].(map[string]interface{}); ok {
				if token, ok := oauth["accessToken"].(string); ok && token != "" {
					return token
				}
			}
		}
	}

	// Try environment variable
	if token := os.Getenv("CLAUDE_CODE_OAUTH_TOKEN"); token != "" {
		return token
	}

	return ""
}

// extractHiveFiles copies all necessary hive files to .hive/ directory
// Note: docker-compose.yml is generated dynamically by generateDockerCompose()
func extractHiveFiles(projectType string) error {
	hiveDir := ".hive"

	// Create .hive directory
	if err := os.MkdirAll(hiveDir, 0755); err != nil {
		return fmt.Errorf("failed to create .hive directory: %w", err)
	}

	// docker-compose.yml is generated dynamically after worker count is known
	// See generateDockerCompose()

	// Extract entrypoint.sh
	if err := embed.ExtractFile("entrypoint.sh", filepath.Join(hiveDir, "entrypoint.sh")); err != nil {
		return fmt.Errorf("failed to extract entrypoint.sh: %w", err)
	}

	// Extract worker daemon files for autonomous mode
	if err := embed.ExtractFile("start-worker.sh", filepath.Join(hiveDir, "start-worker.sh")); err != nil {
		return fmt.Errorf("failed to extract start-worker.sh: %w", err)
	}
	if err := embed.ExtractFile("worker-daemon.py", filepath.Join(hiveDir, "worker-daemon.py")); err != nil {
		return fmt.Errorf("failed to extract worker-daemon.py: %w", err)
	}
	if err := embed.ExtractFile("backends.py", filepath.Join(hiveDir, "backends.py")); err != nil {
		return fmt.Errorf("failed to extract backends.py: %w", err)
	}
	if err := embed.ExtractFile("tools.py", filepath.Join(hiveDir, "tools.py")); err != nil {
		return fmt.Errorf("failed to extract tools.py: %w", err)
	}

	// Extract docker directory
	if err := embed.ExtractDir("docker", filepath.Join(hiveDir, "docker")); err != nil {
		return fmt.Errorf("failed to extract docker/: %w", err)
	}

	// Extract scripts directory
	if err := embed.ExtractDir("scripts", filepath.Join(hiveDir, "scripts")); err != nil {
		return fmt.Errorf("failed to extract scripts/: %w", err)
	}

	// Extract templates directory
	if err := embed.ExtractDir("templates", filepath.Join(hiveDir, "templates")); err != nil {
		return fmt.Errorf("failed to extract templates/: %w", err)
	}

	// Create workspaces directory inside .hive
	if err := os.MkdirAll(filepath.Join(hiveDir, "workspaces"), 0755); err != nil {
		return fmt.Errorf("failed to create workspaces directory: %w", err)
	}

	return nil
}

// createWorktrees creates git worktrees for each agent if in a git repo
func createWorktrees(workers int) error {
	// Check if we're in a git repository
	gitCmd := exec.Command("git", "rev-parse", "--is-inside-work-tree")
	gitCmd.Stdout = nil
	gitCmd.Stderr = nil
	if err := gitCmd.Run(); err != nil {
		// Not a git repo, skip worktree creation
		return nil
	}

	// Get current branch
	branchCmd := exec.Command("git", "rev-parse", "--abbrev-ref", "HEAD")
	branchOutput, err := branchCmd.Output()
	if err != nil {
		return fmt.Errorf("failed to get current branch: %w", err)
	}
	currentBranch := strings.TrimSpace(string(branchOutput))

	fmt.Printf("  %s ", ui.StyleDim.Render("🌳 Creating git worktrees..."))

	// Create worktree for queen
	queenPath := ".hive/workspaces/queen"
	if err := createWorktree(queenPath, currentBranch, "queen"); err != nil {
		fmt.Printf("%s\n", ui.StyleYellow.Render("⚠️"))
		return err
	}

	// Create worktrees for workers
	for i := 1; i <= workers; i++ {
		workerPath := fmt.Sprintf(".hive/workspaces/drone-%d", i)
		workerName := fmt.Sprintf("drone-%d", i)
		if err := createWorktree(workerPath, currentBranch, workerName); err != nil {
			fmt.Printf("%s\n", ui.StyleYellow.Render("⚠️"))
			return err
		}
	}

	fmt.Printf("%s (%d worktree%s)\n", ui.StyleGreen.Render("✓"), workers+1, pluralize(workers+1))
	return nil
}

// createWorktree creates a single git worktree
func createWorktree(path, branch, agentName string) error {
	agentBranch := fmt.Sprintf("hive/%s", agentName)

	// ALWAYS prune orphaned worktrees first (critical after hive clean)
	pruneCmd := exec.Command("git", "worktree", "prune")
	pruneCmd.Run() // Ignore errors, prune is best-effort

	// Check if worktree already exists and is valid
	if _, err := os.Stat(filepath.Join(path, ".git")); err == nil {
		// Worktree directory exists, verify it's still registered with git
		checkCmd := exec.Command("git", "worktree", "list")
		output, _ := checkCmd.Output()
		if strings.Contains(string(output), path) {
			return nil // Valid worktree already exists
		}
		// Directory exists but worktree is orphaned, remove it
		os.RemoveAll(path)
	}

	// Create parent directory if needed
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		return fmt.Errorf("failed to create directory for %s: %w", agentName, err)
	}

	// Check if the agent branch already exists
	checkBranchCmd := exec.Command("git", "rev-parse", "--verify", agentBranch)
	var checkOut, checkErr bytes.Buffer
	checkBranchCmd.Stdout = &checkOut
	checkBranchCmd.Stderr = &checkErr
	branchExists := checkBranchCmd.Run() == nil

	if branchExists {
		// Branch exists, try to add worktree with existing branch
		cmd := exec.Command("git", "worktree", "add", path, agentBranch)
		var cmdOut, cmdErr bytes.Buffer
		cmd.Stdout = &cmdOut
		cmd.Stderr = &cmdErr
		if err := cmd.Run(); err != nil {
			// If it fails, force delete the branch and recreate from scratch
			deleteCmd := exec.Command("git", "branch", "-D", agentBranch)
			deleteCmd.Run() // Ignore errors
			branchExists = false
		} else {
			return nil // Success!
		}
	}

	// Create new worktree with new branch
	cmd := exec.Command("git", "worktree", "add", "-b", agentBranch, path, branch)
	var cmdOut, cmdErr bytes.Buffer
	cmd.Stdout = &cmdOut
	cmd.Stderr = &cmdErr
	if err := cmd.Run(); err != nil {
		// Show actual git error for debugging
		errMsg := strings.TrimSpace(cmdErr.String())
		if errMsg == "" {
			errMsg = err.Error()
		}
		return fmt.Errorf("failed to create worktree for %s: %s", agentName, errMsg)
	}

	return nil
}

// detectNodeVersion attempts to detect Node.js version from package.json or .nvmrc
func detectNodeVersion() string {
	// Try package.json first
	if data, err := os.ReadFile("package.json"); err == nil {
		var pkg struct {
			Engines struct {
				Node string `json:"node"`
			} `json:"engines"`
		}
		if err := json.Unmarshal(data, &pkg); err == nil && pkg.Engines.Node != "" {
			// Extract major version number
			// Examples: ">=24.0.0" → "24", "^20.5.0" → "20", "24" → "24"
			nodeVersion := pkg.Engines.Node
			// Remove common prefixes
			nodeVersion = strings.TrimPrefix(nodeVersion, ">=")
			nodeVersion = strings.TrimPrefix(nodeVersion, "^")
			nodeVersion = strings.TrimPrefix(nodeVersion, "~")
			nodeVersion = strings.TrimPrefix(nodeVersion, ">")
			nodeVersion = strings.TrimPrefix(nodeVersion, "<")
			nodeVersion = strings.TrimSpace(nodeVersion)

			// Extract first number sequence (major version)
			parts := strings.Split(nodeVersion, ".")
			if len(parts) > 0 {
				// Remove any non-numeric characters from major version
				major := parts[0]
				var digits strings.Builder
				for _, ch := range major {
					if ch >= '0' && ch <= '9' {
						digits.WriteRune(ch)
					}
				}
				if digits.Len() > 0 {
					return digits.String()
				}
			}
		}
	}

	// Try .nvmrc as fallback
	if data, err := os.ReadFile(".nvmrc"); err == nil {
		version := strings.TrimSpace(string(data))
		// Remove 'v' prefix if present (e.g., "v24.0.0" → "24.0.0")
		version = strings.TrimPrefix(version, "v")
		// Extract major version
		parts := strings.Split(version, ".")
		if len(parts) > 0 {
			var digits strings.Builder
			for _, ch := range parts[0] {
				if ch >= '0' && ch <= '9' {
					digits.WriteRune(ch)
				}
			}
			if digits.Len() > 0 {
				return digits.String()
			}
		}
	}

	return "" // Not found, will use default (22)
}

// updateGitignore adds hive-specific entries to .gitignore
func updateGitignore() error {
	entries := []string{
		"",
		"# Hive (multi-agent Claude)",
		".hive/",
	}

	gitignorePath := ".gitignore"
	var content string

	// Read existing .gitignore if it exists
	if data, err := os.ReadFile(gitignorePath); err == nil {
		content = string(data)
	}

	// Check if hive entries already exist
	if strings.Contains(content, ".hive/") {
		return nil // Already configured
	}

	// Append hive entries
	content += strings.Join(entries, "\n") + "\n"

	return os.WriteFile(gitignorePath, []byte(content), 0644)
}

// generateDockerCompose creates docker-compose.yml with the specified number of workers
func generateDockerCompose(workers int) error {
	return generateDockerComposeWithConfig(workers, 6379)
}

// generateDockerComposeWithConfig creates docker-compose.yml with full config options
func generateDockerComposeWithConfig(workers int, redisPort int) error {
	content := compose.GenerateWithOptions(compose.Options{
		WorkerCount: workers,
		RedisPort:   redisPort,
	})
	return os.WriteFile(".hive/docker-compose.yml", []byte(content), 0644)
}
