package cmd

import (
	"bufio"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"

	"github.com/mbourmaud/hive/internal/config"
	"github.com/spf13/cobra"
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
	if fileExists(".env") {
		return fmt.Errorf(".env already exists. Use 'rm .env' to reinitialize or edit it manually")
	}

	fmt.Println("🐝 Welcome to HIVE - Multi-Agent Claude System\n")

	var config map[string]string

	if flagNonInteractive {
		// Non-interactive mode: use flags
		if err := validateFlags(); err != nil {
			return err
		}
		config = map[string]string{
			"GIT_USER_EMAIL":       flagEmail,
			"GIT_USER_NAME":        flagName,
			"CLAUDE_CODE_OAUTH_TOKEN": flagToken,
			"WORKSPACE_NAME":       flagWorkspace,
			"GIT_REPO_URL":         flagGitURL,
		}
	} else {
		// Interactive mode
		fmt.Println("Let's set up your hive in 3 steps:\n")
		var err error
		config, err = interactiveWizard()
		if err != nil {
			return err
		}
	}

	// Write .env file
	if err := writeEnvFile(config); err != nil {
		return fmt.Errorf("failed to write .env: %w", err)
	}
	fmt.Println("✅ Created .env file")

	// Write hive.yaml file
	if err := writeHiveYAML(config["WORKSPACE_NAME"], config["GIT_REPO_URL"], flagWorkers); err != nil {
		return fmt.Errorf("failed to write hive.yaml: %w", err)
	}
	fmt.Println("✅ Created hive.yaml\n")

	// Build and install CLI
	fmt.Println("🔨 Building and installing CLI...")
	buildCmd := exec.Command("make", "install")
	buildCmd.Stdout = os.Stdout
	buildCmd.Stderr = os.Stderr
	if err := buildCmd.Run(); err != nil {
		fmt.Println("⚠️  Failed to install CLI (you may need sudo)")
		fmt.Println("   You can manually run: make install")
	} else {
		fmt.Println("✅ CLI installed\n")
	}

	// Start Hive
	workers := flagWorkers
	if !flagNonInteractive {
		fmt.Print("\n🚀 Ready to start!\n\n")
		workersStr := promptWithDefault("  Workers to start", "2")
		workers, _ = strconv.Atoi(workersStr)
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
	config["CLAUDE_CODE_OAUTH_TOKEN"] = promptRequired("  OAuth Token", nil)
	fmt.Println()

	// Project Setup
	fmt.Println("📂 Project Setup")
	config["WORKSPACE_NAME"] = promptWithDefault("  Workspace name", "my-project")
	config["GIT_REPO_URL"] = promptOptional("  Git repo URL (optional)", "")
	fmt.Println()

	return config, nil
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
		input, _ := reader.ReadString('\n')
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
	input, _ := reader.ReadString('\n')
	input = strings.TrimSpace(input)

	if input == "" {
		return defaultValue
	}
	return input
}

func promptOptional(label, defaultValue string) string {
	reader := bufio.NewReader(os.Stdin)
	fmt.Printf("%s: ", label)
	input, _ := reader.ReadString('\n')
	input = strings.TrimSpace(input)

	if input == "" {
		return defaultValue
	}
	return input
}

func writeEnvFile(config map[string]string) error {
	// Read .env.example as template
	templatePath := ".env.example"
	template, err := os.ReadFile(templatePath)
	if err != nil {
		return err
	}

	content := string(template)

	// Replace placeholders
	content = strings.ReplaceAll(content, "your.email@example.com", config["GIT_USER_EMAIL"])
	content = strings.ReplaceAll(content, "Your Name", config["GIT_USER_NAME"])
	content = strings.ReplaceAll(content, "your_oauth_token_here", config["CLAUDE_CODE_OAUTH_TOKEN"])
	content = strings.ReplaceAll(content, "my-project", config["WORKSPACE_NAME"])

	// Add git URL if provided
	if config["GIT_REPO_URL"] != "" {
		content = strings.ReplaceAll(content, "# GIT_REPO_URL=https://github.com/user/repo.git",
			"GIT_REPO_URL="+config["GIT_REPO_URL"])
	}

	return os.WriteFile(".env", []byte(content), 0600)
}

func fileExists(path string) bool {
	absPath, _ := filepath.Abs(path)
	_, err := os.Stat(absPath)
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
	fmt.Println("\n✅ Setup complete!\n")
	fmt.Printf("  Hive is now running with %d workers.\n\n", workers)
	fmt.Println("  Next steps:")
	fmt.Println("    hive connect queen  # Connect to orchestrator")
	fmt.Println("    hive connect 1      # Connect to worker 1")
	fmt.Println("    hive status         # Check status\n")
	fmt.Println("  Need help? https://github.com/mbourmaud/hive\n")
}
