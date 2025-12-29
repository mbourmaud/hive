package cmd

import (
	"strings"
	"testing"
)

func TestVersionVariables(t *testing.T) {
	// Test that version variables have default values
	if Version == "" {
		t.Error("Version should not be empty")
	}
	if GitCommit == "" {
		t.Error("GitCommit should not be empty")
	}
	if BuildDate == "" {
		t.Error("BuildDate should not be empty")
	}
}

func TestGetVersionString(t *testing.T) {
	// Save original values
	origVersion := Version
	origCommit := GitCommit
	origDate := BuildDate
	defer func() {
		Version = origVersion
		GitCommit = origCommit
		BuildDate = origDate
	}()

	Version = "2.0.0"
	GitCommit = "def456"
	BuildDate = "2024-06-01"

	result := GetVersionString()

	// With lipgloss styling, we verify the content is present rather than exact format
	requiredStrings := []string{
		"hive",
		"2.0.0",
		"def456",
		"2024-06-01",
	}

	for _, required := range requiredStrings {
		if !strings.Contains(result, required) {
			t.Errorf("GetVersionString() missing required string %q, got: %s", required, result)
		}
	}
}

func TestRootCmdUsage(t *testing.T) {
	// Test that root command has correct usage info
	if rootCmd.Use != "hive" {
		t.Errorf("expected Use to be 'hive', got '%s'", rootCmd.Use)
	}
	if rootCmd.Short == "" {
		t.Error("Short description should not be empty")
	}
	if rootCmd.Long == "" {
		t.Error("Long description should not be empty")
	}
}

func TestMapAgentIDWithPrefix(t *testing.T) {
	tests := []struct {
		input    string
		prefix   string
		expected string
	}{
		{"queen", "hive", "hive-queen"},
		{"q", "hive", "hive-queen"},
		{"0", "hive", "hive-queen"},
		{"1", "hive", "hive-drone-1"},
		{"2", "hive", "hive-drone-2"},
		{"10", "hive", "hive-drone-10"},
		{"queen", "my-project", "my-project-queen"},
		{"1", "my-project", "my-project-drone-1"},
		{"queen", "custom", "custom-queen"},
		{"5", "custom", "custom-drone-5"},
	}

	for _, tt := range tests {
		t.Run(tt.prefix+"-"+tt.input, func(t *testing.T) {
			result := mapAgentIDWithPrefix(tt.input, tt.prefix)
			if result != tt.expected {
				t.Errorf("mapAgentIDWithPrefix(%s, %s) = %s, expected %s", tt.input, tt.prefix, result, tt.expected)
			}
		})
	}
}

func TestValidateEmail(t *testing.T) {
	tests := []struct {
		email   string
		wantErr bool
	}{
		{"user@example.com", false},
		{"test.user@domain.org", false},
		{"user+tag@example.co.uk", false},
		{"invalid", true},
		{"@example.com", true},
		{"user@", true},
		{"user@.com", true},
		{"", true},
	}

	for _, tt := range tests {
		t.Run(tt.email, func(t *testing.T) {
			err := validateEmail(tt.email)
			if tt.wantErr && err == nil {
				t.Errorf("validateEmail(%s) expected error, got nil", tt.email)
			}
			if !tt.wantErr && err != nil {
				t.Errorf("validateEmail(%s) unexpected error: %v", tt.email, err)
			}
		})
	}
}

func TestFileExists(t *testing.T) {
	// Test with existing file
	exists := fileExists("root.go")
	if !exists {
		t.Error("expected root.go to exist")
	}

	// Test with non-existing file
	exists = fileExists("nonexistent-file-12345.txt")
	if exists {
		t.Error("expected nonexistent file to not exist")
	}
}
