package main

import (
	"fmt"
	"os"
	"os/exec"

	catppuccin "github.com/catppuccin/go"
	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/log"
)

var (
	// Styles using Catppuccin Mocha colors
	titleStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color(catppuccin.Mocha.Mauve().Hex)).
			MarginTop(1).
			MarginBottom(1)

	successStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(catppuccin.Mocha.Green().Hex)).
			Bold(true)

	errorStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(catppuccin.Mocha.Red().Hex)).
			Bold(true)

	infoStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(catppuccin.Mocha.Blue().Hex))

	commandStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(catppuccin.Mocha.Peach().Hex)).
			Italic(true)

	stepStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color(catppuccin.Mocha.Sapphire().Hex)).
			Bold(true)
)

func main() {
	if err := run(); err != nil {
		fmt.Println(errorStyle.Render("✗ Error:"), err)
		os.Exit(1)
	}
}

func run() error {
	fmt.Println(titleStyle.Render("🚀 NixOS System Update"))

	// Get the nix-config directory (assuming this binary runs from within the repo)
	configDir, err := getConfigDir()
	if err != nil {
		return fmt.Errorf("failed to determine config directory: %w", err)
	}

	fmt.Println(infoStyle.Render(fmt.Sprintf("📂 Config directory: %s", configDir)))

	// Step 1: Git pull
	fmt.Println()
	fmt.Println(stepStyle.Render("Step 1/2:"), "Pulling latest changes from git")
	if err := gitPull(configDir); err != nil {
		return fmt.Errorf("git pull failed: %w", err)
	}
	fmt.Println(successStyle.Render("✓ Git pull completed"))

	// Step 2: NH OS switch
	fmt.Println()
	fmt.Println(stepStyle.Render("Step 2/2:"), "Switching NixOS configuration")
	fmt.Println(commandStyle.Render("→ Running: nh os switch"))
	if err := nhOsSwitch(configDir); err != nil {
		return fmt.Errorf("nh os switch failed: %w", err)
	}

	fmt.Println()
	fmt.Println(successStyle.Render("✓ System update completed successfully!"))
	return nil
}

func getConfigDir() (string, error) {
	// Try to get from environment variable first
	dir := os.Getenv("NH_FLAKE")
	if dir == "" {
		log.Error("NH_FLAKE", "environment variable is not set")
	}
	return dir, nil
}

func gitPull(dir string) error {
	cmd := exec.Command("git", "pull")
	cmd.Dir = dir
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	return cmd.Run()
}

func nhOsSwitch(dir string) error {
	cmd := exec.Command("nh", "os", "switch")
	cmd.Dir = dir
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	return cmd.Run()
}
