package cmd

import (
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

// TestValidateFlags tests flag validation logic
func TestValidateFlags(t *testing.T) {
	tests := []struct {
		name      string
		email     string
		userName  string
		token     string
		wantErr   bool
		errSubstr string
	}{
		{
			name:     "all flags provided",
			email:    "user@example.com",
			userName: "Test User",
			token:    "test-token",
			wantErr:  false,
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
			name:      "missing token",
			email:     "user@example.com",
			userName:  "Test User",
			token:     "",
			wantErr:   true,
			errSubstr: "--token is required",
		},
		{
			name:      "invalid email format",
			email:     "invalid-email",
			userName:  "Test User",
			token:     "test-token",
			wantErr:   true,
			errSubstr: "invalid email",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Set flags
			flagEmail = tt.email
			flagName = tt.userName
			flagToken = tt.token

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
	requiredFiles := []string{
		".hive/docker-compose.yml",
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
