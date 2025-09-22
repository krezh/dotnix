package notify

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// Notifier handles desktop notifications
type Notifier struct {
	iconPath string
}

// New creates a new notifier
func New() *Notifier {
	return &Notifier{
		iconPath: findIcon(),
	}
}

// findIcon attempts to locate the recshot.png icon
func findIcon() string {
	execPath, err := os.Executable()
	if err != nil {
		return ""
	}

	paths := []string{
		filepath.Join(filepath.Dir(execPath), "recshot.png"),
		filepath.Join(filepath.Dir(execPath), "..", "share", "recshot", "recshot.png"),
		filepath.Join(filepath.Dir(execPath), "..", "share", "pixmaps", "recshot.png"),
		"/usr/share/pixmaps/recshot.png",
		"/usr/local/share/pixmaps/recshot.png",
	}

	for _, path := range paths {
		if _, err := os.Stat(path); err == nil {
			return path
		}
	}
	return ""
}

// send sends a desktop notification with optional action button
func (n *Notifier) send(urgency, message, url string) error {
	args := []string{"-t", "5000"}
	if n.iconPath != "" {
		args = append(args, "-i", n.iconPath)
	}
	if urgency != "" {
		args = append(args, "-u", urgency)
	}
	if url != "" {
		args = append(args, "-A", "open=Open URL")
	}
	args = append(args, "Recshot", message)

	cmd := exec.Command("notify-send", args...)

	if url != "" {
		output, err := cmd.Output()
		if err == nil {
			response := strings.TrimSpace(string(output))
			if response == "open" {
				fmt.Println("Opening URL:", url)
				n.openURL(url)
			}
		}
		return nil
	}
	return cmd.Run()
}

// openURL attempts to open a URL using available system commands
func (n *Notifier) openURL(url string) {
	// Try different commands in order of preference
	commands := []string{"xdg-open", "open", "firefox", "chromium", "google-chrome"}

	fmt.Println(url)

	for _, cmd := range commands {
		if _, err := exec.LookPath(cmd); err == nil {
			exec.Command(cmd, url).Start()
			return
		}
	}
}

// Send sends a basic notification
func (n *Notifier) Send(message string) error {
	return n.send("", message, "")
}

// SendSuccess sends a success notification
func (n *Notifier) SendSuccess(message string) error {
	return n.send("", "✓ "+message, "")
}

// SendSuccessWithAction sends a success notification with an action button
func (n *Notifier) SendSuccessWithAction(message, url string) error {
	return n.send("", "✓ "+message, url)
}

// SendError sends an error notification
func (n *Notifier) SendError(message string) error {
	return n.send("critical", "✗ "+message, "")
}
