package cmd

import (
	"fmt"
	"strings"
)

// validateFlags validates non-interactive mode flags
func validateFlags() error {
	// In non-interactive mode, we need valid inputs
	if flagEmail == "" {
		return fmt.Errorf("--email is required in non-interactive mode")
	}
	if err := validateEmail(flagEmail); err != nil {
		return fmt.Errorf("invalid --email: %w", err)
	}

	if flagName == "" {
		return fmt.Errorf("--name is required in non-interactive mode")
	}

	// For auth, we need either token or API key depending on backend
	switch flagAuthBackend {
	case "cli":
		if flagToken == "" {
			return fmt.Errorf("--token is required for cli auth backend")
		}
	case "api":
		if flagApiKey == "" {
			return fmt.Errorf("--api-key is required for api auth backend")
		}
	case "bedrock":
		// Bedrock uses AWS credentials, no token needed
	default:
		return fmt.Errorf("--auth must be one of: cli, api, bedrock")
	}

	if flagWorkers < 0 || flagWorkers > 10 {
		return fmt.Errorf("--workers must be between 0 and 10")
	}

	return nil
}

// validateEmail validates an email address format
func validateEmail(email string) error {
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
