package cmd

import (
	"bytes"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

// setupTestRepo creates a test git repository
func setupTestRepo(t *testing.T) (string, string, func()) {
	t.Helper()

	// Create temp directory
	tmpDir, err := os.MkdirTemp("", "hive-test-*")
	if err != nil {
		t.Fatalf("failed to create temp dir: %v", err)
	}

	// Change to temp directory
	origDir, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get current dir: %v", err)
	}
	if err := os.Chdir(tmpDir); err != nil {
		t.Fatalf("failed to chdir: %v", err)
	}

	// Initialize git repo
	cmds := [][]string{
		{"git", "init"},
		{"git", "config", "user.email", "test@example.com"},
		{"git", "config", "user.name", "Test User"},
		{"git", "commit", "--allow-empty", "-m", "initial commit"},
	}

	for _, cmdArgs := range cmds {
		cmd := exec.Command(cmdArgs[0], cmdArgs[1:]...)
		if err := cmd.Run(); err != nil {
			cleanup := func() {
				os.Chdir(origDir)
				os.RemoveAll(tmpDir)
			}
			cleanup()
			t.Fatalf("failed to run %v: %v", cmdArgs, err)
		}
	}

	// Detect default branch name (main or master)
	branchCmd := exec.Command("git", "rev-parse", "--abbrev-ref", "HEAD")
	branchOut, err := branchCmd.Output()
	if err != nil {
		cleanup := func() {
			os.Chdir(origDir)
			os.RemoveAll(tmpDir)
		}
		cleanup()
		t.Fatalf("failed to detect default branch: %v", err)
	}
	defaultBranch := strings.TrimSpace(string(branchOut))

	cleanup := func() {
		os.Chdir(origDir)
		os.RemoveAll(tmpDir)
	}

	return tmpDir, defaultBranch, cleanup
}

// TestCreateWorktree_FreshRepo tests creating a worktree in a fresh repo
func TestCreateWorktree_FreshRepo(t *testing.T) {
	_, defaultBranch, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreePath := filepath.Join(".hive", "workspaces", "queen")
	err := createWorktree(worktreePath, defaultBranch, "queen")
	if err != nil {
		t.Fatalf("createWorktree failed: %v", err)
	}

	// Verify worktree exists
	if _, err := os.Stat(filepath.Join(worktreePath, ".git")); os.IsNotExist(err) {
		t.Error("worktree .git file not created")
	}

	// Verify branch exists
	cmd := exec.Command("git", "branch", "--list", "hive/queen")
	output, err := cmd.Output()
	if err != nil || !strings.Contains(string(output), "hive/queen") {
		t.Error("hive/queen branch not created")
	}
}

// TestCreateWorktree_AfterClean simulates state after hive clean
func TestCreateWorktree_AfterClean(t *testing.T) {
	_, defaultBranch, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreePath := filepath.Join(".hive", "workspaces", "queen")

	// Create initial worktree
	if err := createWorktree(worktreePath, defaultBranch, "queen"); err != nil {
		t.Fatalf("initial createWorktree failed: %v", err)
	}

	// Simulate hive clean: remove directory but leave branch
	os.RemoveAll(".hive")

	// Try to create worktree again (should succeed)
	err := createWorktree(worktreePath, defaultBranch, "queen")
	if err != nil {
		t.Fatalf("createWorktree after clean failed: %v", err)
	}

	// Verify worktree exists
	if _, err := os.Stat(filepath.Join(worktreePath, ".git")); os.IsNotExist(err) {
		t.Error("worktree not recreated after clean")
	}
}

// TestCreateWorktree_OrphanedDirectory simulates orphaned worktree directory
func TestCreateWorktree_OrphanedDirectory(t *testing.T) {
	_, defaultBranch, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreePath := filepath.Join(".hive", "workspaces", "queen")

	// Create initial worktree
	if err := createWorktree(worktreePath, defaultBranch, "queen"); err != nil {
		t.Fatalf("initial createWorktree failed: %v", err)
	}

	// Remove worktree from git but leave directory
	cmd := exec.Command("git", "worktree", "remove", worktreePath, "--force")
	cmd.Run() // Ignore errors

	// Try to create worktree again (should handle orphaned directory)
	err := createWorktree(worktreePath, defaultBranch, "queen")
	if err != nil {
		t.Fatalf("createWorktree with orphaned directory failed: %v", err)
	}

	// Verify worktree exists and is valid
	listCmd := exec.Command("git", "worktree", "list")
	output, err := listCmd.Output()
	if err != nil || !strings.Contains(string(output), worktreePath) {
		t.Error("worktree not properly recreated")
	}
}

// TestCreateWorktree_ExistingBranch tests creating worktree when branch already exists
func TestCreateWorktree_ExistingBranch(t *testing.T) {
	_, defaultBranch, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreePath := filepath.Join(".hive", "workspaces", "queen")

	// Create branch manually
	cmd := exec.Command("git", "branch", "hive/queen")
	if err := cmd.Run(); err != nil {
		t.Fatalf("failed to create branch: %v", err)
	}

	// Try to create worktree (should use existing branch)
	err := createWorktree(worktreePath, defaultBranch, "queen")
	if err != nil {
		t.Fatalf("createWorktree with existing branch failed: %v", err)
	}

	// Verify worktree exists
	if _, err := os.Stat(filepath.Join(worktreePath, ".git")); os.IsNotExist(err) {
		t.Error("worktree not created with existing branch")
	}
}

