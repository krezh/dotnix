package ui

import (
	"fmt"
	"regexp"
	"strings"
)

// Dynamic is a constant used to indicate dynamic column sizing
const Dynamic = -1

// TableBuilder helps create formatted tables with box drawing characters
type TableBuilder struct {
	title        string
	columns      []TableColumn
	rows         [][]string
	columnWidths []int
	dynamicCols  []bool
}

type TableColumn struct {
	Header   string
	Width    int
	MinWidth int
}

// NewTableBuilder creates a new table builder
func NewTableBuilder(title string) *TableBuilder {
	return &TableBuilder{
		title: title,
	}
}

// NewAutoSizeTableBuilder creates a new table builder with automatic column sizing
func NewAutoSizeTableBuilder(title string) *TableBuilder {
	return &TableBuilder{
		title: title,
	}
}

// AddColumn adds a column to the table
// Use Dynamic constant for auto-sizing, or specify a fixed/minimum width
func (tb *TableBuilder) AddColumn(header string, width int) {
	if width == Dynamic {
		minWidth := len(header)
		tb.columns = append(tb.columns, TableColumn{Header: header, MinWidth: minWidth})
		tb.columnWidths = append(tb.columnWidths, minWidth)
		tb.dynamicCols = append(tb.dynamicCols, true)
	} else {
		tb.columns = append(tb.columns, TableColumn{Header: header, Width: width})
		tb.columnWidths = append(tb.columnWidths, width)
		tb.dynamicCols = append(tb.dynamicCols, false)
	}
}

// AddAutoColumn adds a column with automatic sizing (minimum width specified)
// Use Dynamic constant for pure auto-sizing, or specify a minimum width
func (tb *TableBuilder) AddAutoColumn(header string, minWidth int) {
	actualMinWidth := minWidth
	if minWidth == Dynamic {
		actualMinWidth = len(header) // Use header length as minimum
	}
	tb.columns = append(tb.columns, TableColumn{Header: header, MinWidth: actualMinWidth})
	tb.columnWidths = append(tb.columnWidths, actualMinWidth)
	tb.dynamicCols = append(tb.dynamicCols, true)
}

// AddRow adds a row of data to the table
func (tb *TableBuilder) AddRow(values ...string) {
	tb.rows = append(tb.rows, values)
}

