package cmd

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/mbourmaud/hive/internal/ui"
)

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

// Note: pluralize is defined in clean.go
