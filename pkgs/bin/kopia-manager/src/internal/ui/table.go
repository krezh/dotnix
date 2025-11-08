package ui

import (
	"os"
	"strings"
	"sync"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/lipgloss/table"
	"github.com/charmbracelet/x/term"
)

// Cache terminal width to avoid repeated system calls
var (
	cachedTermWidth int
	termWidthOnce   sync.Once
)

// getTerminalWidth returns the cached terminal width
func getTerminalWidth() int {
	termWidthOnce.Do(func() {
		width, _, err := term.GetSize(os.Stdout.Fd())
		if err != nil || width <= 0 {
			cachedTermWidth = 120 // Default fallback width
		} else {
			cachedTermWidth = width
		}
	})
	return cachedTermWidth
}

// calculateNaturalTableWidth estimates the natural width a table would take
// based on the longest cell in each column
func calculateNaturalTableWidth(headers []string, rows [][]string) int {
	if len(headers) == 0 {
		return 0
	}

	// Track max width per column
	colWidths := make([]int, len(headers))

	// Check header widths
	for i, header := range headers {
		colWidths[i] = lipgloss.Width(header)
	}

	// Check each row's cell widths
	for _, row := range rows {
		for i, cell := range row {
			if i < len(colWidths) {
				cellWidth := lipgloss.Width(cell)
				if cellWidth > colWidths[i] {
					colWidths[i] = cellWidth
				}
			}
		}
	}

	// Calculate total table width:
	// sum of column widths + padding (2 per column) + borders (1 per column + 1 final)
	totalWidth := 1 // Start with left border
	for _, width := range colWidths {
		totalWidth += width + 2 + 1 // content + padding + border
	}

	return totalWidth
}

// RenderTable creates a styled table with optional title embedded in the top border
func RenderTable(title string, headers []string, rows [][]string) string {
	// Calculate what the natural table width would be
	naturalWidth := calculateNaturalTableWidth(headers, rows)
	termWidth := getTerminalWidth()

	// Build the table with smart width handling
	t := table.New().
		Border(lipgloss.ThickBorder()).
		BorderStyle(TableBorderStyle).
		StyleFunc(func(row, col int) lipgloss.Style {
			if row == table.HeaderRow {
				return TableHeaderStyle
			}
			if row%2 == 0 {
				return TableEvenRowStyle
			}
			return TableOddRowStyle
		}).
		Headers(headers...).
		Rows(rows...)

	// If the natural width would exceed terminal width, constrain and enable wrapping
	// Otherwise, let it size naturally to content
	if naturalWidth > termWidth {
		t = t.Width(termWidth).Wrap(true)
	}

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

	// Add spacing around title for consistency
	titleWithSpaces := " " + title + " "
	titleStyled := TableTitleStyle.Render(titleWithSpaces)
	titleWidth := lipgloss.Width(titleStyled)

	// Calculate how many dashes on each side
	remainingWidth := borderWidth - titleWidth - 2 // -2 for the corner characters
	if remainingWidth < 0 {
		remainingWidth = 0
	}

	leftDashes := remainingWidth / 2
	rightDashes := remainingWidth - leftDashes

	// Build new top border with thick characters: ┏━━━ title ━━━┓
	newTopBorder := TableBorderStyle.Render("┏"+strings.Repeat("━", leftDashes)) +
		titleStyled +
		TableBorderStyle.Render(strings.Repeat("━", rightDashes)+"┓")

	// Replace the first line
	lines[0] = newTopBorder

	return strings.Join(lines, "\n")
}
