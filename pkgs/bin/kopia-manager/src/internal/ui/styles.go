package ui

import (
	"fmt"

	"charm.land/lipgloss/v2"
)

// Centralized lipgloss styles for consistent UI
// Using Catppuccin Mocha color palette

// Catppuccin Mocha color palette
const (
	ColorMauve    = "#cba6f7"
	ColorGreen    = "#a6e3a1"
	ColorRed      = "#f38ba8"
	ColorPeach    = "#fab387"
	ColorYellow   = "#f9e2af"
	ColorBlue     = "#89b4fa"
	ColorSky      = "#89dceb"
	ColorSapphire = "#74c7ec"
	ColorText     = "#cdd6f4"
	ColorSubtext1 = "#bac2de"
)

var (
	// Message styles
	SuccessStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(ColorGreen)).
			Bold(true)

	ErrorStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(ColorRed)).
			Bold(true)

	WarningStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(ColorPeach)).
			Bold(true)

	InfoStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(ColorYellow)).
			Italic(true)

	PromptStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(ColorRed)).
			Bold(true)

	SummaryStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(ColorBlue)).
			Bold(true)

	NoteStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(ColorYellow)).
			Italic(true)

	HelpStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(ColorSky)).
			Italic(true)

	// List item style
	ItemStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(ColorPeach)).
			PaddingLeft(2)

	// Highlight style for IDs, paths, etc.
	HighlightStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(ColorMauve)).
			Bold(true)

	// Command style
	CommandStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(ColorSapphire)).
			Bold(true)

	// Table styles
	TableTitleStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color(ColorMauve))

	TableHeaderStyle = lipgloss.NewStyle().
				Bold(true).
				Foreground(lipgloss.Color(ColorMauve)).
				Align(lipgloss.Center).
				Padding(0, 1)

	TableEvenRowStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color(ColorText)).
				Padding(0, 1)

	TableOddRowStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color(ColorSubtext1)).
				Padding(0, 1)

	TableBorderStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color(ColorBlue))

	// Progress indicator styles
	ProgressTitleStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color(ColorMauve)).
				Bold(true)

	ProgressInfoStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color(ColorSubtext1))

	SpinnerStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(ColorGreen))
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
