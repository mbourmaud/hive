package ui

import "github.com/charmbracelet/lipgloss"

// Color palette for the Hive CLI
var (
	// Primary colors
	ColorCyan   = lipgloss.AdaptiveColor{Light: "#00CED1", Dark: "#00FFFF"}
	ColorGreen  = lipgloss.AdaptiveColor{Light: "#00A000", Dark: "#00FF00"}
	ColorYellow = lipgloss.AdaptiveColor{Light: "#FFD700", Dark: "#FFFF00"} // Bright gold/yellow (bee color)
	ColorOrange = lipgloss.AdaptiveColor{Light: "#FF6B00", Dark: "#FF8C00"}
	ColorRed    = lipgloss.AdaptiveColor{Light: "#CC0000", Dark: "#FF0000"}

	// Neutral colors
	ColorGray = lipgloss.AdaptiveColor{Light: "#888888", Dark: "#666666"}
	ColorDim  = lipgloss.AdaptiveColor{Light: "#888888", Dark: "#555555"}
)
