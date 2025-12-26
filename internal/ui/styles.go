package ui

import "github.com/charmbracelet/lipgloss"

// Base text styles
var (
	StyleBold   = lipgloss.NewStyle().Bold(true)
	StyleDim    = lipgloss.NewStyle().Foreground(ColorDim)
	StyleItalic = lipgloss.NewStyle().Italic(true)
)

// Colored text styles
var (
	StyleCyan   = lipgloss.NewStyle().Foreground(ColorCyan)
	StyleGreen  = lipgloss.NewStyle().Foreground(ColorGreen)
	StyleYellow = lipgloss.NewStyle().Foreground(ColorYellow)
	StyleOrange = lipgloss.NewStyle().Foreground(ColorOrange)
	StyleRed    = lipgloss.NewStyle().Foreground(ColorRed)
)

// Semantic styles (combining base and color)
var (
	StyleHeader  = StyleBold.Copy().Foreground(ColorYellow) // Bee yellow for all headers
	StyleSuccess = StyleBold.Copy().Foreground(ColorGreen)
	StyleWarning = StyleBold.Copy().Foreground(ColorYellow)
	StyleError   = StyleBold.Copy().Foreground(ColorOrange)
)

// Component styles
var (
	StyleCommand = StyleCyan.Copy()
	StyleComment = StyleDim.Copy()
)

// Box styles
var (
	ErrorBoxStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(ColorOrange).
			Padding(0, 1).
			Bold(true).
			MaxWidth(80)

	InfoBoxStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(ColorCyan).
			Padding(0, 1).
			MaxWidth(80)

	SuccessBoxStyle = lipgloss.NewStyle().
			Border(lipgloss.DoubleBorder()).
			BorderForeground(ColorGreen).
			Padding(1, 2).
			Bold(true).
			MaxWidth(80)
)

// Table styles
var (
	TableHeaderStyle = StyleBold.Copy().
				Foreground(ColorCyan).
				PaddingRight(2)

	TableCellStyle = lipgloss.NewStyle().
			PaddingRight(2)
)
