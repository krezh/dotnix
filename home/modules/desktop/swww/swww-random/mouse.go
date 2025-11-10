package main

import (
	"os/exec"
	"strconv"
	"strings"
)

// getMousePosition attempts to get the current mouse position for Wayland
func getMousePosition() string {
	// Try Hyprland
	if pos := getMousePositionHyprland(); pos != "" {
		return pos
	}

	// Try Sway (placeholder for future implementation)
	if pos := getMousePositionSway(); pos != "" {
		return pos
	}

	return ""
}

// getMousePositionHyprland gets mouse position using hyprctl for Hyprland
func getMousePositionHyprland() string {
	// Get cursor position
	cmd := exec.Command("hyprctl", "cursorpos")
	output, err := cmd.Output()
	if err != nil {
		return ""
	}

	// Parse cursor position "x, y"
	pos := strings.TrimSpace(string(output))
	coords := strings.Split(pos, ", ")
	if len(coords) != 2 {
		return ""
	}

	mouseX, err1 := strconv.Atoi(strings.TrimSpace(coords[0]))
	mouseY, err2 := strconv.Atoi(strings.TrimSpace(coords[1]))

	if err1 != nil || err2 != nil {
		return ""
	}

	// Get monitor information and convert to local coordinates
	monitors := getHyprlandMonitors()
	if len(monitors) == 0 {
		return ""
	}

	localX, localY := calculateLocalPosition(mouseX, mouseY, monitors)
	return formatPosition(localX, localY)
}

// getMousePositionSway gets mouse position using swaymsg for Sway
func getMousePositionSway() string {
	// Placeholder - Sway doesn't easily expose cursor position
	return ""
}

// formatPosition formats coordinates as "x,y" string
func formatPosition(x, y int) string {
	return strconv.Itoa(x) + "," + strconv.Itoa(y)
}
