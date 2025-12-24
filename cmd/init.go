package cmd

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/mbourmaud/hive/internal/embed"
	"github.com/spf13/cobra"
	"golang.org/x/term"
)

var (
	flagNonInteractive bool
	flagEmail          string
	flagName           string
	flagToken          string
	flagWorkspace      string
	flagWorkers        int
	flagGitURL         string
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
	initCmd.Flags().StringVar(&flagToken, "token", "", "Claude OAuth token")
	initCmd.Flags().StringVar(&flagWorkspace, "workspace", "my-project", "Workspace name")
	initCmd.Flags().IntVar(&flagWorkers, "workers", 2, "Number of workers to start")
	initCmd.Flags().StringVar(&flagGitURL, "git-url", "", "Git repository URL to clone (optional)")
}

func runInit(cmd *cobra.Command, args []string) error {
	// Check if already initialized
	if fileExists(".hive/.env") {
		return fmt.Errorf(".hive/ already exists. Use 'rm -rf .hive' to reinitialize")
	}

	fmt.Println("🐝 Welcome to HIVE - Multi-Agent Claude System")
	fmt.Println()

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
			"GIT_USER_EMAIL":          flagEmail,
			"GIT_USER_NAME":           flagName,
			"CLAUDE_CODE_OAUTH_TOKEN": flagToken,
			"WORKSPACE_NAME":          flagWorkspace,
			"GIT_REPO_URL":            flagGitURL,
			"PROJECT_TYPE":            projectType,
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
	fmt.Println("📦 Extracting hive files to .hive/...")
	if err := extractHiveFiles(cfg["PROJECT_TYPE"]); err != nil {
		return fmt.Errorf("failed to extract hive files: %w", err)
	}
	fmt.Println("✅ Extracted .hive/")

	// Write .env file
	if err := writeEnvFile(cfg); err != nil {
		return fmt.Errorf("failed to write .hive/.env: %w", err)
	}
	fmt.Println("✅ Created .hive/.env")

	// Write hive.yaml file
	workers := flagWorkers
	if err := writeHiveYAML(cfg["WORKSPACE_NAME"], cfg["GIT_REPO_URL"], workers); err != nil {
		return fmt.Errorf("failed to write hive.yaml: %w", err)
	}
	fmt.Println("✅ Created hive.yaml")

	// Update .gitignore
	if err := updateGitignore(); err != nil {
		fmt.Printf("⚠️  Failed to update .gitignore: %v\n", err)
	} else {
		fmt.Println("✅ Updated .gitignore")
	}

	// Ask for workers count
	if !flagNonInteractive {
		fmt.Println()
		workersStr := promptWithDefault("🚀 Workers to start", "2")
		if w, err := strconv.Atoi(workersStr); err == nil {
			workers = w
		}
	}

	// Create git worktrees for each agent
	if err := createWorktrees(workers); err != nil {
		fmt.Printf("\n⚠️  Failed to create worktrees: %v\n", err)
		fmt.Println("   Agents will use empty workspaces")
	}

	fmt.Printf("\n🐝 Starting Hive with %d workers...\n", workers)
	startCmd := exec.Command("hive", "start", strconv.Itoa(workers))
	startCmd.Stdout = os.Stdout
	startCmd.Stderr = os.Stderr

	if err := startCmd.Run(); err != nil {
		fmt.Println("\n⚠️  Failed to start Hive automatically")
		fmt.Printf("   Run manually: hive start %d\n", workers)
	}

	// Print success message
	printSuccessMessage(workers)

	return nil
}

