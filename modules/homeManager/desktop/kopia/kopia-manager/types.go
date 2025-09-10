package main

import (
	"time"
)

// KopiaSnapshot represents a Kopia snapshot
type KopiaSnapshot struct {
	ID          string    `json:"id"`
	Source      Source    `json:"source"`
	StartTime   time.Time `json:"startTime"`
	EndTime     time.Time `json:"endTime"`
	Stats       Stats     `json:"stats"`
	Description string    `json:"description"`
	RootEntry   RootEntry `json:"rootEntry"`
}

// Source represents the source of a snapshot
type Source struct {
	Host string `json:"host"`
	User string `json:"userName"`
	Path string `json:"path"`
}

// Stats contains statistics about a snapshot
type Stats struct {
	TotalSize      int64 `json:"totalSize"`
	FileCount      int   `json:"fileCount"`
	DirCount       int   `json:"dirCount"`
	CachedFiles    int   `json:"cachedFiles"`
	NonCachedFiles int   `json:"nonCachedFiles"`
	ErrorCount     int   `json:"errorCount"`
}

// RootEntry represents the root entry of a snapshot
type RootEntry struct {
	Obj string `json:"obj"`
}

// ServiceStatus represents the status of a systemd service
type ServiceStatus struct {
	Unit        string
	LoadState   string
	ActiveState string
	SubState    string
	Description string
}

// TimerStatus represents the status of a systemd timer
type TimerStatus struct {
	Next      string
	Left      string
	Last      string
	Passed    string
	Unit      string
	Activates string
}

// Constants
const (
	AppName    = "kopia-manager"
	AppVersion = "1.0.0"
)