// TestCreateWorktree_MultipleWorkers tests creating multiple worktrees
func TestCreateWorktree_MultipleWorkers(t *testing.T) {
	_, defaultBranch, cleanup := setupTestRepo(t)
	defer cleanup()

	agents := []string{"queen", "drone-1", "drone-2"}

	for _, agent := range agents {
		worktreePath := filepath.Join(".hive", "workspaces", agent)
		err := createWorktree(worktreePath, defaultBranch, agent)
		if err != nil {
			t.Fatalf("createWorktree failed for %s: %v", agent, err)
		}
	}

	// Verify all worktrees exist
	listCmd := exec.Command("git", "worktree", "list")
	output, err := listCmd.Output()
	if err != nil {
		t.Fatalf("failed to list worktrees: %v", err)
	}

	for _, agent := range agents {
		if !strings.Contains(string(output), agent) {
			t.Errorf("worktree for %s not found in list", agent)
		}
	}
}

// TestCreateWorktree_Idempotent tests that calling createWorktree twice is safe
func TestCreateWorktree_Idempotent(t *testing.T) {
	_, defaultBranch, cleanup := setupTestRepo(t)
	defer cleanup()

	worktreePath := filepath.Join(".hive", "workspaces", "queen")

	// Create worktree
	if err := createWorktree(worktreePath, defaultBranch, "queen"); err != nil {
		t.Fatalf("first createWorktree failed: %v", err)
	}

	// Call again (should be no-op)
	if err := createWorktree(worktreePath, defaultBranch, "queen"); err != nil {
		t.Fatalf("second createWorktree failed: %v", err)
	}

	// Verify only one worktree exists
	listCmd := exec.Command("git", "worktree", "list")
	output, err := listCmd.Output()
	if err != nil {
		t.Fatalf("failed to list worktrees: %v", err)
	}

	// Count occurrences of worktree path
	count := strings.Count(string(output), worktreePath)
	if count != 1 {
		t.Errorf("expected 1 worktree, found %d", count)
	}
}

// TestRunInit_AlreadyInitialized tests runInit when .hive already exists
func TestRunInit_AlreadyInitialized(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Create .hive/.env to simulate already initialized
	os.MkdirAll(".hive", 0755)
	os.WriteFile(".hive/.env", []byte("TEST=value"), 0644)

	err := runInit(nil, nil)
	if err == nil {
		t.Error("runInit() should error when .hive/.env already exists")
	}
	if !strings.Contains(err.Error(), "already exists") {
		t.Errorf("runInit() error = %q, want 'already exists'", err.Error())
	}
}

// TestRunInit_NonInteractive tests runInit in non-interactive mode
func TestRunInit_NonInteractive(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Set up flags for non-interactive mode
	flagNonInteractive = true
	flagEmail = "test@example.com"
	flagName = "Test User"
	flagToken = "test-oauth-token"
	flagAuthBackend = "cli"
	flagWorkspace = "test-project"
	flagWorkers = 2
	flagSkipStart = true
	defer func() {
		flagNonInteractive = false
		flagEmail = ""
		flagName = ""
		flagToken = ""
		flagAuthBackend = "cli"
		flagWorkspace = "my-project"
		flagWorkers = 2
		flagSkipStart = false
	}()

	// Initialize git repo (required for worktrees)
	exec.Command("git", "init").Run()
	exec.Command("git", "config", "user.email", "test@example.com").Run()
	exec.Command("git", "config", "user.name", "Test User").Run()
	exec.Command("git", "commit", "--allow-empty", "-m", "initial commit").Run()

	// Run init - should succeed until it tries to run docker compose
	err := runInit(nil, nil)
	// The function will likely fail when trying to build docker images
	// but we test that it gets past the initial setup

	// Check if .hive directory was created
	if _, statErr := os.Stat(".hive"); os.IsNotExist(statErr) {
		t.Error("runInit() should create .hive directory")
	}

	// Check if .env file was created
	if _, statErr := os.Stat(".hive/.env"); os.IsNotExist(statErr) {
		t.Error("runInit() should create .hive/.env file")
	}

	// If error is about docker, that's expected
	if err != nil && !strings.Contains(err.Error(), "docker") && !strings.Contains(err.Error(), "worktree") && !strings.Contains(err.Error(), "compose") {
		t.Logf("runInit() error = %v (may be expected)", err)
	}
}

// TestValidateFlags tests flag validation logic
func TestValidateFlags(t *testing.T) {
	tests := []struct {
		name        string
		email       string
		userName    string
		token       string
		apiKey      string
		authBackend string
		wantErr     bool
		errSubstr   string
	}{
		{
			name:        "cli backend all flags provided",
			email:       "user@example.com",
			userName:    "Test User",
			token:       "test-token",
			authBackend: "cli",
			wantErr:     false,
		},
		{
			name:      "missing email",
			email:     "",
			userName:  "Test User",
			token:     "test-token",
			wantErr:   true,
			errSubstr: "--email is required",
		},
		{
			name:      "missing name",
			email:     "user@example.com",
			userName:  "",
			token:     "test-token",
			wantErr:   true,
			errSubstr: "--name is required",
		},
		{
			name:        "cli backend missing token",
			email:       "user@example.com",
			userName:    "Test User",
			token:       "",
			authBackend: "cli",
			wantErr:     true,
			errSubstr:   "--token is required",
		},
		{
			name:      "invalid email format",
			email:     "invalid-email",
			userName:  "Test User",
			token:     "test-token",
			wantErr:   true,
			errSubstr: "invalid email",
		},
		{
			name:        "api backend with api key",
			email:       "user@example.com",
			userName:    "Test User",
			apiKey:      "sk-ant-api01-xxx",
			authBackend: "api",
			wantErr:     false,
		},
		{
			name:        "api backend missing api key",
			email:       "user@example.com",
			userName:    "Test User",
			apiKey:      "",
			authBackend: "api",
			wantErr:     true,
			errSubstr:   "--api-key is required",
		},
		{
			name:        "bedrock backend no credentials needed",
			email:       "user@example.com",
			userName:    "Test User",
			authBackend: "bedrock",
			wantErr:     false,
		},
		{
			name:        "invalid auth backend",
			email:       "user@example.com",
			userName:    "Test User",
			authBackend: "invalid",
			wantErr:     true,
			errSubstr:   "--auth must be one of",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Set flags
			flagEmail = tt.email
			flagName = tt.userName
			flagToken = tt.token
			flagApiKey = tt.apiKey
			if tt.authBackend != "" {
				flagAuthBackend = tt.authBackend
			} else {
				flagAuthBackend = "cli" // default
			}

			// Execute
			err := validateFlags()

			// Verify
			if (err != nil) != tt.wantErr {
				t.Errorf("validateFlags() error = %v, wantErr %v", err, tt.wantErr)
				return
			}

			if tt.wantErr && err != nil && tt.errSubstr != "" {
				if !strings.Contains(err.Error(), tt.errSubstr) {
					t.Errorf("validateFlags() error = %q, want to contain %q", err.Error(), tt.errSubstr)
				}
			}
		})
	}
}