func interactiveWizard() (map[string]string, error) {
	config := make(map[string]string)

	// Git Configuration
	fmt.Println("📧 Git Configuration")
	config["GIT_USER_EMAIL"] = promptRequired("  Email", validateEmail)
	config["GIT_USER_NAME"] = promptRequired("  Name", nil)
	fmt.Println()

	// Claude Authentication
	fmt.Println("🔑 Claude Authentication")
	fmt.Println("  Get your token: claude setup-token")
	config["CLAUDE_CODE_OAUTH_TOKEN"] = promptSecure("  OAuth Token")
	fmt.Println()

	// Project Setup
	fmt.Println("📂 Project Setup")
	config["WORKSPACE_NAME"] = promptWithDefault("  Workspace name", "my-project")
	config["GIT_REPO_URL"] = promptOptional("  Git repo URL (optional)", "")
	fmt.Println()

	return config, nil
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

	// Use detected values
	cfg["GIT_USER_EMAIL"] = email
	cfg["GIT_USER_NAME"] = name
	cfg["GIT_REPO_URL"] = repoURL
	cfg["WORKSPACE_NAME"] = workspaceName
	cfg["PROJECT_TYPE"] = projectType

	// Ask for missing git info
	if email == "" {
		fmt.Println("📧 Git Configuration")
		cfg["GIT_USER_EMAIL"] = promptRequired("  Email", validateEmail)
	}
	if name == "" {
		if email == "" {
			cfg["GIT_USER_NAME"] = promptRequired("  Name", nil)
		} else {
			fmt.Println("📧 Git Configuration")
			cfg["GIT_USER_NAME"] = promptRequired("  Name", nil)
		}
	}

	// Handle Claude token
	if claudeToken != "" {
		fmt.Println("🔑 Claude token detected from ~/.claude")
		cfg["CLAUDE_CODE_OAUTH_TOKEN"] = claudeToken
	} else {
		fmt.Println("🔑 Claude Authentication")
		fmt.Println("   Get your token: claude /auth")
		cfg["CLAUDE_CODE_OAUTH_TOKEN"] = promptSecure("   OAuth Token")
	}
	fmt.Println()

	return cfg, nil
}

func validateFlags() error {
	if flagEmail == "" {
		return fmt.Errorf("--email is required in non-interactive mode")
	}
	if flagName == "" {
		return fmt.Errorf("--name is required in non-interactive mode")
	}
	if flagToken == "" {
		return fmt.Errorf("--token is required in non-interactive mode")
	}
	if err := validateEmail(flagEmail); err != nil {
		return err
	}
	return nil
}

func validateEmail(email string) error {
	re := regexp.MustCompile(`^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$`)
	if !re.MatchString(email) {
		return fmt.Errorf("invalid email format: %s", email)
	}
	return nil
}

func promptRequired(label string, validator func(string) error) string {
	reader := bufio.NewReader(os.Stdin)
	for {
		fmt.Printf("%s: ", label)
		input, _ := reader.ReadString('\n') // nolint:errcheck
		input = strings.TrimSpace(input)

		if input == "" {
			fmt.Println("  ⚠️  This field is required")
			continue
		}

		if validator != nil {
			if err := validator(input); err != nil {
				fmt.Printf("  ⚠️  %s\n", err)
				continue
			}
		}

		return input
	}
}

func promptWithDefault(label, defaultValue string) string {
	reader := bufio.NewReader(os.Stdin)
	fmt.Printf("%s (default: %s): ", label, defaultValue)
	input, _ := reader.ReadString('\n') // nolint:errcheck
	input = strings.TrimSpace(input)

	if input == "" {
		return defaultValue
	}
	return input
}

func promptOptional(label, defaultValue string) string {
	reader := bufio.NewReader(os.Stdin)
	fmt.Printf("%s: ", label)
	input, _ := reader.ReadString('\n') // nolint:errcheck
	input = strings.TrimSpace(input)

	if input == "" {
		return defaultValue
	}
	return input
}

// promptSecure prompts for sensitive input (like tokens/passwords) with masked input
func promptSecure(label string) string {
	for {
		fmt.Printf("%s: ", label)

		// Read password (masked input)
		bytePassword, err := term.ReadPassword(int(os.Stdin.Fd()))
		fmt.Println() // Print newline after password input

		if err != nil {
			fmt.Printf("  ⚠️  Error reading input: %v\n", err)
			continue
		}

		input := strings.TrimSpace(string(bytePassword))

		if input == "" {
			fmt.Println("  ⚠️  This field is required")
			continue
		}

		return input
	}
}

