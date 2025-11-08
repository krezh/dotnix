package manager

import (
	"context"
	"fmt"
	"strings"
	"time"

	"kopia-manager/internal/ui"

	"github.com/charmbracelet/log"
	"github.com/kopia/kopia/fs"
	"github.com/kopia/kopia/fs/localfs"
)

// BackupEstimator handles backup size estimation
type BackupEstimator struct {
	km *KopiaManager
}

// NewBackupEstimator creates a new BackupEstimator
func NewBackupEstimator(km *KopiaManager) *BackupEstimator {
	return &BackupEstimator{km: km}
}

// EstimateBackupSize estimates the size of a backup for given paths
func (be *BackupEstimator) EstimateBackupSize(paths []string, uploadSpeed int) (string, error) {
	ctx := context.Background()
	headers := []string{"Path", "Files", "Size", "Est. Upload", "Est. Time"}
	var rows [][]string

	for i, path := range paths {
		// Create spinner for this path
		spin := ui.NewSpinner(fmt.Sprintf("Calculating size for path [%d/%d]: %s", i+1, len(paths), ui.ShortenPath(path)))
		spin.Start()

		totalSize, totalFiles, err := be.calculatePathSize(ctx, path)

		if err != nil {
			spin.Fail(fmt.Sprintf("Failed to calculate size for %s", ui.ShortenPath(path)))
			log.Warn("Failed to fully calculate size for path", "path", path, "error", err)
			log.Info("Continuing with partial results")
			// Add a row with error indication
			rows = append(rows, []string{path, "Error", "N/A", "N/A", "N/A"})
			continue
		}

		spin.Success(fmt.Sprintf("Calculated: %s, %d files", ui.FormatSize(totalSize), totalFiles))

		// Estimate upload time based on upload speed (in MB/s)
		var estimatedTime time.Duration
		if uploadSpeed > 0 {
			estimatedSeconds := totalSize / (int64(uploadSpeed) * 1024 * 1024)
			estimatedTime = time.Duration(estimatedSeconds) * time.Second
		}

		rows = append(rows, []string{
			path,
			fmt.Sprintf("%d", totalFiles),
			ui.FormatSize(totalSize),
			ui.FormatSize(totalSize) + " (no dedup)",
			fmt.Sprintf("%v", estimatedTime),
		})
	}

	return ui.RenderTable("Backup Size Estimates", headers, rows), nil
}

// calculatePathSize recursively calculates the total size and file count for a path
func (be *BackupEstimator) calculatePathSize(ctx context.Context, path string) (int64, int64, error) {
	entry, err := localfs.NewEntry(path)
	if err != nil {
		return 0, 0, fmt.Errorf("failed to access path: %w", err)
	}

	var totalSize int64
	var totalFiles int64
	err = be.walkEntry(ctx, entry, &totalSize, &totalFiles)
	if err != nil {
		return 0, 0, fmt.Errorf("failed to walk path: %w", err)
	}

	return totalSize, totalFiles, nil
}

// walkEntry recursively walks filesystem entries and accumulates size and file count
func (be *BackupEstimator) walkEntry(ctx context.Context, entry fs.Entry, totalSize *int64, totalFiles *int64) error {
	if entry.IsDir() {
		dir, ok := entry.(fs.Directory)
		if !ok {
			return fmt.Errorf("directory entry does not implement Directory interface")
		}

		// Iterate through directory entries
		return fs.IterateEntries(ctx, dir, func(ctx context.Context, child fs.Entry) error {
			// Recursively process each child entry
			err := be.walkEntry(ctx, child, totalSize, totalFiles)
			if err != nil {
				// Check if this is a permission error or similar access issue
				if be.isAccessError(err) {
					// Log the skipped directory/file but continue
					log.Debug("Skipping inaccessible path", "name", child.Name(), "error", err)
					return nil
				}
				// For other errors, propagate them
				return fmt.Errorf("failed processing %s: %w", child.Name(), err)
			}
			return nil
		})
	} else {
		// Regular file
		*totalSize += entry.Size()
		*totalFiles++
	}

	return nil
}

// isAccessError checks if an error is related to file access permissions
func (be *BackupEstimator) isAccessError(err error) bool {
	if err == nil {
		return false
	}

	errStr := err.Error()
	// Check for common permission-related error messages
	return strings.Contains(errStr, "permission denied") ||
		strings.Contains(errStr, "access denied") ||
		strings.Contains(errStr, "operation not permitted") ||
		strings.Contains(errStr, "no such file or directory") ||
		strings.Contains(errStr, "file name too long") ||
		strings.Contains(errStr, "too many levels of symbolic links")
}
