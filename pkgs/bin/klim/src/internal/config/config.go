package config

import (
	"time"

	"klim/pkg/types"
)

// DefaultConfig returns the default configuration.
func DefaultConfig() *types.Config {
	return &types.Config{
		HistoryDuration: 7 * 24 * time.Hour,
		MemoryBuffer:    0.5,
		MinMemory:       10.0,
		OutputFormat:    "table",
		Concurrency:     10,
	}
}

// Validate checks if the configuration is valid.
func Validate(cfg *types.Config) error {
	// Add validation logic as needed
	return nil
}