func writeEnvFile(cfg map[string]string) error {
	// Read .env.example from embedded files
	template, err := embed.GetFile(".env.example")
	if err != nil {
		// Fallback: generate minimal .env if template not found
		return writeMinimalEnvFile(cfg)
	}

	content := string(template)

	// Replace placeholders
	content = strings.ReplaceAll(content, "your.email@example.com", cfg["GIT_USER_EMAIL"])
	content = strings.ReplaceAll(content, "Your Name", cfg["GIT_USER_NAME"])
	content = strings.ReplaceAll(content, "your_oauth_token_here", cfg["CLAUDE_CODE_OAUTH_TOKEN"])
	content = strings.ReplaceAll(content, "my-project", cfg["WORKSPACE_NAME"])

	// Add git URL if provided
	if cfg["GIT_REPO_URL"] != "" {
		content = strings.ReplaceAll(content, "# GIT_REPO_URL=https://github.com/user/repo.git",
			"GIT_REPO_URL="+cfg["GIT_REPO_URL"])
	}

	// Set Dockerfile based on project type
	if cfg["PROJECT_TYPE"] != "" && cfg["PROJECT_TYPE"] != "node" {
		content = strings.ReplaceAll(content, "HIVE_DOCKERFILE=docker/Dockerfile.node",
			"HIVE_DOCKERFILE=docker/Dockerfile."+cfg["PROJECT_TYPE"])
	}

	// Write to .hive/.env
	return os.WriteFile(".hive/.env", []byte(content), 0600)
}

// writeMinimalEnvFile generates a minimal .env file without template
func writeMinimalEnvFile(cfg map[string]string) error {
	dockerfile := "docker/Dockerfile.node"
	if cfg["PROJECT_TYPE"] != "" {
		dockerfile = "docker/Dockerfile." + cfg["PROJECT_TYPE"]
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
`,
		cfg["GIT_USER_EMAIL"],
		cfg["GIT_USER_NAME"],
		cfg["CLAUDE_CODE_OAUTH_TOKEN"],
		cfg["WORKSPACE_NAME"],
		cfg["GIT_REPO_URL"],
		dockerfile,
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

func writeHiveYAML(workspace, gitURL string, workers int) error {
	cfg := config.Default()

	// Update with user values
	cfg.Workspace.Name = workspace
	if gitURL != "" {
		cfg.Workspace.GitURL = gitURL
	}
	cfg.Agents.Workers.Count = workers

	return cfg.Save("hive.yaml")
}

func printSuccessMessage(workers int) {
	fmt.Println("\n✅ Setup complete!")
	fmt.Printf("  Hive is now running with %d workers.\n\n", workers)
	fmt.Println("  Next steps:")
	fmt.Println("    hive connect queen  # Connect to orchestrator")
	fmt.Println("    hive connect 1      # Connect to worker 1")
	fmt.Println("    hive status         # Check status")
	fmt.Println("  Need help? https://github.com/mbourmaud/hive")
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
func extractHiveFiles(projectType string) error {
	hiveDir := ".hive"

	// Create .hive directory
	if err := os.MkdirAll(hiveDir, 0755); err != nil {
		return fmt.Errorf("failed to create .hive directory: %w", err)
	}

	// Extract docker-compose.yml
	if err := embed.ExtractFile("docker-compose.yml", filepath.Join(hiveDir, "docker-compose.yml")); err != nil {
		return fmt.Errorf("failed to extract docker-compose.yml: %w", err)
	}

	// Extract entrypoint.sh
	if err := embed.ExtractFile("entrypoint.sh", filepath.Join(hiveDir, "entrypoint.sh")); err != nil {
		return fmt.Errorf("failed to extract entrypoint.sh: %w", err)
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

	fmt.Println("\n📦 Creating git worktrees for agents...")

	// Create worktree for queen
	queenPath := ".hive/workspaces/queen"
	if err := createWorktree(queenPath, currentBranch, "queen"); err != nil {
		return err
	}

	// Create worktrees for workers
	for i := 1; i <= workers; i++ {
		workerPath := fmt.Sprintf(".hive/workspaces/drone-%d", i)
		workerName := fmt.Sprintf("drone-%d", i)
		if err := createWorktree(workerPath, currentBranch, workerName); err != nil {
			return err
		}
	}

	fmt.Println("✅ Created worktrees for all agents")
	return nil
}

// createWorktree creates a single git worktree
func createWorktree(path, branch, agentName string) error {
	// Check if worktree already exists
	if _, err := os.Stat(filepath.Join(path, ".git")); err == nil {
		fmt.Printf("  ✓ Worktree for %s already exists\n", agentName)
		return nil
	}

	// Create parent directory if needed
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		return fmt.Errorf("failed to create directory for %s: %w", agentName, err)
	}

	// Create detached worktree (allows multiple worktrees on same branch)
	cmd := exec.Command("git", "worktree", "add", "--detach", path, branch)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to create worktree for %s: %w", agentName, err)
	}

	fmt.Printf("  ✓ Created worktree for %s\n", agentName)
	return nil
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
