package notify

import "os/exec"

// Notifier handles desktop notifications
type Notifier struct{}

// New creates a new notifier
func New() *Notifier {
	return &Notifier{}
}

// Send sends a desktop notification
func (n *Notifier) Send(message string) error {
	return exec.Command("notify-send", "-t", "5000", "Recshot", message).Run()
}

// SendSuccess sends a success notification
func (n *Notifier) SendSuccess(message string) error {
	return n.Send("✓ " + message)
}

// SendError sends an error notification
func (n *Notifier) SendError(message string) error {
	return exec.Command("notify-send", "-u", "critical", "-t", "5000", "Recshot", "✗ "+message).Run()
}
