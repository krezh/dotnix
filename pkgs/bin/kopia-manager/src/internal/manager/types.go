package manager

import (
	"context"
	"sync"
	"time"

	"github.com/kopia/kopia/repo"
	"github.com/kopia/kopia/snapshot"
)

// KopiaManager is the main entry point for Kopia operations
// It coordinates access to specialized managers for different operation types
type KopiaManager struct {
	ConfigPath   string
	PasswordPath string

	// Connection caching - shared across all managers
	mu         sync.Mutex
	cachedRepo repo.Repository
	cachedCtx  context.Context
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

// BackupEstimate represents backup size estimation
type BackupEstimate struct {
	Path                string
	TotalFiles          int64
	TotalSize           int64
	EstimatedUploadSize int64
	EstimatedTime       time.Duration
}

// SnapshotSummary provides a simplified view of snapshot for CLI display
type SnapshotSummary struct {
	ID          string
	Source      string
	Hostname    string
	Username    string
	Description string
	StartTime   time.Time
	EndTime     time.Time
	TotalSize   int64
	FileCount   int32
	DirCount    int32
}

// Constants
const (
	AppName    = "km"
	AppVersion = "1.0.0"
)

// Convert Kopia snapshot.Manifest to our SnapshotSummary for display
func manifestToSummary(m *snapshot.Manifest) SnapshotSummary {
	return SnapshotSummary{
		ID:          string(m.ID),
		Source:      m.Source.Path,
		Hostname:    m.Source.Host,
		Username:    m.Source.UserName,
		Description: m.Description,
		StartTime:   m.StartTime.ToTime(),
		EndTime:     m.EndTime.ToTime(),
		TotalSize:   m.Stats.TotalFileSize,
		FileCount:   m.Stats.TotalFileCount,
		DirCount:    m.Stats.TotalDirectoryCount,
	}
}