// TestDetectProjectType tests project type detection
func TestDetectProjectType(t *testing.T) {
	tests := []struct {
		name         string
		createFiles  []string
		expectedType string
	}{
		{"node project", []string{"package.json"}, "node"},
		{"go project", []string{"go.mod"}, "go"},
		{"python with pyproject", []string{"pyproject.toml"}, "python"},
		{"python with requirements", []string{"requirements.txt"}, "python"},
		{"rust project", []string{"Cargo.toml"}, "rust"},
		{"minimal project", []string{}, "minimal"},
		{"node priority", []string{"package.json", "go.mod"}, "node"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tmpDir := t.TempDir()
			oldWd, _ := os.Getwd()
			os.Chdir(tmpDir)
			defer os.Chdir(oldWd)

			// Create test files
			for _, file := range tt.createFiles {
				os.WriteFile(file, []byte("test"), 0644)
			}

			got := detectProjectType()
			if got != tt.expectedType {
				t.Errorf("detectProjectType() = %q, want %q", got, tt.expectedType)
			}
		})
	}
}

// TestDetectNodeVersion tests Node.js version detection
func TestDetectNodeVersion(t *testing.T) {
	tests := []struct {
		name            string
		packageJSON     string
		nvmrc           string
		expectedVersion string
	}{
		{"from package.json >=", `{"engines":{"node":">=24.0.0"}}`, "", "24"},
		{"from package.json caret", `{"engines":{"node":"^20.5.0"}}`, "", "20"},
		{"from package.json tilde", `{"engines":{"node":"~18.12.0"}}`, "", "18"},
		{"from package.json plain", `{"engines":{"node":"22"}}`, "", "22"},
		{"from .nvmrc with v", "", "v24.0.0", "24"},
		{"from .nvmrc without v", "", "20.5.1", "20"},
		{"from .nvmrc major only", "", "22", "22"},
		{"package.json priority", `{"engines":{"node":"24"}}`, "20", "24"},
		{"no version found", "", "", ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tmpDir := t.TempDir()
			oldWd, _ := os.Getwd()
			os.Chdir(tmpDir)
			defer os.Chdir(oldWd)

			if tt.packageJSON != "" {
				os.WriteFile("package.json", []byte(tt.packageJSON), 0644)
			}
			if tt.nvmrc != "" {
				os.WriteFile(".nvmrc", []byte(tt.nvmrc), 0644)
			}

			got := detectNodeVersion()
			if got != tt.expectedVersion {
				t.Errorf("detectNodeVersion() = %q, want %q", got, tt.expectedVersion)
			}
		})
	}
}

// TestUpdateGitignore tests .gitignore update logic
func TestUpdateGitignore(t *testing.T) {
	tests := []struct {
		name            string
		existingContent string
		expectHiveEntry bool
		shouldModify    bool
	}{
		{"create new", "", true, true},
		{"add to existing", "node_modules/\n*.log\n", true, true},
		{"already has entry", "node_modules/\n.hive/\n", true, false},
		{"already has comment", "# Hive (multi-agent Claude)\n.hive/\n", true, false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tmpDir := t.TempDir()
			oldWd, _ := os.Getwd()
			os.Chdir(tmpDir)
			defer os.Chdir(oldWd)

			gitignorePath := ".gitignore"
			originalContent := tt.existingContent
			if tt.existingContent != "" {
				os.WriteFile(gitignorePath, []byte(tt.existingContent), 0644)
			}

			err := updateGitignore()
			if err != nil {
				t.Fatalf("updateGitignore() error = %v", err)
			}

			content, _ := os.ReadFile(gitignorePath)
			contentStr := string(content)

			if tt.expectHiveEntry && !strings.Contains(contentStr, ".hive/") {
				t.Errorf("updateGitignore() did not add .hive/ entry")
			}

			if !tt.shouldModify && contentStr != originalContent {
				t.Errorf("updateGitignore() modified content when it shouldn't")
			}
		})
	}
}

