package main

import (
	"fmt"
	"os"
	"os/exec"
)

// copyImageToClipboard copies an image file to the system clipboard.
func copyImageToClipboard(filename string) error {
	file, err := os.Open(filename)
	if err != nil {
		return fmt.Errorf("failed to open image: %w", err)
	}
	defer file.Close()

	cmd := exec.Command("wl-copy", "-t", "image/png")
	cmd.Stdin = file
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to copy to clipboard: %w", err)
	}

	return nil
}
