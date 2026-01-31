package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// TransitionOptions holds all transition-related settings
type TransitionOptions struct {
	FPS  string
	Step string
	Type string
	Pos  string
}

// displayImage shows an image using swww with the specified transition options
func displayImage(imagePath string, options TransitionOptions) {
	fmt.Printf("Displaying: %s\n", filepath.Base(imagePath))

	args := []string{"img", imagePath}

	// Add transition options if specified
	if options.FPS != "" {
		args = append(args, "--transition-fps", options.FPS)
	}
	if options.Step != "" {
		args = append(args, "--transition-step", options.Step)
	}
	if options.Type != "" {
		args = append(args, "--transition-type", options.Type)
	}
	if options.Pos != "" {
		pos := options.Pos
		// Handle "mouse" position specially
		if strings.ToLower(pos) == "mouse" {
			if mousePos := getMousePosition(); mousePos != "" {
				pos = mousePos
			} else {
				// If mouse detection fails, skip the position argument
				pos = ""
			}
		}
		if pos != "" {
			args = append(args, "--transition-pos", pos)
		}
	}

	if err := exec.Command("swww", args...).Run(); err != nil {
		fmt.Printf("Warning: Failed to display %s: %v\n", filepath.Base(imagePath), err)
	}
}

// getEnvWithDefault returns environment variable value or default if not set
func getEnvWithDefault(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

// getTransitionValue returns flag value, then env variable, then default
func getTransitionValue(flagValue, envKey, defaultValue string) string {
	if flagValue != "" {
		return flagValue
	}
	return getEnvWithDefault(envKey, defaultValue)
}
