package config

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// Config holds all configuration options for recshot
type Config struct {
	// Zipline settings
	ZiplineURL      string
	TokenFile       string
	UseOriginalName bool
	Zipline         bool

	// File settings
	SavePath string
	Mode     string

	// UI settings
	Status bool

	// Internal fields
	Dependencies []string
}

// New creates a new configuration with default values
func New() *Config {
	return &Config{
		ZiplineURL:      "",
		TokenFile:       filepath.Join(os.Getenv("HOME"), ".config/zipline/token"),
		SavePath:        "/tmp",
		UseOriginalName: true,
		Zipline:         false,
		Status:          false,
		Dependencies: []string{
			"hyprctl", "gulp", "wl-screenrec", "grim", "wl-copy", "notify-send", "hyprpicker",
		},
	}
}

// Validate checks if the configuration is valid
func (c *Config) Validate() error {
	if c.Mode == "" && !c.Status {
		return &ValidationError{Field: "mode", Message: "mode is required"}
	}

	if c.Mode != "" && !c.IsImageMode() && !c.IsVideoMode() {
		return &ValidationError{Field: "mode", Message: "invalid mode: " + c.Mode}
	}

	if c.Zipline {
		if c.ZiplineURL == "" {
			return &ValidationError{Field: "url", Message: "Zipline URL is required when --zipline is enabled. Use -u/--url to specify it"}
		}

		if _, err := os.Stat(c.TokenFile); os.IsNotExist(err) {
			return &ValidationError{Field: "token", Message: fmt.Sprintf("token file not found: %s. Use -t/--token to specify a different path", c.TokenFile)}
		}

		// Check if token file is readable and not empty using buffered I/O
		if err := c.validateTokenFile(); err != nil {
			return err
		}
	}

	return nil
}

// IsImageMode returns true if the mode is for taking screenshots
func (c *Config) IsImageMode() bool {
	return strings.HasPrefix(c.Mode, "image-")
}

// IsVideoMode returns true if the mode is for recording video
func (c *Config) IsVideoMode() bool {
	return strings.HasPrefix(c.Mode, "video-")
}

// GetFileExtension returns the appropriate file extension for the current mode
func (c *Config) GetFileExtension() string {
	if c.IsImageMode() {
		return ".png"
	}
	return ".mp4"
}

// ValidationError represents a configuration validation error
type ValidationError struct {
	Field   string
	Message string
}

// validateTokenFile validates the token file using buffered I/O
func (c *Config) validateTokenFile() error {
	file, err := os.Open(c.TokenFile)
	if err != nil {
		return &ValidationError{Field: "token", Message: fmt.Sprintf("cannot read token file: %s", err)}
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	if !scanner.Scan() {
		return &ValidationError{Field: "token", Message: "token file is empty: " + c.TokenFile}
	}

	token := strings.TrimSpace(scanner.Text())
	if len(token) == 0 {
		return &ValidationError{Field: "token", Message: "token file is empty: " + c.TokenFile}
	}

	if err := scanner.Err(); err != nil {
		return &ValidationError{Field: "token", Message: fmt.Sprintf("error reading token file: %s", err)}
	}

	return nil
}

func (e *ValidationError) Error() string {
	return e.Message
}
