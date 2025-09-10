package config

import (
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
		ZiplineURL:      "https://zipline.talos.plexuz.xyz",
		TokenFile:       filepath.Join(os.Getenv("HOME"), ".config/flameshot/zipline-token"),
		SavePath:        "/tmp",
		UseOriginalName: true,
		Zipline:         false,
		Status:          false,
		Dependencies: []string{
			"hyprctl", "slurp", "wl-screenrec", "grim", "wl-copy", "notify-send", "hyprpicker",
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
		if _, err := os.Stat(c.TokenFile); os.IsNotExist(err) {
			return &ValidationError{Field: "token", Message: "token file not found: " + c.TokenFile}
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

func (e *ValidationError) Error() string {
	return e.Message
}
