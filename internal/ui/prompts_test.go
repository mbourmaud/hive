//go:build !windows
// +build !windows

package ui

import (
	"errors"
	"testing"

	"github.com/AlecAivazis/survey/v2/terminal"
	"github.com/mbourmaud/hive/internal/testutil"
)

// TestPromptRequiredWithStdio tests required input prompt
func TestPromptRequiredWithStdio(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Enter name:")
			c.SendLine("John Doe")
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptRequiredWithStdio("Enter name:", stdio)
			if err != nil {
				return err
			}
			if result != "John Doe" {
				t.Errorf("expected 'John Doe', got %q", result)
			}
			return nil
		},
	)
}

// TestPromptRequiredWithStdio_WithValidator tests required input with custom validator
func TestPromptRequiredWithStdio_WithValidator(t *testing.T) {
	validator := func(s string) error {
		if len(s) < 3 {
			return errors.New("must be at least 3 characters")
		}
		return nil
	}

	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Enter name:")
			c.SendLine("Joe")
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptRequiredWithStdio("Enter name:", stdio, validator)
			if err != nil {
				return err
			}
			if result != "Joe" {
				t.Errorf("expected 'Joe', got %q", result)
			}
			return nil
		},
	)
}

// TestPromptDefaultWithStdio tests prompt with default value
func TestPromptDefaultWithStdio(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Enter name:")
			c.SendLine("") // Accept default
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptDefaultWithStdio("Enter name:", "Default Name", stdio)
			if err != nil {
				return err
			}
			if result != "Default Name" {
				t.Errorf("expected 'Default Name', got %q", result)
			}
			return nil
		},
	)
}

// TestPromptDefaultWithStdio_Override tests prompt with default value when user provides input
func TestPromptDefaultWithStdio_Override(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Enter name:")
			c.SendLine("Custom Name")
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptDefaultWithStdio("Enter name:", "Default Name", stdio)
			if err != nil {
				return err
			}
			if result != "Custom Name" {
				t.Errorf("expected 'Custom Name', got %q", result)
			}
			return nil
		},
	)
}

// TestPromptOptionalWithStdio tests optional input prompt
func TestPromptOptionalWithStdio(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Description:")
			c.SendLine("Optional value")
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptOptionalWithStdio("Description:", stdio)
			if err != nil {
				return err
			}
			if result != "Optional value" {
				t.Errorf("expected 'Optional value', got %q", result)
			}
			return nil
		},
	)
}

// TestPromptOptionalWithStdio_Empty tests optional input when user enters nothing
func TestPromptOptionalWithStdio_Empty(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Description:")
			c.SendLine("")
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptOptionalWithStdio("Description:", stdio)
			if err != nil {
				return err
			}
			if result != "" {
				t.Errorf("expected empty string, got %q", result)
			}
			return nil
		},
	)
}

// TestPromptSecretWithStdio tests password input prompt
func TestPromptSecretWithStdio(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Enter token:")
			c.SendLine("secret-token-123")
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptSecretWithStdio("Enter token:", stdio)
			if err != nil {
				return err
			}
			if result != "secret-token-123" {
				t.Errorf("expected 'secret-token-123', got %q", result)
			}
			return nil
		},
	)
}

// TestPromptConfirmWithStdio_Yes tests confirm prompt with yes answer
func TestPromptConfirmWithStdio_Yes(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Continue?")
			c.SendLine("y")
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptConfirmWithStdio("Continue?", false, stdio)
			if err != nil {
				return err
			}
			if !result {
				t.Error("expected true, got false")
			}
			return nil
		},
	)
}

// TestPromptConfirmWithStdio_No tests confirm prompt with no answer
func TestPromptConfirmWithStdio_No(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Continue?")
			c.SendLine("n")
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptConfirmWithStdio("Continue?", true, stdio)
			if err != nil {
				return err
			}
			if result {
				t.Error("expected false, got true")
			}
			return nil
		},
	)
}

// TestPromptConfirmWithStdio_DefaultYes tests confirm prompt accepting default yes
func TestPromptConfirmWithStdio_DefaultYes(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Continue?")
			c.SendLine("") // Accept default
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptConfirmWithStdio("Continue?", true, stdio)
			if err != nil {
				return err
			}
			if !result {
				t.Error("expected true (default), got false")
			}
			return nil
		},
	)
}

// TestPromptConfirmWithStdio_DefaultNo tests confirm prompt accepting default no
func TestPromptConfirmWithStdio_DefaultNo(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Continue?")
			c.SendLine("") // Accept default
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptConfirmWithStdio("Continue?", false, stdio)
			if err != nil {
				return err
			}
			if result {
				t.Error("expected false (default), got true")
			}
			return nil
		},
	)
}

// TestPromptSelectWithStdio tests select prompt with first option
func TestPromptSelectWithStdio(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Choose:")
			c.SendLine("") // Select first option
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptSelectWithStdio("Choose:", []string{"Option A", "Option B", "Option C"}, stdio)
			if err != nil {
				return err
			}
			if result != "Option A" {
				t.Errorf("expected 'Option A', got %q", result)
			}
			return nil
		},
	)
}

// TestPromptMultiSelectWithStdio tests multiselect prompt
func TestPromptMultiSelectWithStdio(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Select features:")
			c.Send(" ")         // Toggle first option
			c.SendLine("")      // Confirm selection
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptMultiSelectWithStdio("Select features:", []string{"Feature A", "Feature B", "Feature C"}, stdio)
			if err != nil {
				return err
			}
			if len(result) != 1 || result[0] != "Feature A" {
				t.Errorf("expected ['Feature A'], got %v", result)
			}
			return nil
		},
	)
}

// TestPromptMultiSelectWithStdio_Empty tests multiselect with no selection
func TestPromptMultiSelectWithStdio_Empty(t *testing.T) {
	testutil.RunPromptTest(t,
		func(c testutil.ExpectConsole) {
			c.ExpectString("Select features:")
			c.SendLine("") // No selection
			c.ExpectEOF()
		},
		func(stdio terminal.Stdio) error {
			result, err := PromptMultiSelectWithStdio("Select features:", []string{"Feature A", "Feature B"}, stdio)
			if err != nil {
				return err
			}
			if len(result) != 0 {
				t.Errorf("expected empty selection, got %v", result)
			}
			return nil
		},
	)
}
