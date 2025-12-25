package shell

import (
	"os/exec"
	"strings"
	"testing"
)

// TestNewRunner tests runner creation
func TestNewRunner(t *testing.T) {
	tests := []struct {
		name  string
		debug bool
	}{
		{"debug mode enabled", true},
		{"debug mode disabled", false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			runner := NewRunner(tt.debug)
			if runner == nil {
				t.Fatal("NewRunner returned nil")
			}
			if runner.Debug != tt.debug {
				t.Errorf("NewRunner(%v).Debug = %v, want %v", tt.debug, runner.Debug, tt.debug)
			}
		})
	}
}

// TestGetDebugMode tests the debug mode getter
func TestGetDebugMode(t *testing.T) {
	runner := NewRunner(true)
	if !runner.GetDebugMode() {
		t.Error("GetDebugMode() = false, want true")
	}

	runner = NewRunner(false)
	if runner.GetDebugMode() {
		t.Error("GetDebugMode() = true, want false")
	}
}

// TestSetDebugMode tests the debug mode setter
func TestSetDebugMode(t *testing.T) {
	runner := NewRunner(false)

	runner.SetDebugMode(true)
	if !runner.Debug {
		t.Error("SetDebugMode(true) did not set debug mode")
	}

	runner.SetDebugMode(false)
	if runner.Debug {
		t.Error("SetDebugMode(false) did not unset debug mode")
	}
}

// TestFormatCommand tests command formatting for display
func TestFormatCommand(t *testing.T) {
	runner := NewRunner(false)

	tests := []struct {
		name     string
		cmd      *exec.Cmd
		contains []string
	}{
		{
			name:     "simple command",
			cmd:      exec.Command("echo", "hello"),
			contains: []string{"echo", "hello"},
		},
		{
			name:     "command with multiple args",
			cmd:      exec.Command("docker", "compose", "-f", "file.yml", "up"),
			contains: []string{"docker", "compose", "-f", "file.yml", "up"},
		},
		{
			name: "command with working directory",
			cmd: func() *exec.Cmd {
				c := exec.Command("ls", "-la")
				c.Dir = "/tmp"
				return c
			}(),
			contains: []string{"cd /tmp", "ls", "-la"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := runner.formatCommand(tt.cmd)

			for _, s := range tt.contains {
				if !strings.Contains(result, s) {
					t.Errorf("formatCommand() = %q, missing %q", result, s)
				}
			}
		})
	}
}

