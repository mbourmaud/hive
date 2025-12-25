package cmd

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mbourmaud/hive/internal/shell"
)

// TestPluralize tests the pluralize helper function
func TestPluralize(t *testing.T) {
	tests := []struct {
		count    int
		expected string
	}{
		{0, ""},
		{1, ""},
		{2, "s"},
		{5, "s"},
		{10, "s"},
		{100, "s"},
	}

	for _, tt := range tests {
		t.Run(string(rune(tt.count+'0')), func(t *testing.T) {
			got := pluralize(tt.count)
			if got != tt.expected {
				t.Errorf("pluralize(%d) = %q, want %q", tt.count, got, tt.expected)
			}
		})
	}
}

// TestCleanGitignore tests .gitignore cleaning logic
func TestCleanGitignore(t *testing.T) {
	tests := []struct {
		name            string
		inputContent    string
		expectedContent string
		shouldModify    bool
	}{
		{
			name: "remove hive section with comment",
			inputContent: `node_modules/
*.log

# Hive (multi-agent Claude)
.hive/

package-lock.json
`,
			expectedContent: `node_modules/
*.log

package-lock.json
`,
			shouldModify: true,
		},
		{
			name: "remove hive section without trailing newline",
			inputContent: `node_modules/

# Hive (multi-agent Claude)
.hive/`,
			expectedContent: `node_modules/
`,
			shouldModify: true,
		},
		{
			name: "no hive section",
			inputContent: `node_modules/
*.log
dist/
`,
			expectedContent: `node_modules/
*.log
dist/
`,
			shouldModify: false,
		},
		{
			name: "hive at start of file",
			inputContent: `# Hive (multi-agent Claude)
.hive/

node_modules/
*.log
`,
			expectedContent: `node_modules/
*.log
`,
			shouldModify: true,
		},
		{
			name: "hive at end of file",
			inputContent: `node_modules/
*.log

# Hive (multi-agent Claude)
.hive/
`,
			expectedContent: `node_modules/
*.log
`,
			shouldModify: true,
		},
		{
			name: "hive with .hive variant",
			inputContent: `node_modules/

# Hive (multi-agent Claude)
.hive

dist/
`,
			expectedContent: `node_modules/

dist/
`,
			shouldModify: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Create temp directory
			tmpDir := t.TempDir()
			oldWd, _ := os.Getwd()
			os.Chdir(tmpDir)
			defer os.Chdir(oldWd)

			// Write input .gitignore
			gitignorePath := ".gitignore"
			if err := os.WriteFile(gitignorePath, []byte(tt.inputContent), 0644); err != nil {
				t.Fatalf("Failed to create test .gitignore: %v", err)
			}

			// Run cleanGitignore
			err := cleanGitignore()
			if err != nil {
				t.Fatalf("cleanGitignore() error = %v", err)
			}

			// Read result
			content, readErr := os.ReadFile(gitignorePath)
			if readErr != nil {
				t.Fatalf("Failed to read .gitignore: %v", readErr)
			}

			got := string(content)

			// Compare
			if got != tt.expectedContent {
				t.Errorf("cleanGitignore() content mismatch\nGot:\n%s\nWant:\n%s",
					got, tt.expectedContent)
			}

			// Verify modification expectation
			if tt.shouldModify && got == tt.inputContent {
				t.Errorf("cleanGitignore() should have modified content but didn't")
			}
			if !tt.shouldModify && got != tt.inputContent {
				t.Errorf("cleanGitignore() should not have modified content but did")
			}
		})
	}
}

// TestCleanGitignore_NoFile tests behavior when .gitignore doesn't exist
func TestCleanGitignore_NoFile(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	// Run cleanGitignore without .gitignore
	err := cleanGitignore()
	if err != nil {
		t.Errorf("cleanGitignore() with no file should not error, got: %v", err)
	}

	// Verify no .gitignore was created
	if _, statErr := os.Stat(".gitignore"); !os.IsNotExist(statErr) {
		t.Error("cleanGitignore() should not create .gitignore if it doesn't exist")
	}
}

