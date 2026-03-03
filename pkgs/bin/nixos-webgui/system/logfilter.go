package system

import (
	"regexp"
	"strings"
)

var (
	// ANSI escape sequences (colors, cursor movement, etc.)
	ansiRegex = regexp.MustCompile(`\x1b\[[0-9;]*[a-zA-Z]`)

	// Unicode box drawing and progress characters
	boxDrawingChars = []string{
		"┏", "┃", "┣", "┗", "━", "│", "├", "└", "┌", "┐", "┘", "┤", "┬", "┴", "┼",
		"▪", "▫", "■", "□", "▲", "△", "▼", "▽", "◆", "◇", "○", "●", "◎", "◉",
		"⏱", "⚠", "✓", "✗", "⠀", "⠁", "⠂", "⠃", "⠄", "⠅", "⠆", "⠇",
	}

	// Pattern to match timestamp progress indicators like "⏱ 0s", "⏱ 1s", etc.
	timestampRegex = regexp.MustCompile(`⏱\s+\d+s`)
)

// CleanLogLine removes ANSI escape codes and noisy Unicode characters from log output.
func CleanLogLine(line string) string {
	// Remove ANSI escape sequences
	line = ansiRegex.ReplaceAllString(line, "")

	// Remove timestamp progress indicators
	line = timestampRegex.ReplaceAllString(line, "")

	// Remove box drawing characters
	for _, char := range boxDrawingChars {
		line = strings.ReplaceAll(line, char, "")
	}

	// Trim excessive whitespace
	line = strings.TrimSpace(line)

	// Skip empty lines or lines with only whitespace
	if line == "" {
		return ""
	}

	return line
}
