package shell

import (
	"os/exec"
)

// CommandExecutor abstracts shell command execution for testing
type CommandExecutor interface {
	// RunCommand executes a command and returns stdout, stderr, error
	RunCommand(name string, args ...string) (stdout string, stderr string, err error)
	// RunQuietCommand executes a command silently, only returning error
	RunQuietCommand(name string, args ...string) error
}

// RealExecutor implements CommandExecutor using real shell commands
type RealExecutor struct {
	runner *Runner
}

// NewRealExecutor creates a new RealExecutor
func NewRealExecutor(debug bool) *RealExecutor {
	return &RealExecutor{runner: NewRunner(debug)}
}

// RunCommand executes a command and returns stdout, stderr, error
func (e *RealExecutor) RunCommand(name string, args ...string) (string, string, error) {
	cmd := exec.Command(name, args...)
	return e.runner.RunCapture(cmd)
}

// RunQuietCommand executes a command silently
func (e *RealExecutor) RunQuietCommand(name string, args ...string) error {
	cmd := exec.Command(name, args...)
	return e.runner.RunQuiet(cmd)
}

// Runner returns the underlying Runner for backward compatibility
func (e *RealExecutor) Runner() *Runner {
	return e.runner
}
