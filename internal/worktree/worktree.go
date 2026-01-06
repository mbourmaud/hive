// Package worktree provides git worktree management for agent isolation.
package worktree

import (
	"bytes"
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// Worktree represents a git worktree instance.
type Worktree struct {
	Name     string // Logical name (e.g., "front", "back")
	Path     string // Absolute path to worktree directory
	Branch   string // Branch checked out in this worktree
	RepoPath string // Path to the main repository
}

// CreateOptions contains options for creating a worktree.
type CreateOptions struct {
	Name       string // Logical name for the worktree
	RepoPath   string // Path to the main git repository
	Branch     string // Target branch to checkout (or create from BaseBranch)
	BaseBranch string // Base branch to create from (default: current branch)
	WorkDir    string // Parent directory for worktrees
}

// Manager handles git worktree operations.
type Manager interface {
	Create(ctx context.Context, opts CreateOptions) (*Worktree, error)
	Delete(ctx context.Context, name string) error
	List(ctx context.Context) ([]Worktree, error)
	Get(ctx context.Context, name string) (*Worktree, error)
	Prune(ctx context.Context) error
}

// GitManager implements Manager using git commands.
type GitManager struct {
	repoPath string
	workDir  string
}

// NewGitManager creates a new git worktree manager.
func NewGitManager(repoPath, workDir string) *GitManager {
	if workDir == "" {
		home, _ := os.UserHomeDir()
		workDir = filepath.Join(home, "hive-worktrees")
	}
	// Resolve symlinks to get canonical paths (important on macOS where /var -> /private/var)
	workDir = resolveSymlinks(workDir)
	repoPath = resolveSymlinks(repoPath)
	return &GitManager{
		repoPath: repoPath,
		workDir:  workDir,
	}
}

// resolveSymlinks resolves symlinks in a path. If the path doesn't exist,
// it tries to resolve the parent path.
func resolveSymlinks(path string) string {
	if resolved, err := filepath.EvalSymlinks(path); err == nil {
		return resolved
	}
	// Path doesn't exist, try to resolve parent
	parent := filepath.Dir(path)
	if resolvedParent, err := filepath.EvalSymlinks(parent); err == nil {
		return filepath.Join(resolvedParent, filepath.Base(path))
	}
	// Try grandparent
	grandparent := filepath.Dir(parent)
	if resolvedGrandparent, err := filepath.EvalSymlinks(grandparent); err == nil {
		return filepath.Join(resolvedGrandparent, filepath.Base(parent), filepath.Base(path))
	}
	return path
}

// Create creates a new git worktree for an agent.
func (m *GitManager) Create(ctx context.Context, opts CreateOptions) (*Worktree, error) {
	if opts.Name == "" {
		return nil, fmt.Errorf("worktree name is required")
	}

	repoPath := opts.RepoPath
	if repoPath == "" {
		repoPath = m.repoPath
	}

	workDir := opts.WorkDir
	if workDir == "" {
		workDir = m.workDir
	}

	// Ensure work directory exists
	if err := os.MkdirAll(workDir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create worktree directory: %w", err)
	}

	worktreePath := filepath.Join(workDir, opts.Name)
	branchName := opts.Branch
	if branchName == "" {
		branchName = fmt.Sprintf("hive/%s", opts.Name)
	}

	// Prune orphaned worktrees first
	if err := m.Prune(ctx); err != nil {
		// Non-fatal, continue
	}

	// Check if worktree already exists and is valid
	if m.isValidWorktree(ctx, worktreePath) {
		return &Worktree{
			Name:     opts.Name,
			Path:     worktreePath,
			Branch:   branchName,
			RepoPath: repoPath,
		}, nil
	}

	// Remove any orphaned directory
	os.RemoveAll(worktreePath)

	// Determine base branch
	baseBranch := opts.BaseBranch
	if baseBranch == "" {
		var err error
		baseBranch, err = m.getCurrentBranch(ctx)
		if err != nil {
			baseBranch = "main"
		}
	}

	// Check if branch already exists
	branchExists := m.branchExists(ctx, branchName)

	if branchExists {
		// Try to create worktree with existing branch
		cmd := exec.CommandContext(ctx, "git", "-C", repoPath, "worktree", "add", worktreePath, branchName)
		if err := cmd.Run(); err == nil {
			return &Worktree{
				Name:     opts.Name,
				Path:     worktreePath,
				Branch:   branchName,
				RepoPath: repoPath,
			}, nil
		}
		// Branch exists but worktree failed, delete branch and recreate
		exec.CommandContext(ctx, "git", "-C", repoPath, "branch", "-D", branchName).Run()
	}

	// Create new worktree with new branch
	cmd := exec.CommandContext(ctx, "git", "-C", repoPath, "worktree", "add", "-b", branchName, worktreePath, baseBranch)
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		errMsg := strings.TrimSpace(stderr.String())
		if errMsg == "" {
			errMsg = err.Error()
		}
		return nil, fmt.Errorf("failed to create worktree %s: %s", opts.Name, errMsg)
	}

	return &Worktree{
		Name:     opts.Name,
		Path:     worktreePath,
		Branch:   branchName,
		RepoPath: repoPath,
	}, nil
}

