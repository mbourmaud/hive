package shell

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"

	"github.com/mbourmaud/hive/internal/ui"
)

// Runner executes shell commands with optional debug mode
type Runner struct {
	Debug bool
}

// NewRunner creates a new command runner
func NewRunner(debug bool) *Runner {
	return &Runner{Debug: debug}
}

// Run executes a command with appropriate output handling based on debug mode
func (r *Runner) Run(cmd *exec.Cmd) error {
	return r.RunWithTitle(cmd, "")
}

// RunWithTitle executes a command with a descriptive title for error messages
func (r *Runner) RunWithTitle(cmd *exec.Cmd, title string) error {
	// Log command in debug mode
	if r.Debug {
		cmdStr := r.formatCommand(cmd)
		fmt.Fprintf(os.Stderr, "%s\n", ui.StyleDim.Render("[DEBUG] Executing: "+cmdStr))
	}

	var stdout, stderr bytes.Buffer

	if r.Debug {
		// Debug mode: stream output in real-time
		cmd.Stdout = io.MultiWriter(os.Stdout, &stdout)
		cmd.Stderr = io.MultiWriter(os.Stderr, &stderr)
	} else {
		// Normal mode: capture output
		cmd.Stdout = &stdout
		cmd.Stderr = &stderr
	}

	err := cmd.Run()

	// Handle errors
	if err != nil {
		if !r.Debug && stderr.Len() > 0 {
			// In normal mode, show error box with stderr
			errorTitle := title
			if errorTitle == "" {
				errorTitle = "Command Failed"
			}
			fmt.Println(ui.ErrorBox(errorTitle, stderr.String()))
		}
		return err
	}

	// In normal mode with no error, show stdout if not empty
	if !r.Debug && stdout.Len() > 0 {
		fmt.Print(stdout.String())
	}

	return nil
}

// RunQuiet executes a command and only returns error, no output shown
func (r *Runner) RunQuiet(cmd *exec.Cmd) error {
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	if r.Debug {
		cmdStr := r.formatCommand(cmd)
		fmt.Fprintf(os.Stderr, "%s\n", ui.StyleDim.Render("[DEBUG] Executing (quiet): "+cmdStr))
	}

	return cmd.Run()
}

// RunCapture executes a command and returns stdout, stderr, and error
func (r *Runner) RunCapture(cmd *exec.Cmd) (string, string, error) {
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	if r.Debug {
		cmdStr := r.formatCommand(cmd)
		fmt.Fprintf(os.Stderr, "%s\n", ui.StyleDim.Render("[DEBUG] Executing (capture): "+cmdStr))
	}

	err := cmd.Run()

	if r.Debug && err != nil {
		fmt.Fprintf(os.Stderr, "%s\n", ui.StyleDim.Render("[DEBUG] Command failed with: "+err.Error()))
		if stderr.Len() > 0 {
			fmt.Fprintf(os.Stderr, "%s\n", ui.StyleDim.Render("[DEBUG] stderr: "+stderr.String()))
		}
	}

	return stdout.String(), stderr.String(), err
}

// formatCommand formats a command for display
func (r *Runner) formatCommand(cmd *exec.Cmd) string {
	parts := []string{cmd.Path}
	parts = append(parts, cmd.Args[1:]...)

	// Add working directory if set
	if cmd.Dir != "" {
		return fmt.Sprintf("(cd %s && %s)", cmd.Dir, strings.Join(parts, " "))
	}

	return strings.Join(parts, " ")
}

// GetDebugMode returns the current debug mode setting
func (r *Runner) GetDebugMode() bool {
	return r.Debug
}

// SetDebugMode sets the debug mode
func (r *Runner) SetDebugMode(debug bool) {
	r.Debug = debug
}
