package ui

import (
	"os"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/lipgloss/table"
	"github.com/charmbracelet/x/term"
)

// Table styles using Catppuccin Mocha palette
var (
	tableTitleStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("#cba6f7")) // Mauve

	tableHeaderStyle = lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color("#cba6f7")). // Mauve
				Align(lipgloss.Center).
				Padding(0, 1)

	tableEvenRowStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color("#cdd6f4")). // Text
				Padding(0, 1)

	tableOddRowStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color("#bac2de")). // Subtext1
				Padding(0, 1)

	tableBorderStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color("#89b4fa")) // Blue
)

// RenderTable creates a styled table with optional title embedded in the top border
func RenderTable(title string, headers []string, rows [][]string) string {
	// Get terminal width for auto-sizing
	width, _, err := term.GetSize(os.Stdout.Fd())
	if err != nil || width <= 0 {
		width = 120 // Default fallback width
	}

	// Create table with lipgloss/table package
	t := table.New().
		Border(lipgloss.ThickBorder()).
		BorderStyle(tableBorderStyle).
		Width(width). // Auto-size columns to fit terminal width
		StyleFunc(func(row, col int) lipgloss.Style {
			if row == table.HeaderRow {
				return tableHeaderStyle
			}
			if row%2 == 0 {
				return tableEvenRowStyle
			}
			return tableOddRowStyle
		}).
		Headers(headers...).
		Rows(rows...)

	tableOutput := t.Render()

	// If no title, just return the table
	if title == "" {
		return tableOutput
	}

	// Embed title in the top border line
	lines := strings.Split(tableOutput, "\n")
	if len(lines) == 0 {
		return tableOutput
	}

	// Get the original top border and its width
	borderWidth := lipgloss.Width(lines[0])

	// Create title
	titleStyled := tableTitleStyle.Render(title)
	titleWidth := lipgloss.Width(titleStyled)

	// Calculate how many dashes on each side
	remainingWidth := borderWidth - titleWidth - 2 // -2 for the corner characters
	if remainingWidth < 0 {
		remainingWidth = 0
	}

	leftDashes := remainingWidth / 2
	rightDashes := remainingWidth - leftDashes

	// Build new top border with thick characters: ┏━━━ title ━━━┓
	newTopBorder := tableBorderStyle.Render("┏"+strings.Repeat("━", leftDashes)) +
		titleStyled +
		tableBorderStyle.Render(strings.Repeat("━", rightDashes)+"┓")

	// Replace the first line
	lines[0] = newTopBorder

	return strings.Join(lines, "\n")
}
