package cmd

import (
	"fmt"

	"github.com/mbourmaud/hive/internal/ui"
)

// interactiveWizardWithDetection runs the interactive setup wizard
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

// printSuccessMessage displays the success message after initialization
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