// Build creates the formatted table string
func (tb *TableBuilder) Build() string {
	if len(tb.columns) == 0 {
		return ""
	}

	// Calculate dynamic column widths for dynamic columns only
	tb.CalculateDynamicWidths()

	// Now rebuild header row with final column widths
	headerRow := "│"
	for i, col := range tb.columns {
		headerRow += fmt.Sprintf(" %-*s ", tb.columnWidths[i], col.Header)
		if i < len(tb.columns)-1 {
			headerRow += "│"
		}
	}
	headerRow += "│"
	headerRowLen := len([]rune(headerRow))

	var result strings.Builder

	// Box drawing characters
	vertical := "│"
	horizontal := "─"
	topLeft, topRight := "╭", "╮"
	bottomLeft, bottomRight := "╰", "╯"

	// Build top border with title using final header row length
	titleLen := len([]rune(tb.title))
	if titleLen >= headerRowLen-2 {
		if headerRowLen > 5 {
			tb.title = string([]rune(tb.title)[:headerRowLen-5]) + "..."
		} else {
			tb.title = string([]rune(tb.title)[:headerRowLen-2])
		}
		titleLen = len([]rune(tb.title))
	}
	pad := headerRowLen - 2 - titleLen
	leftPad := pad / 2
	rightPad := pad - leftPad
	topBorder := topLeft + strings.Repeat(horizontal, leftPad) + tb.title + strings.Repeat(horizontal, rightPad) + topRight

	result.WriteString(topBorder + "\n")
	result.WriteString(headerRow + "\n")

	// Header separator - align with column positions
	headerSep := "├"
	for i, width := range tb.columnWidths {
		headerSep += strings.Repeat(horizontal, 1+width+1)
		if i < len(tb.columnWidths)-1 {
			headerSep += "┬"
		}
	}
	headerSep += "┤"
	result.WriteString(headerSep + "\n")

	// Add data rows
	for _, row := range tb.rows {
		rowStr := vertical
		for i, width := range tb.columnWidths {
			value := ""
			if i < len(row) {
				value = row[i]
			}
			valueVisibleWidth := VisibleWidth(value)
			if valueVisibleWidth > width && width > 0 {
				if width > 3 {
					// For colored text, we need to truncate more carefully
					// This is a simple truncation that may break in the middle of color codes
					// For now, we'll use a basic approach
					if valueVisibleWidth == len(value) {
						// No color codes, safe to truncate normally
						value = value[:width-3] + "..."
					} else {
						// Has color codes, use visible width truncation
						value = TruncateWithColors(value, width-3) + "..."
					}
				} else {
					if valueVisibleWidth == len(value) {
						value = value[:width]
					} else {
						value = TruncateWithColors(value, width)
					}
				}
			} else if width <= 0 {
				value = ""
			}
			// Pad colored text properly to fill column width
			if valueVisibleWidth != len(value) && valueVisibleWidth < width {
				// Text has color codes and needs padding
				padding := strings.Repeat(" ", width-valueVisibleWidth)
				rowStr += fmt.Sprintf(" %s%s ", value, padding)
			} else {
				rowStr += fmt.Sprintf(" %-*s ", width, value)
			}
			if i < len(tb.columnWidths)-1 {
				rowStr += vertical
			}
		}
		rowStr += vertical
		result.WriteString(rowStr + "\n")
	}

	// Bottom border - use the same length as top border
	bottomBorder := bottomLeft + strings.Repeat(horizontal, headerRowLen-2) + bottomRight
	result.WriteString(bottomBorder + "\n")

	return result.String()
}

// visibleWidth calculates the visible width of text, ignoring ANSI color codes
func VisibleWidth(text string) int {
	// Regular expression to match ANSI escape sequences
	ansiRegex := regexp.MustCompile(`\x1b\[[0-9;]*m`)
	// Remove all ANSI codes and measure the remaining text
	cleanText := ansiRegex.ReplaceAllString(text, "")
	return len([]rune(cleanText))
}

// calculateDynamicWidths calculates optimal column widths based on content for dynamic columns only
func (tb *TableBuilder) CalculateDynamicWidths() {
	for i, col := range tb.columns {
		// Only recalculate width for dynamic columns
		if i < len(tb.dynamicCols) && tb.dynamicCols[i] {
			maxWidth := VisibleWidth(col.Header)

			// Check minimum width
			maxWidth = max(maxWidth, col.MinWidth)

			// Check all row data for this column
			for _, row := range tb.rows {
				if i < len(row) {
					cellWidth := VisibleWidth(row[i])
					maxWidth = max(maxWidth, cellWidth)
				}
			}

			// Ensure minimum width of 1
			maxWidth = max(maxWidth, 1)

			tb.columnWidths[i] = maxWidth
		}
	}
}

// truncateWithColors truncates text with ANSI color codes while preserving colors
func TruncateWithColors(text string, maxWidth int) string {
	if maxWidth <= 0 {
		return ""
	}

	var result strings.Builder
	var visibleCount int
	i := 0

	for i < len(text) && visibleCount < maxWidth {
		// Check if we're at the start of an ANSI sequence
		if text[i] == '\x1b' && i+1 < len(text) && text[i+1] == '[' {
			// Find the end of the ANSI sequence
			end := i + 2
			for end < len(text) && text[end] != 'm' {
				end++
			}
			if end < len(text) {
				end++ // Include the 'm'
				// Add the entire ANSI sequence to result
				result.WriteString(text[i:end])
				i = end
			} else {
				// Malformed ANSI sequence, treat as regular character
				result.WriteByte(text[i])
				i++
				visibleCount++
			}
		} else {
			// Regular character
			result.WriteByte(text[i])
			i++
			visibleCount++
		}
	}

	return result.String()
}