// TestDetectClaudeToken tests Claude token detection
func TestDetectClaudeToken(t *testing.T) {
	tests := []struct {
		name          string
		setupFunc     func(tmpHome string)
		envToken      string
		expectedToken string
	}{
		{
			name: "from settings.json",
			setupFunc: func(tmpHome string) {
				claudeDir := filepath.Join(tmpHome, ".claude")
				os.MkdirAll(claudeDir, 0755)
				settingsJSON := `{"oauthAccount":{"accessToken":"token-from-settings"}}`
				os.WriteFile(filepath.Join(claudeDir, "settings.json"), []byte(settingsJSON), 0644)
			},
			expectedToken: "token-from-settings",
		},
		{
			name: "from environment variable",
			setupFunc: func(tmpHome string) {
				// Don't create settings.json
			},
			envToken:      "token-from-env",
			expectedToken: "token-from-env",
		},
		{
			name: "settings.json priority",
			setupFunc: func(tmpHome string) {
				claudeDir := filepath.Join(tmpHome, ".claude")
				os.MkdirAll(claudeDir, 0755)
				settingsJSON := `{"oauthAccount":{"accessToken":"token-from-settings"}}`
				os.WriteFile(filepath.Join(claudeDir, "settings.json"), []byte(settingsJSON), 0644)
			},
			envToken:      "token-from-env",
			expectedToken: "token-from-settings",
		},
		{
			name: "no token found",
			setupFunc: func(tmpHome string) {
				// Don't create anything
			},
			expectedToken: "",
		},
		{
			name: "invalid JSON",
			setupFunc: func(tmpHome string) {
				claudeDir := filepath.Join(tmpHome, ".claude")
				os.MkdirAll(claudeDir, 0755)
				os.WriteFile(filepath.Join(claudeDir, "settings.json"), []byte("invalid"), 0644)
			},
			expectedToken: "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tmpHome := t.TempDir()
			oldHome := os.Getenv("HOME")
			os.Setenv("HOME", tmpHome)
			defer os.Setenv("HOME", oldHome)

			if tt.envToken != "" {
				oldToken := os.Getenv("CLAUDE_CODE_OAUTH_TOKEN")
				os.Setenv("CLAUDE_CODE_OAUTH_TOKEN", tt.envToken)
				defer func() {
					if oldToken != "" {
						os.Setenv("CLAUDE_CODE_OAUTH_TOKEN", oldToken)
					} else {
						os.Unsetenv("CLAUDE_CODE_OAUTH_TOKEN")
					}
				}()
			} else {
				os.Unsetenv("CLAUDE_CODE_OAUTH_TOKEN")
			}

			tt.setupFunc(tmpHome)

			got := detectClaudeToken()
			if got != tt.expectedToken {
				t.Errorf("detectClaudeToken() = %q, want %q", got, tt.expectedToken)
			}
		})
	}
}

// TestWriteMinimalEnvFile tests minimal .env file generation
func TestWriteMinimalEnvFile(t *testing.T) {
	tests := []struct {
		name        string
		cfg         map[string]string
		checkFunc   func(content string) error
	}{
		{
			name: "node project",
			cfg: map[string]string{
				"GIT_USER_EMAIL":          "test@example.com",
				"GIT_USER_NAME":           "Test User",
				"CLAUDE_CODE_OAUTH_TOKEN": "test-token",
				"WORKSPACE_NAME":          "test-workspace",
				"GIT_REPO_URL":            "https://github.com/test/repo.git",
				"PROJECT_TYPE":            "node",
			},
			checkFunc: func(content string) error {
				required := []string{
					"test@example.com",
					"Test User",
					"test-token",
					"test-workspace",
					"https://github.com/test/repo.git",
					"docker/Dockerfile.node",
				}
				for _, req := range required {
					if !strings.Contains(content, req) {
						t.Errorf("missing required content: %s", req)
						return nil
					}
				}
				return nil
			},
		},
		{
			name: "go project",
			cfg: map[string]string{
				"GIT_USER_EMAIL":          "test@example.com",
				"GIT_USER_NAME":           "Test User",
				"CLAUDE_CODE_OAUTH_TOKEN": "test-token",
				"WORKSPACE_NAME":          "test-workspace",
				"GIT_REPO_URL":            "",
				"PROJECT_TYPE":            "go",
			},
			checkFunc: func(content string) error {
				if !strings.Contains(content, "docker/Dockerfile.go") {
					t.Errorf("should use Dockerfile.go for Go project")
				}
				return nil
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			tmpDir := t.TempDir()
			oldWd, _ := os.Getwd()
			os.Chdir(tmpDir)
			defer os.Chdir(oldWd)

			os.MkdirAll(".hive", 0755)

			err := writeMinimalEnvFile(tt.cfg)
			if err != nil {
				t.Fatalf("writeMinimalEnvFile() error = %v", err)
			}

			content, readErr := os.ReadFile(".hive/.env")
			if readErr != nil {
				t.Fatalf("Failed to read .hive/.env: %v", readErr)
			}

			if tt.checkFunc != nil {
				tt.checkFunc(string(content))
			}

			// Verify file permissions
			info, _ := os.Stat(".hive/.env")
			if info.Mode().Perm() != 0600 {
				t.Errorf("file permissions = %v, want 0600", info.Mode().Perm())
			}
		})
	}
}

