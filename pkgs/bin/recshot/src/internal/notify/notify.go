package notify

import (
	"os"
	"os/exec"
	"path/filepath"
)

// Notifier handles desktop notifications
type Notifier struct {
	iconPath string
}

// New creates a new notifier
func New() *Notifier {
	// Try to find the icon in various locations
	iconPath := findIcon()
	return &Notifier{
		iconPath: iconPath,
	}
}

// findIcon attempts to locate the recshot.png icon
func findIcon() string {
	// Get the executable path
	execPath, err := os.Executable()
	if err != nil {
		return ""
	}

	// Try different locations relative to the executable
	possiblePaths := []string{
		filepath.Join(filepath.Dir(execPath), "recshot.png"),
		filepath.Join(filepath.Dir(execPath), "..", "share", "recshot", "recshot.png"),
		filepath.Join(filepath.Dir(execPath), "..", "share", "pixmaps", "recshot.png"),
		"/usr/share/pixmaps/recshot.png",
		"/usr/local/share/pixmaps/recshot.png",
	}

	for _, path := range possiblePaths {
		if _, err := os.Stat(path); err == nil {
			return path
		}
	}

	return ""
}

// Send sends a desktop notification
func (n *Notifier) Send(message string) error {
	args := []string{"-t", "5000", "Recshot", message}
	if n.iconPath != "" {
		args = append([]string{"-i", n.iconPath}, args...)
	}
	return exec.Command("notify-send", args...).Run()
}

// SendSuccess sends a success notification
func (n *Notifier) SendSuccess(message string) error {
	return n.Send("✓ " + message)
}

// SendError sends an error notification
func (n *Notifier) SendError(message string) error {
	args := []string{"-u", "critical", "-t", "5000", "Recshot", "✗ " + message}
	if n.iconPath != "" {
		args = append([]string{"-i", n.iconPath}, args...)
	}
	return exec.Command("notify-send", args...).Run()
}
