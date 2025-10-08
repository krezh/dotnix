package manager

import (
	"time"

	"github.com/kopia/kopia/snapshot"
)

// KopiaManager handles Kopia operations using the official library
type KopiaManager struct {
	ConfigPath   string
	PasswordPath string
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
	Description string
	StartTime   time.Time
	EndTime     time.Time
	TotalSize   int64
	FileCount   int32
	DirCount    int32
}

// Constants
const (
	AppName    = "kopia-manager"
	AppVersion = "1.0.0"
)

// Convert Kopia snapshot.Manifest to our SnapshotSummary for display
func manifestToSummary(m *snapshot.Manifest) SnapshotSummary {
	return SnapshotSummary{
		ID:          string(m.ID),
		Source:      m.Source.Path,
		Description: m.Description,
		StartTime:   m.StartTime.ToTime(),
		EndTime:     m.EndTime.ToTime(),
		TotalSize:   m.Stats.TotalFileSize,
		FileCount:   m.Stats.TotalFileCount,
		DirCount:    m.Stats.TotalDirectoryCount,
	}
}
