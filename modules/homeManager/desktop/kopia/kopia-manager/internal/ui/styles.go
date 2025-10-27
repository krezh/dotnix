package ui

import (
	"fmt"

	"github.com/charmbracelet/lipgloss"
)

// Centralized lipgloss styles for consistent UI
// Using Catppuccin Mocha color palette

var (
	// Message styles
	SuccessStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#a6e3a1")). // Green
			Bold(true)

	ErrorStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#f38ba8")). // Red
			Bold(true)

	WarningStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#fab387")). // Peach
			Bold(true)

	InfoStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#f9e2af")). // Yellow
			Italic(true)

	PromptStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#f38ba8")). // Red
			Bold(true)

	SummaryStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#89b4fa")). // Blue
			Bold(true)

	NoteStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#f9e2af")). // Yellow
			Italic(true)

	HelpStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#89dceb")). // Sky
			Italic(true)

	// List item style
	ItemStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#fab387")). // Peach
			PaddingLeft(2)

	// Highlight style for IDs, paths, etc.
	HighlightStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#cba6f7")). // Mauve
			Bold(true)

	// Command style
	CommandStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#74c7ec")). // Sapphire
			Bold(true)
)

// Helper functions for common message patterns

// Success prints a success message in green
func Success(message string) {
	fmt.Println(SuccessStyle.Render(message))
}

// Successf prints a formatted success message in green
func Successf(format string, args ...interface{}) {
	Success(fmt.Sprintf(format, args...))
}

// Error prints an error message in red
func Error(message string) {
	fmt.Println(ErrorStyle.Render(message))
}

// Errorf prints a formatted error message in red
func Errorf(format string, args ...interface{}) {
	Error(fmt.Sprintf(format, args...))
}

// Warning prints a warning message in peach
func Warning(message string) {
	fmt.Println(WarningStyle.Render(message))
}

// Warningf prints a formatted warning message in peach
func Warningf(format string, args ...interface{}) {
	Warning(fmt.Sprintf(format, args...))
}

// Info prints an info message in yellow
func Info(message string) {
	fmt.Println(InfoStyle.Render(message))
}

// Infof prints a formatted info message in yellow
func Infof(format string, args ...interface{}) {
	Info(fmt.Sprintf(format, args...))
}

// Summary prints a summary message in blue
func Summary(message string) {
	fmt.Println(SummaryStyle.Render(message))
}

// Summaryf prints a formatted summary message in blue
func Summaryf(format string, args ...interface{}) {
	Summary(fmt.Sprintf(format, args...))
}

// Note prints a note message in yellow italic
func Note(message string) {
	fmt.Println(NoteStyle.Render(message))
}

// Notef prints a formatted note message in yellow italic
func Notef(format string, args ...interface{}) {
	Note(fmt.Sprintf(format, args...))
}

// Help prints a help message in sky blue italic
func Help(message string) {
	fmt.Println(HelpStyle.Render(message))
}

// Helpf prints a formatted help message in sky blue italic
func Helpf(format string, args ...interface{}) {
	Help(fmt.Sprintf(format, args...))
}

// Prompt prints a prompt message in red bold and returns the styled string
func Prompt(message string) string {
	return PromptStyle.Render(message)
}

// Promptf prints a formatted prompt message in red bold and returns the styled string
func Promptf(format string, args ...interface{}) string {
	return Prompt(fmt.Sprintf(format, args...))
}

// Item prints a list item with indentation
func Item(message string) {
	fmt.Println(ItemStyle.Render(message))
}

// Itemf prints a formatted list item with indentation
func Itemf(format string, args ...interface{}) {
	Item(fmt.Sprintf(format, args...))
}