// TestCleanGitignore_PreservesOtherContent tests that non-hive content is preserved
func TestCleanGitignore_PreservesOtherContent(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	input := `# Project-specific
node_modules/
dist/
*.log

# IDE
.vscode/
.idea/

# Hive (multi-agent Claude)
.hive/

# OS
.DS_Store
Thumbs.db
`

	expected := `# Project-specific
node_modules/
dist/
*.log

# IDE
.vscode/
.idea/

# OS
.DS_Store
Thumbs.db
`

	os.WriteFile(".gitignore", []byte(input), 0644)

	err := cleanGitignore()
	if err != nil {
		t.Fatalf("cleanGitignore() error = %v", err)
	}

	content, _ := os.ReadFile(".gitignore")
	got := string(content)

	if got != expected {
		t.Errorf("cleanGitignore() did not preserve non-hive content\nGot:\n%s\nWant:\n%s",
			got, expected)
	}

	// Verify all non-hive sections are present
	requiredSections := []string{"# Project-specific", "# IDE", "# OS", ".DS_Store", "node_modules/"}
	for _, section := range requiredSections {
		if !strings.Contains(got, section) {
			t.Errorf("cleanGitignore() removed non-hive content: %s", section)
		}
	}

	// Verify hive section is removed
	if strings.Contains(got, "# Hive") || strings.Contains(got, ".hive/") {
		t.Error("cleanGitignore() did not remove hive section")
	}
}

// TestCleanWorktrees tests worktree cleanup
func TestCleanWorktrees(t *testing.T) {
	// Setup test git repo
	_, _, cleanup := setupTestRepo(t)
	defer cleanup()

	// Create .hive/workspaces with worktrees
	os.MkdirAll(".hive/workspaces", 0755)

	// Get current branch
	branchCmd := exec.Command("git", "rev-parse", "--abbrev-ref", "HEAD")
	branchOut, err := branchCmd.Output()
	if err != nil {
		t.Fatalf("failed to get branch: %v", err)
	}
	defaultBranch := strings.TrimSpace(string(branchOut))

	// Create worktrees
	worktreePath := filepath.Join(".hive", "workspaces", "queen")
	if err := createWorktree(worktreePath, defaultBranch, "queen"); err != nil {
		t.Fatalf("failed to create worktree: %v", err)
	}

	// Verify worktree exists
	if _, err := os.Stat(worktreePath); os.IsNotExist(err) {
		t.Fatal("worktree not created")
	}

	// Clean worktrees
	runner := shell.NewRunner(false)
	err = cleanWorktrees(runner)
	if err != nil {
		t.Errorf("cleanWorktrees() error = %v", err)
	}

	// Verify worktree was removed
	listCmd := exec.Command("git", "worktree", "list")
	output, err := listCmd.Output()
	if err != nil {
		t.Fatalf("failed to list worktrees: %v", err)
	}

	if strings.Contains(string(output), "queen") {
		t.Error("cleanWorktrees() did not remove worktree")
	}
}

// TestCleanWorktrees_NoWorkspacesDir tests cleanup when no workspaces directory
func TestCleanWorktrees_NoWorkspacesDir(t *testing.T) {
	_, _, cleanup := setupTestRepo(t)
	defer cleanup()

	runner := shell.NewRunner(false)
	err := cleanWorktrees(runner)
	if err != nil {
		t.Errorf("cleanWorktrees() with no workspaces dir should not error: %v", err)
	}
}

// TestCleanWorktrees_NotGitRepo tests cleanup when not in git repo
func TestCleanWorktrees_NotGitRepo(t *testing.T) {
	tmpDir := t.TempDir()
	oldWd, _ := os.Getwd()
	os.Chdir(tmpDir)
	defer os.Chdir(oldWd)

	runner := shell.NewRunner(false)
	err := cleanWorktrees(runner)
	if err != nil {
		t.Errorf("cleanWorktrees() outside git repo should not error: %v", err)
	}
}
