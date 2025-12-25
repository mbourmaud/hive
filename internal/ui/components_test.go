package ui

import (
	"strings"
	"testing"
)

// TestHeader tests header rendering
func TestHeader(t *testing.T) {
	tests := []struct {
		name     string
		emoji    string
		title    string
		contains []string
	}{
		{
			name:     "basic header",
			emoji:    "🐝",
			title:    "Hive",
			contains: []string{"🐝", "Hive"},
		},
		{
			name:     "empty emoji",
			emoji:    "",
			title:    "Test",
			contains: []string{"Test"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := Header(tt.emoji, tt.title)
			for _, s := range tt.contains {
				if !strings.Contains(result, s) {
					t.Errorf("Header(%q, %q) = %q, missing %q", tt.emoji, tt.title, result, s)
				}
			}
		})
	}
}

// TestSuccess tests success message rendering
func TestSuccess(t *testing.T) {
	result := Success("Operation completed")
	if !strings.Contains(result, "Operation completed") {
		t.Errorf("Success() missing message")
	}
	if !strings.Contains(result, "✨") {
		t.Errorf("Success() missing emoji")
	}
}

// TestWarning tests warning message rendering
func TestWarning(t *testing.T) {
	result := Warning("Be careful")
	if !strings.Contains(result, "Be careful") {
		t.Errorf("Warning() missing message")
	}
	if !strings.Contains(result, "⚠️") {
		t.Errorf("Warning() missing emoji")
	}
}

// TestError tests error message rendering
func TestError(t *testing.T) {
	result := Error("Something failed")
	if !strings.Contains(result, "Something failed") {
		t.Errorf("Error() missing message")
	}
	if !strings.Contains(result, "❌") {
		t.Errorf("Error() missing emoji")
	}
}

// TestErrorBox tests error box rendering
func TestErrorBox(t *testing.T) {
	tests := []struct {
		name     string
		title    string
		content  string
		contains []string
	}{
		{
			name:     "with title and content",
			title:    "Error Title",
			content:  "Error details here",
			contains: []string{"Error Title", "Error details here"},
		},
		{
			name:     "with empty title",
			title:    "",
			content:  "Just content",
			contains: []string{"Error", "Just content"},
		},
		{
			name:     "with empty content",
			title:    "Title Only",
			content:  "",
			contains: []string{"Title Only"},
		},
		{
			name:     "with multiline content",
			title:    "Multi",
			content:  "Line 1\nLine 2\nLine 3",
			contains: []string{"Multi", "Line 1", "Line 2"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := ErrorBox(tt.title, tt.content)
			for _, s := range tt.contains {
				if !strings.Contains(result, s) {
					t.Errorf("ErrorBox(%q, %q) missing %q", tt.title, tt.content, s)
				}
			}
		})
	}
}

// TestErrorBox_LongContent tests content truncation
func TestErrorBox_LongContent(t *testing.T) {
	longLine := strings.Repeat("x", 100)
	result := ErrorBox("Title", longLine)

	// Should contain truncation indicator
	if !strings.Contains(result, "...") {
		t.Error("ErrorBox() should truncate long lines")
	}
}

// TestInfoBox tests info box rendering
func TestInfoBox(t *testing.T) {
	tests := []struct {
		name     string
		title    string
		content  string
		contains []string
	}{
		{
			name:     "with title and content",
			title:    "Information",
			content:  "Some info",
			contains: []string{"Information", "Some info", "ℹ️"},
		},
		{
			name:     "empty title defaults to Info",
			title:    "",
			content:  "Content",
			contains: []string{"Info", "Content"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := InfoBox(tt.title, tt.content)
			for _, s := range tt.contains {
				if !strings.Contains(result, s) {
					t.Errorf("InfoBox(%q, %q) missing %q", tt.title, tt.content, s)
				}
			}
		})
	}
}

// TestSuccessBox tests success box rendering
func TestSuccessBox(t *testing.T) {
	tests := []struct {
		name     string
		title    string
		content  string
		contains []string
	}{
		{
			name:     "with title and content",
			title:    "Done",
			content:  "All good",
			contains: []string{"Done", "All good", "✨"},
		},
		{
			name:     "empty title defaults to Success",
			title:    "",
			content:  "Content",
			contains: []string{"Success", "Content"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := SuccessBox(tt.title, tt.content)
			for _, s := range tt.contains {
				if !strings.Contains(result, s) {
					t.Errorf("SuccessBox(%q, %q) missing %q", tt.title, tt.content, s)
				}
			}
		})
	}
}

// TestCheckMark tests checkmark rendering
func TestCheckMark(t *testing.T) {
	tests := []struct {
		name     string
		label    string
		contains string
	}{
		{
			name:     "with label",
			label:    "Done",
			contains: "Done",
		},
		{
			name:     "without label",
			label:    "",
			contains: "✓",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := CheckMark(tt.label)
			if !strings.Contains(result, tt.contains) {
				t.Errorf("CheckMark(%q) = %q, missing %q", tt.label, result, tt.contains)
			}
			if !strings.Contains(result, "✓") {
				t.Errorf("CheckMark(%q) missing checkmark", tt.label)
			}
		})
	}
}

