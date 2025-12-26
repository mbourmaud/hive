package shell

import (
	"fmt"
	"strings"
)

// MockExecutor implements CommandExecutor for testing
type MockExecutor struct {
	// Commands records all executed commands for verification
	Commands []string
	// Responses maps command patterns to (stdout, stderr, error)
	Responses map[string]MockResponse
	// DefaultError is returned when no matching response is found
	DefaultError error
}

// MockResponse holds the mocked response for a command
type MockResponse struct {
	Stdout string
	Stderr string
	Err    error
}

// NewMockExecutor creates a new MockExecutor
func NewMockExecutor() *MockExecutor {
	return &MockExecutor{
		Commands:  []string{},
		Responses: make(map[string]MockResponse),
	}
}

// RunCommand executes a command and returns mocked stdout, stderr, error
func (m *MockExecutor) RunCommand(name string, args ...string) (string, string, error) {
	cmdStr := m.formatCommand(name, args...)
	m.Commands = append(m.Commands, cmdStr)

	// Look for matching response
	for pattern, response := range m.Responses {
		if strings.Contains(cmdStr, pattern) {
			return response.Stdout, response.Stderr, response.Err
		}
	}

	if m.DefaultError != nil {
		return "", "", m.DefaultError
	}

	return "", "", nil
}

// RunQuietCommand executes a command silently with mocked response
func (m *MockExecutor) RunQuietCommand(name string, args ...string) error {
	cmdStr := m.formatCommand(name, args...)
	m.Commands = append(m.Commands, cmdStr)

	// Look for matching response
	for pattern, response := range m.Responses {
		if strings.Contains(cmdStr, pattern) {
			return response.Err
		}
	}

	return m.DefaultError
}

// SetResponse sets a response for commands matching the pattern
func (m *MockExecutor) SetResponse(pattern string, stdout, stderr string, err error) {
	m.Responses[pattern] = MockResponse{
		Stdout: stdout,
		Stderr: stderr,
		Err:    err,
	}
}

// SetError sets a response with only an error
func (m *MockExecutor) SetError(pattern string, err error) {
	m.SetResponse(pattern, "", "", err)
}

// SetOutput sets a response with only stdout
func (m *MockExecutor) SetOutput(pattern string, stdout string) {
	m.SetResponse(pattern, stdout, "", nil)
}

// HasCommand checks if a command matching the pattern was executed
func (m *MockExecutor) HasCommand(pattern string) bool {
	for _, cmd := range m.Commands {
		if strings.Contains(cmd, pattern) {
			return true
		}
	}
	return false
}

// CommandCount returns the number of times a command matching the pattern was executed
func (m *MockExecutor) CommandCount(pattern string) int {
	count := 0
	for _, cmd := range m.Commands {
		if strings.Contains(cmd, pattern) {
			count++
		}
	}
	return count
}

// formatCommand formats a command for recording
func (m *MockExecutor) formatCommand(name string, args ...string) string {
	return fmt.Sprintf("%s %s", name, strings.Join(args, " "))
}