// TestRunQuiet tests quiet command execution
func TestRunQuiet(t *testing.T) {
	runner := NewRunner(false)

	tests := []struct {
		name    string
		cmd     *exec.Cmd
		wantErr bool
	}{
		{
			name:    "successful command",
			cmd:     exec.Command("echo", "hello"),
			wantErr: false,
		},
		{
			name:    "true command",
			cmd:     exec.Command("true"),
			wantErr: false,
		},
		{
			name:    "false command fails",
			cmd:     exec.Command("false"),
			wantErr: true,
		},
		{
			name:    "non-existent command fails",
			cmd:     exec.Command("nonexistent-command-12345"),
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := runner.RunQuiet(tt.cmd)
			if (err != nil) != tt.wantErr {
				t.Errorf("RunQuiet() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

// TestRunQuiet_DebugMode tests quiet execution in debug mode
func TestRunQuiet_DebugMode(t *testing.T) {
	runner := NewRunner(true)

	cmd := exec.Command("echo", "hello")
	err := runner.RunQuiet(cmd)
	if err != nil {
		t.Errorf("RunQuiet() in debug mode error = %v", err)
	}
}

// TestRunCapture tests command output capture
func TestRunCapture(t *testing.T) {
	runner := NewRunner(false)

	tests := []struct {
		name       string
		cmd        *exec.Cmd
		wantStdout string
		wantErr    bool
	}{
		{
			name:       "capture stdout",
			cmd:        exec.Command("echo", "hello"),
			wantStdout: "hello",
			wantErr:    false,
		},
		{
			name:       "capture multiple lines",
			cmd:        exec.Command("printf", "line1\nline2"),
			wantStdout: "line1\nline2",
			wantErr:    false,
		},
		{
			name:       "failing command",
			cmd:        exec.Command("false"),
			wantStdout: "",
			wantErr:    true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			stdout, _, err := runner.RunCapture(tt.cmd)

			if (err != nil) != tt.wantErr {
				t.Errorf("RunCapture() error = %v, wantErr %v", err, tt.wantErr)
				return
			}

			if !tt.wantErr && !strings.Contains(stdout, tt.wantStdout) {
				t.Errorf("RunCapture() stdout = %q, want to contain %q", stdout, tt.wantStdout)
			}
		})
	}
}

// TestRunCapture_DebugMode tests capture in debug mode
func TestRunCapture_DebugMode(t *testing.T) {
	runner := NewRunner(true)

	stdout, stderr, err := runner.RunCapture(exec.Command("echo", "test"))
	if err != nil {
		t.Errorf("RunCapture() error = %v", err)
	}
	if !strings.Contains(stdout, "test") {
		t.Errorf("RunCapture() stdout = %q, want to contain 'test'", stdout)
	}
	_ = stderr // stderr is usually empty for echo
}

// TestRunCapture_Stderr tests stderr capture
func TestRunCapture_Stderr(t *testing.T) {
	runner := NewRunner(false)

	// Use sh -c to redirect echo to stderr
	cmd := exec.Command("sh", "-c", "echo error >&2")
	_, stderr, err := runner.RunCapture(cmd)

	if err != nil {
		t.Errorf("RunCapture() unexpected error = %v", err)
	}
	if !strings.Contains(stderr, "error") {
		t.Errorf("RunCapture() stderr = %q, want to contain 'error'", stderr)
	}
}

// TestRun tests basic command execution
func TestRun(t *testing.T) {
	runner := NewRunner(false)

	tests := []struct {
		name    string
		cmd     *exec.Cmd
		wantErr bool
	}{
		{
			name:    "successful command",
			cmd:     exec.Command("true"),
			wantErr: false,
		},
		{
			name:    "failing command",
			cmd:     exec.Command("false"),
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := runner.Run(tt.cmd)
			if (err != nil) != tt.wantErr {
				t.Errorf("Run() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

// TestRunWithTitle tests command execution with title
func TestRunWithTitle(t *testing.T) {
	runner := NewRunner(false)

	// Test successful command
	cmd := exec.Command("true")
	err := runner.RunWithTitle(cmd, "Test Command")
	if err != nil {
		t.Errorf("RunWithTitle() error = %v", err)
	}

	// Test failing command
	cmd = exec.Command("false")
	err = runner.RunWithTitle(cmd, "Failing Command")
	if err == nil {
		t.Error("RunWithTitle() expected error for failing command")
	}
}

// TestRunWithOutput tests command execution with output
func TestRunWithOutput(t *testing.T) {
	runner := NewRunner(false)

	cmd := exec.Command("echo", "output test")
	err := runner.RunWithOutput(cmd)
	if err != nil {
		t.Errorf("RunWithOutput() error = %v", err)
	}
}

// TestRun_DebugMode tests run in debug mode
func TestRun_DebugMode(t *testing.T) {
	runner := NewRunner(true)

	cmd := exec.Command("echo", "debug test")
	err := runner.Run(cmd)
	if err != nil {
		t.Errorf("Run() in debug mode error = %v", err)
	}
}

// TestRunWithTitle_EmptyTitle tests command with empty title
func TestRunWithTitle_EmptyTitle(t *testing.T) {
	runner := NewRunner(false)

	cmd := exec.Command("false")
	err := runner.RunWithTitle(cmd, "")
	if err == nil {
		t.Error("RunWithTitle() with empty title expected error for failing command")
	}
}