// Delete removes a worktree.
func (m *GitManager) Delete(ctx context.Context, name string) error {
	worktreePath := filepath.Join(m.workDir, name)

	// Remove the worktree using git
	cmd := exec.CommandContext(ctx, "git", "-C", m.repoPath, "worktree", "remove", "--force", worktreePath)
	if err := cmd.Run(); err != nil {
		// If git worktree remove fails, try manual cleanup
		os.RemoveAll(worktreePath)
	}

	// Prune to clean up references
	return m.Prune(ctx)
}

// List returns all worktrees managed by this manager.
func (m *GitManager) List(ctx context.Context) ([]Worktree, error) {
	cmd := exec.CommandContext(ctx, "git", "-C", m.repoPath, "worktree", "list", "--porcelain")
	output, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("failed to list worktrees: %w", err)
	}

	return m.parseWorktreeList(string(output)), nil
}

// Get returns a specific worktree by name.
func (m *GitManager) Get(ctx context.Context, name string) (*Worktree, error) {
	worktrees, err := m.List(ctx)
	if err != nil {
		return nil, err
	}

	for _, wt := range worktrees {
		if wt.Name == name {
			return &wt, nil
		}
	}

	return nil, fmt.Errorf("worktree %s not found", name)
}

// Prune removes orphaned worktree references.
func (m *GitManager) Prune(ctx context.Context) error {
	cmd := exec.CommandContext(ctx, "git", "-C", m.repoPath, "worktree", "prune")
	return cmd.Run()
}

// isValidWorktree checks if a worktree path is valid and registered.
func (m *GitManager) isValidWorktree(ctx context.Context, path string) bool {
	// Check if .git exists in the worktree
	if _, err := os.Stat(filepath.Join(path, ".git")); err != nil {
		return false
	}

	// Verify it's registered with git
	cmd := exec.CommandContext(ctx, "git", "-C", m.repoPath, "worktree", "list")
	output, err := cmd.Output()
	if err != nil {
		return false
	}

	return strings.Contains(string(output), path)
}

// getCurrentBranch returns the current branch of the repository.
func (m *GitManager) getCurrentBranch(ctx context.Context) (string, error) {
	cmd := exec.CommandContext(ctx, "git", "-C", m.repoPath, "rev-parse", "--abbrev-ref", "HEAD")
	output, err := cmd.Output()
	if err != nil {
		return "", err
	}
	return strings.TrimSpace(string(output)), nil
}

// branchExists checks if a branch exists.
func (m *GitManager) branchExists(ctx context.Context, branch string) bool {
	cmd := exec.CommandContext(ctx, "git", "-C", m.repoPath, "rev-parse", "--verify", branch)
	return cmd.Run() == nil
}

// parseWorktreeList parses git worktree list --porcelain output.
func (m *GitManager) parseWorktreeList(output string) []Worktree {
	var worktrees []Worktree
	var current Worktree

	lines := strings.Split(output, "\n")
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" {
			if current.Path != "" {
				// Only include worktrees in our work directory
				if strings.HasPrefix(current.Path, m.workDir) {
					current.Name = filepath.Base(current.Path)
					current.RepoPath = m.repoPath
					worktrees = append(worktrees, current)
				}
			}
			current = Worktree{}
			continue
		}

		if strings.HasPrefix(line, "worktree ") {
			current.Path = strings.TrimPrefix(line, "worktree ")
		} else if strings.HasPrefix(line, "branch ") {
			current.Branch = strings.TrimPrefix(line, "branch refs/heads/")
		}
	}

	// Handle last entry
	if current.Path != "" && strings.HasPrefix(current.Path, m.workDir) {
		current.Name = filepath.Base(current.Path)
		current.RepoPath = m.repoPath
		worktrees = append(worktrees, current)
	}

	return worktrees
}

// IsGitRepository checks if a path is inside a git repository.
func IsGitRepository(path string) bool {
	cmd := exec.Command("git", "-C", path, "rev-parse", "--is-inside-work-tree")
	return cmd.Run() == nil
}

// GetRepoRoot returns the root of the git repository.
func GetRepoRoot(path string) (string, error) {
	cmd := exec.Command("git", "-C", path, "rev-parse", "--show-toplevel")
	output, err := cmd.Output()
	if err != nil {
		return "", fmt.Errorf("not a git repository: %w", err)
	}
	return strings.TrimSpace(string(output)), nil
}
