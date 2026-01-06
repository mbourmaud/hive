package shell

import (
	"testing"
)

// TestNewMockExecutor tests MockExecutor creation
func TestNewMockExecutor(t *testing.T) {
	mock := NewMockExecutor()
	if mock == nil {
		t.Fatal("NewMockExecutor() returned nil")
	}
	if mock.Commands == nil {
		t.Error("Commands slice not initialized")
	}
	if mock.Responses == nil {
		t.Error("Responses map not initialized")
	}
}

// TestMockExecutor_RunCommand tests mocked command execution
func TestMockExecutor_RunCommand(t *testing.T) {
	mock := NewMockExecutor()
	mock.SetOutput("git status", "main")

	stdout, stderr, err := mock.RunCommand("git", "status", "-s")
	if err != nil {
		t.Errorf("RunCommand() error = %v", err)
	}
	if stdout != "main" {
		t.Errorf("RunCommand() stdout = %q, want %q", stdout, "main")
	}
	if stderr != "" {
		t.Errorf("RunCommand() stderr = %q, want empty", stderr)
	}
	if !mock.HasCommand("git status") {
		t.Error("Command not recorded")
	}
}

// TestMockExecutor_RunQuietCommand tests quiet command execution
func TestMockExecutor_RunQuietCommand(t *testing.T) {
	mock := NewMockExecutor()
	mock.SetOutput("rm file", "")

	err := mock.RunQuietCommand("rm", "file", "-f")
	if err != nil {
		t.Errorf("RunQuietCommand() error = %v", err)
	}
	if !mock.HasCommand("rm file") {
		t.Error("Command not recorded")
	}
}

// TestMockExecutor_SetError tests error response
func TestMockExecutor_SetError(t *testing.T) {
	mock := NewMockExecutor()
	mock.SetError("git status", errTest)

	_, _, err := mock.RunCommand("git", "status")
	if err == nil {
		t.Error("RunCommand() expected error")
	}
}

// errTest is a test error
var errTest = &testError{msg: "test error"}

type testError struct {
	msg string
}

func (e *testError) Error() string {
	return e.msg
}

// TestMockExecutor_CommandCount tests command counting
func TestMockExecutor_CommandCount(t *testing.T) {
	mock := NewMockExecutor()

	mock.RunCommand("git", "status")
	mock.RunCommand("git", "status")
	mock.RunCommand("git", "log")

	if count := mock.CommandCount("git status"); count != 2 {
		t.Errorf("CommandCount(git status) = %d, want 2", count)
	}
	if count := mock.CommandCount("git log"); count != 1 {
		t.Errorf("CommandCount(git log) = %d, want 1", count)
	}
}

// TestMockExecutor_DefaultError tests default error behavior
func TestMockExecutor_DefaultError(t *testing.T) {
	mock := NewMockExecutor()
	mock.DefaultError = errTest

	_, _, err := mock.RunCommand("unknown", "command")
	if err == nil {
		t.Error("RunCommand() expected default error")
	}

	err = mock.RunQuietCommand("unknown", "command")
	if err == nil {
		t.Error("RunQuietCommand() expected default error")
	}
}

// TestRealExecutor_New tests RealExecutor creation
func TestRealExecutor_New(t *testing.T) {
	exec := NewRealExecutor(true)
	if exec == nil {
		t.Fatal("NewRealExecutor() returned nil")
	}
	if exec.Runner() == nil {
		t.Error("Runner() returned nil")
	}
	if !exec.Runner().Debug {
		t.Error("Debug mode not set")
	}
}

// TestRealExecutor_RunCommand tests real command execution
func TestRealExecutor_RunCommand(t *testing.T) {
	exec := NewRealExecutor(false)

	// Run a simple command that should work everywhere
	stdout, _, err := exec.RunCommand("echo", "hello")
	if err != nil {
		t.Errorf("RunCommand(echo) error = %v", err)
	}
	if stdout != "hello\n" {
		t.Errorf("RunCommand(echo) stdout = %q, want %q", stdout, "hello\n")
	}
}

// TestRealExecutor_RunQuietCommand tests quiet command execution
func TestRealExecutor_RunQuietCommand(t *testing.T) {
	exec := NewRealExecutor(false)

	// Run a simple command
	err := exec.RunQuietCommand("echo", "hello")
	if err != nil {
		t.Errorf("RunQuietCommand(echo) error = %v", err)
	}
}

// TestMockExecutor_FormatCommand tests command formatting
func TestMockExecutor_FormatCommand(t *testing.T) {
	mock := NewMockExecutor()
	formatted := mock.formatCommand("git", "log", "-n", "10", "--oneline")
	expected := "git log -n 10 --oneline"
	if formatted != expected {
		t.Errorf("formatCommand() = %q, want %q", formatted, expected)
	}
}
