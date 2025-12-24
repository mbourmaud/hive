package ui

import (
	"fmt"
	"strings"
)

// Header renders a header with emoji and title
func Header(emoji, title string) string {
	text := emoji + " " + title
	return StyleHeader.Render(text) + "\n"
}

// Success renders a success message with emoji
func Success(message string) string {
	return StyleSuccess.Render("✨ " + message)
}

// Warning renders a warning message with emoji
func Warning(message string) string {
	return StyleWarning.Render("⚠️  " + message)
}

// Error renders an error message
func Error(message string) string {
	return StyleError.Render("❌ " + message)
}

// ErrorBox renders content in an error box with optional title
func ErrorBox(title, content string) string {
	if title == "" {
		title = "Error"
	}

	// Split content into lines and wrap if needed
	lines := strings.Split(strings.TrimSpace(content), "\n")
	wrappedLines := []string{}

	maxWidth := 76 // 80 - padding
	for _, line := range lines {
		if len(line) > maxWidth {
			line = line[:maxWidth-3] + "..."
		}
		wrappedLines = append(wrappedLines, line)
	}

	// Build the box content
	titleLine := StyleError.Render("⚠️  " + title)
	contentText := strings.Join(wrappedLines, "\n")

	fullContent := titleLine
	if contentText != "" {
		fullContent += "\n\n" + contentText
	}

	return "\n" + ErrorBoxStyle.Render(fullContent) + "\n"
}

// InfoBox renders content in an info box
func InfoBox(title, content string) string {
	if title == "" {
		title = "Info"
	}

	titleLine := StyleCyan.Render("ℹ️  " + title)
	fullContent := titleLine

	if content != "" {
		fullContent += "\n\n" + content
	}

	return "\n" + InfoBoxStyle.Render(fullContent) + "\n"
}

// SuccessBox renders content in a success box
func SuccessBox(title, content string) string {
	if title == "" {
		title = "Success"
	}

	titleLine := StyleSuccess.Render("✨ " + title)
	fullContent := titleLine

	if content != "" {
		fullContent += "\n\n" + content
	}

	return "\n" + SuccessBoxStyle.Render(fullContent) + "\n"
}

// CheckMark renders a green checkmark with optional label
func CheckMark(label string) string {
	if label == "" {
		return StyleGreen.Render("✓")
	}
	return StyleGreen.Render("✓ " + label)
}

// ProgressLine renders a progress line like "label... ✓"
func ProgressLine(label, status string) string {
	dimLabel := StyleDim.Render(label)

	switch status {
	case "✓", "OK", "ok", "success":
		return fmt.Sprintf("  %s... %s\n", dimLabel, StyleGreen.Render("✓"))
	case "✗", "FAIL", "fail", "error":
		return fmt.Sprintf("  %s... %s\n", dimLabel, StyleRed.Render("✗"))
	case "TIMEOUT", "timeout":
		return fmt.Sprintf("  %s... %s\n", dimLabel, StyleYellow.Render("TIMEOUT"))
	default:
		return fmt.Sprintf("  %s...%s\n", dimLabel, status)
	}
}

// Step represents a step in the "Next Steps" section
type Step struct {
	Command     string
	Description string
}

// NextSteps renders a "Next steps:" section with commands
func NextSteps(steps []Step) string {
	var b strings.Builder

	b.WriteString("\n" + StyleBold.Render("Next steps:") + "\n")

	for _, step := range steps {
		command := StyleCommand.Render(step.Command)
		desc := ""
		if step.Description != "" {
			desc = StyleComment.Render("  # " + step.Description)
		}
		b.WriteString("  " + command + desc + "\n")
	}

	return b.String()
}

// Table renders a simple table
func Table(headers []string, rows [][]string) string {
	if len(headers) == 0 {
		return ""
	}

	var b strings.Builder

	// Calculate column widths
	widths := make([]int, len(headers))
	for i, h := range headers {
		widths[i] = len(h)
	}
	for _, row := range rows {
		for i, cell := range row {
			if i < len(widths) {
				// Strip ANSI codes for width calculation
				cleanCell := stripANSI(cell)
				if len(cleanCell) > widths[i] {
					widths[i] = len(cleanCell)
				}
			}
		}
	}

	// Render header
	for i, h := range headers {
		padded := h + strings.Repeat(" ", widths[i]-len(h))
		b.WriteString(TableHeaderStyle.Render(padded))
	}
	b.WriteString("\n")

	// Render separator
	for _, w := range widths {
		b.WriteString(strings.Repeat("─", w+2))
	}
	b.WriteString("\n")

	// Render rows
	for _, row := range rows {
		for i, cell := range row {
			if i < len(widths) {
				cleanCell := stripANSI(cell)
				padding := widths[i] - len(cleanCell)
				b.WriteString(cell)
				b.WriteString(strings.Repeat(" ", padding))
				b.WriteString("  ")
			}
		}
		b.WriteString("\n")
	}

	return b.String()
}

// stripANSI removes ANSI escape codes from a string for width calculation
func stripANSI(str string) string {
	// Simple ANSI stripper - matches \x1b[...m patterns
	result := ""
	inEscape := false

	for i := 0; i < len(str); i++ {
		if str[i] == '\x1b' && i+1 < len(str) && str[i+1] == '[' {
			inEscape = true
			i++ // skip [
			continue
		}

		if inEscape {
			if (str[i] >= 'a' && str[i] <= 'z') || (str[i] >= 'A' && str[i] <= 'Z') {
				inEscape = false
			}
			continue
		}

		result += string(str[i])
	}

	return result
}

// Divider renders a horizontal divider
func Divider() string {
	return StyleDim.Render(strings.Repeat("─", 80))
}

// Section renders a section with title and content
func Section(title, content string) string {
	var b strings.Builder
	b.WriteString("\n" + StyleBold.Render(title) + "\n")
	b.WriteString(content)
	b.WriteString("\n")
	return b.String()
}