// TestWriteEnvFile tests the full writeEnvFile function with template
func TestWriteEnvFile(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Create .hive directory
	os.MkdirAll(".hive", 0755)

	cfg := map[string]string{
		"GIT_USER_EMAIL":          "test@example.com",
		"GIT_USER_NAME":           "Test User",
		"CLAUDE_CODE_OAUTH_TOKEN": "test-token-123",
		"WORKSPACE_NAME":          "my-workspace",
		"GIT_REPO_URL":            "https://github.com/test/repo.git",
		"PROJECT_TYPE":            "node",
		"WORKER_MODE":             "interactive",
	}

	err := writeEnvFile(cfg, 2)
	if err != nil {
		t.Fatalf("writeEnvFile() error = %v", err)
	}

	// Verify file was created
	content, err := os.ReadFile(".hive/.env")
	if err != nil {
		t.Fatalf("Failed to read .hive/.env: %v", err)
	}

	// Check for required values (only secrets now in .env)
	contentStr := string(content)
	required := []string{
		"test-token-123",          // OAuth token
		"HIVE_CLAUDE_BACKEND=cli", // Backend type
	}

	for _, req := range required {
		if !strings.Contains(contentStr, req) {
			t.Errorf("writeEnvFile() missing %q in output", req)
		}
	}

	// These should NOT be in .env anymore (they're in hive.yaml)
	notAllowed := []string{
		"GIT_USER_EMAIL",
		"WORKSPACE_NAME",
		"HIVE_DOCKERFILE",
		"QUEEN_MODEL",
	}

	for _, na := range notAllowed {
		if strings.Contains(contentStr, na+"=") {
			t.Errorf("writeEnvFile() should not contain %q (moved to hive.yaml)", na)
		}
	}
}

// TestWriteEnvFile_DaemonMode tests writeEnvFile with daemon worker mode
func TestWriteEnvFile_DaemonMode(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	os.MkdirAll(".hive", 0755)

	cfg := map[string]string{
		"GIT_USER_EMAIL":          "test@example.com",
		"GIT_USER_NAME":           "Test User",
		"CLAUDE_CODE_OAUTH_TOKEN": "token",
		"WORKSPACE_NAME":          "test",
		"GIT_REPO_URL":            "",
		"PROJECT_TYPE":            "node",
		"WORKER_MODE":             "daemon",
	}

	err := writeEnvFile(cfg, 3)
	if err != nil {
		t.Fatalf("writeEnvFile() error = %v", err)
	}

	content, _ := os.ReadFile(".hive/.env")
	contentStr := string(content)

	// Should have worker modes configured
	if !strings.Contains(contentStr, "WORKER_1_MODE=daemon") {
		t.Error("writeEnvFile() missing WORKER_1_MODE=daemon")
	}
	if !strings.Contains(contentStr, "WORKER_2_MODE=daemon") {
		t.Error("writeEnvFile() missing WORKER_2_MODE=daemon")
	}
	if !strings.Contains(contentStr, "WORKER_3_MODE=daemon") {
		t.Error("writeEnvFile() missing WORKER_3_MODE=daemon")
	}
}

// TestWriteEnvFile_HybridMode tests writeEnvFile with hybrid worker mode
func TestWriteEnvFile_HybridMode(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	os.MkdirAll(".hive", 0755)

	cfg := map[string]string{
		"CLAUDE_CODE_OAUTH_TOKEN": "token",
		"WORKER_MODE":             "hybrid",
	}

	err := writeEnvFile(cfg, 2)
	if err != nil {
		t.Fatalf("writeEnvFile() error = %v", err)
	}

	content, _ := os.ReadFile(".hive/.env")
	contentStr := string(content)

	// Hybrid mode should NOT write individual WORKER_x_MODE entries
	if strings.Contains(contentStr, "WORKER_1_MODE=") {
		t.Error("writeEnvFile() hybrid mode should not have WORKER_1_MODE")
	}
}

// TestWriteEnvFile_APIBackend tests writeEnvFile with API backend
func TestWriteEnvFile_APIBackend(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	os.MkdirAll(".hive", 0755)

	cfg := map[string]string{
		"HIVE_CLAUDE_BACKEND": "api",
		"ANTHROPIC_API_KEY":   "sk-ant-test-key",
	}

	err := writeEnvFile(cfg, 1)
	if err != nil {
		t.Fatalf("writeEnvFile() error = %v", err)
	}

	content, _ := os.ReadFile(".hive/.env")
	contentStr := string(content)

	if !strings.Contains(contentStr, "HIVE_CLAUDE_BACKEND=api") {
		t.Error("writeEnvFile() missing HIVE_CLAUDE_BACKEND=api")
	}
	if !strings.Contains(contentStr, "ANTHROPIC_API_KEY=sk-ant-test-key") {
		t.Error("writeEnvFile() missing ANTHROPIC_API_KEY")
	}
}

// TestWriteEnvFile_BedrockBackend tests writeEnvFile with Bedrock backend
func TestWriteEnvFile_BedrockBackend(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	os.MkdirAll(".hive", 0755)

	cfg := map[string]string{
		"HIVE_CLAUDE_BACKEND": "bedrock",
		"AWS_PROFILE":         "my-profile",
		"AWS_REGION":          "us-west-2",
	}

	err := writeEnvFile(cfg, 1)
	if err != nil {
		t.Fatalf("writeEnvFile() error = %v", err)
	}

	content, _ := os.ReadFile(".hive/.env")
	contentStr := string(content)

	if !strings.Contains(contentStr, "HIVE_CLAUDE_BACKEND=bedrock") {
		t.Error("writeEnvFile() missing HIVE_CLAUDE_BACKEND=bedrock")
	}
	if !strings.Contains(contentStr, "AWS_PROFILE=my-profile") {
		t.Error("writeEnvFile() missing AWS_PROFILE")
	}
	if !strings.Contains(contentStr, "AWS_REGION=us-west-2") {
		t.Error("writeEnvFile() missing AWS_REGION")
	}
}

