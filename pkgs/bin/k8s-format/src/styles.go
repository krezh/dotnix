package main

import (
	"os"
	"path/filepath"

	"charm.land/lipgloss/v2"
)

var (
	bold   = lipgloss.NewStyle().Bold(true)
	muted  = lipgloss.NewStyle().Foreground(lipgloss.Color("#7f849c"))
	accent = lipgloss.NewStyle().Foreground(lipgloss.Color("#89b4fa")).Bold(true)
	green  = lipgloss.NewStyle().Foreground(lipgloss.Color("#a6e3a1")).Bold(true)
	red    = lipgloss.NewStyle().Foreground(lipgloss.Color("#f38ba8")).Bold(true)

	successIcon = green.Render("✓")
	errorIcon   = red.Render("✗")
)

// relPath returns path relative to base, falling back to cwd-relative, then absolute.
func relPath(path, base string) string {
	if base != "" {
		if rel, err := filepath.Rel(base, path); err == nil {
			return rel
		}
	}
	if cwd, err := os.Getwd(); err == nil {
		if rel, err := filepath.Rel(cwd, path); err == nil {
			return rel
		}
	}
	return path
}