// TestProgressLine tests progress line rendering
func TestProgressLine(t *testing.T) {
	tests := []struct {
		name     string
		label    string
		status   string
		contains string
	}{
		{"success with checkmark", "Loading", "✓", "✓"},
		{"success with OK", "Loading", "OK", "✓"},
		{"success with ok", "Loading", "ok", "✓"},
		{"success with success", "Loading", "success", "✓"},
		{"fail with X", "Loading", "✗", "✗"},
		{"fail with FAIL", "Loading", "FAIL", "✗"},
		{"fail with fail", "Loading", "fail", "✗"},
		{"fail with error", "Loading", "error", "✗"},
		{"timeout uppercase", "Loading", "TIMEOUT", "TIMEOUT"},
		{"timeout lowercase", "Loading", "timeout", "TIMEOUT"},
		{"custom status", "Loading", "custom", "custom"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := ProgressLine(tt.label, tt.status)
			if !strings.Contains(result, tt.contains) {
				t.Errorf("ProgressLine(%q, %q) = %q, missing %q", tt.label, tt.status, result, tt.contains)
			}
			if !strings.Contains(result, tt.label) {
				t.Errorf("ProgressLine(%q, %q) missing label", tt.label, tt.status)
			}
		})
	}
}

// TestNextSteps tests next steps rendering
func TestNextSteps(t *testing.T) {
	steps := []Step{
		{Command: "hive start", Description: "Start Hive"},
		{Command: "hive status", Description: "Check status"},
		{Command: "hive connect queen", Description: ""},
	}

	result := NextSteps(steps)

	// Should contain "Next steps:"
	if !strings.Contains(result, "Next steps:") {
		t.Error("NextSteps() missing header")
	}

	// Should contain all commands
	for _, step := range steps {
		if !strings.Contains(result, step.Command) {
			t.Errorf("NextSteps() missing command %q", step.Command)
		}
		if step.Description != "" && !strings.Contains(result, step.Description) {
			t.Errorf("NextSteps() missing description %q", step.Description)
		}
	}
}

// TestNextSteps_Empty tests empty steps
func TestNextSteps_Empty(t *testing.T) {
	result := NextSteps([]Step{})
	if !strings.Contains(result, "Next steps:") {
		t.Error("NextSteps() with empty steps missing header")
	}
}

// TestTable tests table rendering
func TestTable(t *testing.T) {
	headers := []string{"Name", "Status", "Port"}
	rows := [][]string{
		{"queen", "running", "8080"},
		{"drone-1", "stopped", "8081"},
	}

	result := Table(headers, rows)

	// Should contain headers
	for _, h := range headers {
		if !strings.Contains(result, h) {
			t.Errorf("Table() missing header %q", h)
		}
	}

	// Should contain row data
	for _, row := range rows {
		for _, cell := range row {
			if !strings.Contains(result, cell) {
				t.Errorf("Table() missing cell %q", cell)
			}
		}
	}

	// Should contain separator
	if !strings.Contains(result, "─") {
		t.Error("Table() missing separator")
	}
}

// TestTable_EmptyHeaders tests empty headers case
func TestTable_EmptyHeaders(t *testing.T) {
	result := Table([]string{}, [][]string{{"a", "b"}})
	if result != "" {
		t.Errorf("Table() with empty headers should return empty string, got %q", result)
	}
}

// TestTable_EmptyRows tests table with no rows
func TestTable_EmptyRows(t *testing.T) {
	headers := []string{"A", "B"}
	result := Table(headers, [][]string{})

	// Should still contain headers
	for _, h := range headers {
		if !strings.Contains(result, h) {
			t.Errorf("Table() with empty rows missing header %q", h)
		}
	}
}

// TestStripANSI tests ANSI code removal
func TestStripANSI(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{
			name:     "no ANSI codes",
			input:    "plain text",
			expected: "plain text",
		},
		{
			name:     "with color code",
			input:    "\x1b[32mgreen\x1b[0m",
			expected: "green",
		},
		{
			name:     "with bold code",
			input:    "\x1b[1mbold\x1b[0m",
			expected: "bold",
		},
		{
			name:     "multiple codes",
			input:    "\x1b[31mred\x1b[0m and \x1b[34mblue\x1b[0m",
			expected: "red and blue",
		},
		{
			name:     "empty string",
			input:    "",
			expected: "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := stripANSI(tt.input)
			if result != tt.expected {
				t.Errorf("stripANSI(%q) = %q, want %q", tt.input, result, tt.expected)
			}
		})
	}
}

// TestDivider tests divider rendering
func TestDivider(t *testing.T) {
	result := Divider()

	// Should contain horizontal line character
	if !strings.Contains(result, "─") {
		t.Error("Divider() missing horizontal line")
	}

	// Should have non-empty result
	if len(result) == 0 {
		t.Error("Divider() returned empty string")
	}
}

// TestSection tests section rendering
func TestSection(t *testing.T) {
	result := Section("Title", "Content here")

	if !strings.Contains(result, "Title") {
		t.Error("Section() missing title")
	}
	if !strings.Contains(result, "Content here") {
		t.Error("Section() missing content")
	}
}

// TestSection_Empty tests section with empty content
func TestSection_Empty(t *testing.T) {
	result := Section("Title", "")

	if !strings.Contains(result, "Title") {
		t.Error("Section() with empty content missing title")
	}
}