// TestWriteHiveYAML tests hive.yaml generation
func TestWriteHiveYAML(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	cfgMap := map[string]string{
		"WORKSPACE_NAME":   "my-workspace",
		"GIT_REPO_URL":     "https://github.com/test/repo.git",
		"GIT_USER_EMAIL":   "test@example.com",
		"GIT_USER_NAME":    "Test User",
		"QUEEN_MODEL":      "opus",
		"WORKER_MODEL":     "sonnet",
		"HIVE_DOCKERFILE":  "docker/Dockerfile.node",
	}
	err := writeHiveYAML(cfgMap, 3)
	if err != nil {
		t.Fatalf("writeHiveYAML() error = %v", err)
	}

	// Verify file was created
	content, err := os.ReadFile("hive.yaml")
	if err != nil {
		t.Fatalf("Failed to read hive.yaml: %v", err)
	}

	contentStr := string(content)

	// Check for workspace name
	if !strings.Contains(contentStr, "my-workspace") {
		t.Error("writeHiveYAML() missing workspace name")
	}

	// Check for git URL
	if !strings.Contains(contentStr, "https://github.com/test/repo.git") {
		t.Error("writeHiveYAML() missing git URL")
	}

	// Check for git user config
	if !strings.Contains(contentStr, "test@example.com") {
		t.Error("writeHiveYAML() missing git email")
	}

	// Check for models
	if !strings.Contains(contentStr, "opus") {
		t.Error("writeHiveYAML() missing queen model")
	}
}

// TestWriteHiveYAML_EmptyGitURL tests hive.yaml without git URL
func TestWriteHiveYAML_EmptyGitURL(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	cfgMap := map[string]string{
		"WORKSPACE_NAME": "test-workspace",
	}
	err := writeHiveYAML(cfgMap, 2)
	if err != nil {
		t.Fatalf("writeHiveYAML() error = %v", err)
	}

	// Verify file was created
	if _, err := os.Stat("hive.yaml"); os.IsNotExist(err) {
		t.Error("hive.yaml not created")
	}
}

// TestDetectGitConfig tests git config detection
func TestDetectGitConfig(t *testing.T) {
	_, _, cleanup := setupTestRepo(t)
	defer cleanup()

	email, name, repoURL, workspaceName := detectGitConfig()

	if email != "test@example.com" {
		t.Errorf("detectGitConfig() email = %q, want %q", email, "test@example.com")
	}
	if name != "Test User" {
		t.Errorf("detectGitConfig() name = %q, want %q", name, "Test User")
	}
	// repoURL will be empty since no remote is set
	if repoURL != "" {
		t.Errorf("detectGitConfig() repoURL = %q, want empty", repoURL)
	}
	if workspaceName == "" {
		t.Error("detectGitConfig() workspaceName should not be empty")
	}
}

// TestDetectGitConfig_WithRemote tests git config detection with remote
func TestDetectGitConfig_WithRemote(t *testing.T) {
	_, _, cleanup := setupTestRepo(t)
	defer cleanup()

	// Add remote
	cmd := exec.Command("git", "remote", "add", "origin", "https://github.com/test/repo.git")
	if err := cmd.Run(); err != nil {
		t.Fatalf("failed to add remote: %v", err)
	}

	email, name, repoURL, _ := detectGitConfig()

	if email != "test@example.com" {
		t.Errorf("detectGitConfig() email = %q, want %q", email, "test@example.com")
	}
	if name != "Test User" {
		t.Errorf("detectGitConfig() name = %q, want %q", name, "Test User")
	}
	if repoURL != "https://github.com/test/repo.git" {
		t.Errorf("detectGitConfig() repoURL = %q, want %q", repoURL, "https://github.com/test/repo.git")
	}
}

// TestExtractHiveFiles tests extraction of embedded hive files
func TestExtractHiveFiles(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	err := extractHiveFiles("node")
	if err != nil {
		t.Fatalf("extractHiveFiles() error = %v", err)
	}

	// Verify essential files exist
	// Note: docker-compose.yml is generated dynamically by generateDockerCompose()
	requiredFiles := []string{
		".hive/entrypoint.sh",
		".hive/start-worker.sh",
		".hive/worker-daemon.py",
		".hive/backends.py",
		".hive/tools.py",
		".hive/docker",
		".hive/scripts",
		".hive/templates",
		".hive/workspaces",
	}

	for _, file := range requiredFiles {
		if _, err := os.Stat(file); os.IsNotExist(err) {
			t.Errorf("extractHiveFiles() missing: %s", file)
		}
	}
}

// TestExtractHiveFiles_CreatesWorkspacesDir tests that workspaces directory is created
func TestExtractHiveFiles_CreatesWorkspacesDir(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	if err := extractHiveFiles("go"); err != nil {
		t.Fatalf("extractHiveFiles() error = %v", err)
	}

	// Verify workspaces directory exists
	info, err := os.Stat(".hive/workspaces")
	if os.IsNotExist(err) {
		t.Error("workspaces directory not created")
	}
	if !info.IsDir() {
		t.Error("workspaces should be a directory")
	}
}

// TestExtractHiveFiles_AllProjectTypes tests extraction for all project types
func TestExtractHiveFiles_AllProjectTypes(t *testing.T) {
	projectTypes := []string{"node", "go", "python", "rust", "minimal"}

	for _, pType := range projectTypes {
		t.Run(pType, func(t *testing.T) {
			tmpDir := t.TempDir()
			oldWd, _ := os.Getwd()
			os.Chdir(tmpDir)
			defer os.Chdir(oldWd)

			if err := extractHiveFiles(pType); err != nil {
				t.Errorf("extractHiveFiles(%q) error = %v", pType, err)
			}

			// Verify essential files exist
			requiredFiles := []string{
				".hive/entrypoint.sh",
				".hive/start-worker.sh",
				".hive/docker",
			}

			for _, file := range requiredFiles {
				if _, err := os.Stat(file); os.IsNotExist(err) {
					t.Errorf("extractHiveFiles(%q) missing: %s", pType, file)
				}
			}
		})
	}
}

