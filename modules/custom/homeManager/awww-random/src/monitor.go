package main

import (
	"os/exec"
	"strconv"
	"strings"
)

// Monitor represents a monitor configuration
type Monitor struct {
	width  int
	height int
	x      int
	y      int
}

// getHyprlandMonitors parses monitor information from hyprctl
func getHyprlandMonitors() []Monitor {
	cmd := exec.Command("hyprctl", "monitors")
	output, err := cmd.Output()
	if err != nil {
		return nil
	}

	var monitors []Monitor

	for line := range strings.SplitSeq(string(output), "\n") {
		// Look for lines like "	1920x1080@60.00000 at 1920x0"
		if strings.Contains(line, "@") && strings.Contains(line, " at ") {
			parts := strings.Fields(strings.TrimSpace(line))
			if len(parts) >= 3 {
				monitor := parseMonitorLine(parts)
				if monitor != nil {
					monitors = append(monitors, *monitor)
				}
			}
		}
	}

	return monitors
}

// parseMonitorLine parses a single monitor line from hyprctl output
func parseMonitorLine(parts []string) *Monitor {
	// Parse resolution (e.g., "1920x1080@60.00000")
	resPart := strings.Split(parts[0], "@")[0]
	resCoords := strings.Split(resPart, "x")
	if len(resCoords) != 2 {
		return nil
	}

	width, err1 := strconv.Atoi(resCoords[0])
	height, err2 := strconv.Atoi(resCoords[1])
	if err1 != nil || err2 != nil {
		return nil
	}

	// Find "at" keyword and parse position
	var posX, posY int
	for i, part := range parts {
		if part == "at" && i+1 < len(parts) {
			posCoords := strings.Split(parts[i+1], "x")
			if len(posCoords) == 2 {
				posX, _ = strconv.Atoi(posCoords[0])
				posY, _ = strconv.Atoi(posCoords[1])
			}
			break
		}
	}

	return &Monitor{
		width:  width,
		height: height,
		x:      posX,
		y:      posY,
	}
}

// calculateLocalPosition converts global mouse coordinates to local monitor coordinates
func calculateLocalPosition(mouseX, mouseY int, monitors []Monitor) (int, int) {
	// Find the monitor containing the mouse cursor
	for _, monitor := range monitors {
		if isMouseOnMonitor(mouseX, mouseY, monitor) {
			return convertToLocalCoordinates(mouseX, mouseY, monitor)
		}
	}

	// Fallback to center of first monitor
	if len(monitors) > 0 {
		monitor := monitors[0]
		return monitor.width / 2, monitor.height / 2
	}

	return 0, 0
}

// isMouseOnMonitor checks if mouse coordinates fall within monitor bounds
func isMouseOnMonitor(mouseX, mouseY int, monitor Monitor) bool {
	return mouseX >= monitor.x && mouseX < monitor.x+monitor.width &&
		mouseY >= monitor.y && mouseY < monitor.y+monitor.height
}

// convertToLocalCoordinates converts global coordinates to local monitor coordinates
func convertToLocalCoordinates(mouseX, mouseY int, monitor Monitor) (int, int) {
	// Convert to local coordinates within this monitor
	localX := mouseX - monitor.x
	localY := mouseY - monitor.y

	// Convert Y coordinate from top to bottom within this monitor
	bottomY := monitor.height - localY

	return localX, bottomY
}
