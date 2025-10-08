package notify

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

// Notifier handles desktop notifications
type Notifier struct {
	iconPath   string
	execCache  map[string]bool
	cacheMutex sync.RWMutex
}

// New creates a new notifier
func New() *Notifier {
	return &Notifier{
		iconPath:  findIcon(),
		execCache: make(map[string]bool),
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

	// Check paths sequentially
	for _, path := range paths {
		if _, err := os.Stat(path); err == nil {
			return path
		}
	}

	return ""
}

// send sends a desktop notification with optional action button
func (n *Notifier) send(urgency, message, url string) error {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

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

	cmd := exec.CommandContext(ctx, "notify-send", args...)

	if url != "" {
		output, err := cmd.Output()
		if err == nil {
			response := strings.TrimSpace(string(output))
			if response == "open" {
				n.openURL(url)
			}
		}
		return err
	}

	return cmd.Run()
}

// openURL attempts to open a URL using available system commands
func (n *Notifier) openURL(url string) {
	commands := []string{"xdg-open", "open", "firefox", "chromium", "google-chrome"}

	// Check cache first to avoid repeated lookups
	for _, cmdName := range commands {
		if n.isCommandAvailable(cmdName) {
			ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
			defer cancel()

			cmd := exec.CommandContext(ctx, cmdName, url)
			// Detach from current process so browser can outlive the notification
			if err := cmd.Start(); err != nil {
				continue
			}

			// Don't wait for browser to finish, just start it

			fmt.Printf("✓ Opened URL with %s\n", cmdName)
			return
		}
	}
}

// isCommandAvailable checks if a command is available, with simple caching
func (n *Notifier) isCommandAvailable(cmdName string) bool {
	n.cacheMutex.RLock()
	if available, cached := n.execCache[cmdName]; cached {
		n.cacheMutex.RUnlock()
		return available
	}
	n.cacheMutex.RUnlock()

	// Check if command exists
	_, err := exec.LookPath(cmdName)
	available := err == nil

	// Cache the result
	n.cacheMutex.Lock()
	n.execCache[cmdName] = available
	n.cacheMutex.Unlock()

	return available
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