// TestFileExists_PathTypes tests fileExists with different path types
func TestFileExists_PathTypes(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Directory exists (fileExists returns true for any existing path)
	os.Mkdir("testdir", 0755)
	if !fileExists("testdir") {
		t.Error("fileExists() should return true for existing directory")
	}

	// Non-existent path
	if fileExists("nonexistent") {
		t.Error("fileExists() should return false for non-existent path")
	}
}

// TestCreateWorktrees tests the createWorktrees function
func TestCreateWorktrees(t *testing.T) {
	_, _, cleanup := setupTestRepo(t)
	defer cleanup()

	// Create .hive directory
	os.MkdirAll(".hive", 0755)

	err := createWorktrees(2)
	if err != nil {
		t.Fatalf("createWorktrees() error = %v", err)
	}

	// Verify worktrees were created
	worktrees := []string{
		".hive/workspaces/queen",
		".hive/workspaces/drone-1",
		".hive/workspaces/drone-2",
	}

	for _, wt := range worktrees {
		if _, err := os.Stat(wt); os.IsNotExist(err) {
			t.Errorf("createWorktrees() missing: %s", wt)
		}
	}
}

// TestSyncHiveYAML tests the syncHiveYAML function
func TestSyncHiveYAML(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Create hive.yaml
	testContent := "workspace:\n  name: test-project\n"
	if err := os.WriteFile("hive.yaml", []byte(testContent), 0644); err != nil {
		t.Fatalf("failed to create hive.yaml: %v", err)
	}

	// Create .hive directory
	if err := os.MkdirAll(".hive", 0755); err != nil {
		t.Fatalf("failed to create .hive: %v", err)
	}

	// Run sync
	err := syncHiveYAML()
	if err != nil {
		t.Fatalf("syncHiveYAML() error = %v", err)
	}

	// Verify copy
	content, err := os.ReadFile(".hive/hive.yaml")
	if err != nil {
		t.Fatalf("failed to read .hive/hive.yaml: %v", err)
	}

	if string(content) != testContent {
		t.Errorf("syncHiveYAML() content mismatch, got %q, want %q", string(content), testContent)
	}
}

// TestSyncHiveYAML_NoSource tests syncHiveYAML when source doesn't exist
func TestSyncHiveYAML_NoSource(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	os.MkdirAll(".hive", 0755)

	// Should error when hive.yaml doesn't exist
	err := syncHiveYAML()
	if err == nil {
		t.Error("syncHiveYAML() should error when source doesn't exist")
	}
}

// TestSyncHostMCPs tests the syncHostMCPs function
func TestSyncHostMCPs(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Mock HOME to temp dir
	oldHome := os.Getenv("HOME")
	os.Setenv("HOME", tmpDir)
	defer os.Setenv("HOME", oldHome)

	// Create fake ~/.claude/settings.json
	claudeDir := filepath.Join(tmpDir, ".claude")
	if err := os.MkdirAll(claudeDir, 0755); err != nil {
		t.Fatalf("failed to create .claude dir: %v", err)
	}
	testSettings := `{"mcpServers":{"test":{"command":"node"}}}`
	if err := os.WriteFile(filepath.Join(claudeDir, "settings.json"), []byte(testSettings), 0644); err != nil {
		t.Fatalf("failed to create settings.json: %v", err)
	}

	// Create .hive directory
	if err := os.MkdirAll(".hive", 0755); err != nil {
		t.Fatalf("failed to create .hive: %v", err)
	}

	// Run sync
	err := syncHostMCPs()
	if err != nil {
		t.Fatalf("syncHostMCPs() error = %v", err)
	}

	// Verify copy
	content, err := os.ReadFile(".hive/host-mcps.json")
	if err != nil {
		t.Fatalf("failed to read .hive/host-mcps.json: %v", err)
	}

	if string(content) != testSettings {
		t.Errorf("syncHostMCPs() content mismatch, got %q, want %q", string(content), testSettings)
	}
}

// TestSyncHostMCPs_NoSource tests syncHostMCPs when source doesn't exist
func TestSyncHostMCPs_NoSource(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Mock HOME to temp dir (no .claude/settings.json)
	oldHome := os.Getenv("HOME")
	os.Setenv("HOME", tmpDir)
	defer os.Setenv("HOME", oldHome)

	os.MkdirAll(".hive", 0755)

	// Should create empty JSON when source doesn't exist
	err := syncHostMCPs()
	if err != nil {
		t.Fatalf("syncHostMCPs() error = %v", err)
	}

	content, err := os.ReadFile(".hive/host-mcps.json")
	if err != nil {
		t.Fatalf("failed to read .hive/host-mcps.json: %v", err)
	}

	if string(content) != "{}" {
		t.Errorf("syncHostMCPs() should create empty JSON, got %q", string(content))
	}
}

// TestSyncProjectCLAUDEmd tests the syncProjectCLAUDEmd function
func TestSyncProjectCLAUDEmd(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	os.MkdirAll(".hive", 0755)

	// Case 1: No CLAUDE.md - should not error
	err := syncProjectCLAUDEmd()
	if err != nil {
		t.Errorf("syncProjectCLAUDEmd() should not error when CLAUDE.md doesn't exist: %v", err)
	}

	// Verify no file was created
	if _, err := os.Stat(".hive/CLAUDE.md"); !os.IsNotExist(err) {
		t.Error("syncProjectCLAUDEmd() should not create file when source doesn't exist")
	}

	// Case 2: With CLAUDE.md
	testContent := "# Project Guidelines\n\nFollow these rules."
	if err := os.WriteFile("CLAUDE.md", []byte(testContent), 0644); err != nil {
		t.Fatalf("failed to create CLAUDE.md: %v", err)
	}

	err = syncProjectCLAUDEmd()
	if err != nil {
		t.Fatalf("syncProjectCLAUDEmd() error = %v", err)
	}

	content, err := os.ReadFile(".hive/CLAUDE.md")
	if err != nil {
		t.Fatalf("failed to read .hive/CLAUDE.md: %v", err)
	}

	if string(content) != testContent {
		t.Errorf("syncProjectCLAUDEmd() content mismatch, got %q, want %q", string(content), testContent)
	}
}

// TestDetectAnthropicApiKey tests the detectAnthropicApiKey function
func TestDetectAnthropicApiKey(t *testing.T) {
	// Save original value
	original := os.Getenv("ANTHROPIC_API_KEY")
	defer os.Setenv("ANTHROPIC_API_KEY", original)

	// Test: Without env var
	os.Unsetenv("ANTHROPIC_API_KEY")
	key := detectAnthropicApiKey()
	if key != "" {
		t.Errorf("detectAnthropicApiKey() = %q, want empty string", key)
	}

	// Test: With env var
	os.Setenv("ANTHROPIC_API_KEY", "sk-ant-api01-xxx")
	key = detectAnthropicApiKey()
	if key != "sk-ant-api01-xxx" {
		t.Errorf("detectAnthropicApiKey() = %q, want %q", key, "sk-ant-api01-xxx")
	}
}

// TestGenerateDockerComposeWithConfig tests the generateDockerComposeWithConfig function
func TestGenerateDockerComposeWithConfig(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Create .hive directory
	os.MkdirAll(".hive", 0755)

	// Generate docker-compose
	err := generateDockerComposeWithConfig(2, 6380)
	if err != nil {
		t.Fatalf("generateDockerComposeWithConfig() error = %v", err)
	}

	// Verify file was created
	content, err := os.ReadFile(".hive/docker-compose.yml")
	if err != nil {
		t.Fatalf("failed to read docker-compose.yml: %v", err)
	}

	contentStr := string(content)

	// Check expected content
	checks := []string{
		"queen:",
		"drone-1:",
		"drone-2:",
		"redis:",
		"6380:6379", // Redis port mapping
	}

	for _, check := range checks {
		if !strings.Contains(contentStr, check) {
			t.Errorf("generateDockerComposeWithConfig() missing %q", check)
		}
	}
}

// TestGenerateDockerComposeWithConfig_DifferentCounts tests with different worker counts
func TestGenerateDockerComposeWithConfig_DifferentCounts(t *testing.T) {
	tests := []struct {
		workers       int
		expectedDrone string
	}{
		{1, "drone-1:"},
		{3, "drone-3:"},
		{5, "drone-5:"},
	}

	for _, tt := range tests {
		t.Run(string(rune(tt.workers+'0')), func(t *testing.T) {
			tmpDir := t.TempDir()
			oldWd, _ := os.Getwd()
			os.Chdir(tmpDir)
			defer os.Chdir(oldWd)

			os.MkdirAll(".hive", 0755)

			err := generateDockerComposeWithConfig(tt.workers, 6379)
			if err != nil {
				t.Fatalf("generateDockerComposeWithConfig() error = %v", err)
			}

			content, _ := os.ReadFile(".hive/docker-compose.yml")
			if !strings.Contains(string(content), tt.expectedDrone) {
				t.Errorf("generateDockerComposeWithConfig(%d) missing %q", tt.workers, tt.expectedDrone)
			}
		})
	}
}

// TestPrintSuccessMessage tests the success message output
func TestPrintSuccessMessage(t *testing.T) {
	tests := []struct {
		workers  int
		expected []string
	}{
		{
			workers:  1,
			expected: []string{"Setup complete", "1 worker", "hive connect queen", "hive connect 1", "hive status"},
		},
		{
			workers:  3,
			expected: []string{"Setup complete", "3 workers", "hive connect queen"},
		},
	}

	for _, tt := range tests {
		t.Run(string(rune(tt.workers+'0')), func(t *testing.T) {
			// Capture stdout
			old := os.Stdout
			r, w, _ := os.Pipe()
			os.Stdout = w

			printSuccessMessage(tt.workers)

			w.Close()
			os.Stdout = old

			var buf bytes.Buffer
			io.Copy(&buf, r)
			output := buf.String()

			for _, exp := range tt.expected {
				if !strings.Contains(output, exp) {
					t.Errorf("printSuccessMessage(%d) output missing %q\nGot: %s", tt.workers, exp, output)
				}
			}
		})
	}
}

// TestGenerateDockerCompose tests the wrapper function
func TestGenerateDockerCompose(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	os.MkdirAll(".hive", 0755)

	err := generateDockerCompose(2)
	if err != nil {
		t.Fatalf("generateDockerCompose() error = %v", err)
	}

	content, err := os.ReadFile(".hive/docker-compose.yml")
	if err != nil {
		t.Fatalf("Failed to read docker-compose.yml: %v", err)
	}

	// Verify content
	if !strings.Contains(string(content), "queen") {
		t.Error("generateDockerCompose() missing queen service")
	}
	if !strings.Contains(string(content), "drone-1") {
		t.Error("generateDockerCompose() missing drone-1 service")
	}
}
