package manager

import (
	"context"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"time"

	"kopia-manager/internal/ui"

	"github.com/kopia/kopia/fs/localfs"
	"github.com/kopia/kopia/repo"
	"github.com/kopia/kopia/repo/manifest"
	"github.com/kopia/kopia/snapshot"
	"github.com/kopia/kopia/snapshot/policy"
	"github.com/kopia/kopia/snapshot/restore"
	"github.com/kopia/kopia/snapshot/snapshotfs"
	"github.com/kopia/kopia/snapshot/upload"
)

// SnapshotManager handles snapshot-related operations
type SnapshotManager struct {
	km *KopiaManager
}

// NewSnapshotManager creates a new SnapshotManager
func NewSnapshotManager(km *KopiaManager) *SnapshotManager {
	return &SnapshotManager{km: km}
}

// ListSnapshots returns snapshots, optionally filtered by hostname and username
func (sm *SnapshotManager) ListSnapshots(hostname, username string) ([]SnapshotSummary, error) {
	ctx := context.Background()
	r, err := sm.km.openRepository(ctx)
	if err != nil {
		return nil, err
	}

	// Get all sources
	sources, err := snapshot.ListSources(ctx, r)
	if err != nil {
		return nil, fmt.Errorf("failed to list sources: %w", err)
	}

	// Filter sources by hostname and username
	filteredSources := sources
	if hostname != "" || username != "" {
		filteredSources = make([]snapshot.SourceInfo, 0, len(sources))
		for _, source := range sources {
			if hostname != "" && source.Host != hostname {
				continue
			}
			if username != "" && source.UserName != username {
				continue
			}
			filteredSources = append(filteredSources, source)
		}
	}

	var (
		mu           sync.Mutex
		wg           sync.WaitGroup
		allSnapshots = make([]SnapshotSummary, 0, len(filteredSources)*10)
	)

	for _, source := range filteredSources {
		wg.Add(1)
		go func(src snapshot.SourceInfo) {
			defer wg.Done()

			snapshots, err := snapshot.ListSnapshots(ctx, r, src)
			if err != nil {
				return
			}

			// Convert snapshots for this source
			localSnapshots := make([]SnapshotSummary, 0, len(snapshots))
			for _, snap := range snapshots {
				localSnapshots = append(localSnapshots, manifestToSummary(snap))
			}

			// Append to shared slice with mutex protection
			mu.Lock()
			allSnapshots = append(allSnapshots, localSnapshots...)
			mu.Unlock()
		}(source)
	}

	wg.Wait()

	return allSnapshots, nil
}

// GetSnapshotInfo returns detailed information about a specific snapshot
func (sm *SnapshotManager) GetSnapshotInfo(snapshotID string) (string, error) {
	ctx := context.Background()
	r, err := sm.km.openRepository(ctx)
	if err != nil {
		return "", err
	}
	defer r.Close(ctx)

	// Find the snapshot by ID (supports partial IDs)
	manifestID, err := sm.resolveSnapshotID(ctx, r, snapshotID)
	if err != nil {
		return "", err
	}

	snap, err := snapshot.LoadSnapshot(ctx, r, manifestID)
	if err != nil {
		return "", fmt.Errorf("failed to load snapshot: %w", err)
	}

	var result strings.Builder
	result.WriteString(fmt.Sprintf("Snapshot ID:         %s\n", snap.ID))
	result.WriteString(fmt.Sprintf("Description:         %s\n", snap.Description))
	result.WriteString(fmt.Sprintf("Source:              %s@%s:%s\n", snap.Source.UserName, snap.Source.Host, snap.Source.Path))
	result.WriteString(fmt.Sprintf("Start Time:          %s\n", snap.StartTime.ToTime().Format(time.RFC3339)))
	result.WriteString(fmt.Sprintf("End Time:            %s\n", snap.EndTime.ToTime().Format(time.RFC3339)))
	result.WriteString(fmt.Sprintf("Total Size:          %s\n", ui.FormatSize(snap.Stats.TotalFileSize)))
	result.WriteString(fmt.Sprintf("File Count:          %d\n", snap.Stats.TotalFileCount))
	result.WriteString(fmt.Sprintf("Directory Count:     %d\n", snap.Stats.TotalDirectoryCount))
	result.WriteString(fmt.Sprintf("Cached Files:        %d\n", snap.Stats.CachedFiles))
	result.WriteString(fmt.Sprintf("Non-Cached Files:    %d\n", snap.Stats.NonCachedFiles))
	result.WriteString(fmt.Sprintf("Error Count:         %d\n", snap.Stats.ErrorCount))

	if snap.IncompleteReason != "" {
		result.WriteString(fmt.Sprintf("Incomplete Reason:   %s\n", snap.IncompleteReason))
	}

	if len(snap.Tags) > 0 {
		result.WriteString("Tags:\n")
		for key, value := range snap.Tags {
			result.WriteString(fmt.Sprintf("  %s: %s\n", key, value))
		}
	}

	return result.String(), nil
}

// CreateBackup creates a backup of the specified paths
func (sm *SnapshotManager) CreateBackup(paths []string, description string) error {
	ctx := context.Background()
	r, err := sm.km.openRepository(ctx)
	if err != nil {
		return err
	}
	defer r.Close(ctx)

	return repo.WriteSession(ctx, r, repo.WriteSessionOptions{
		Purpose: "snapshot",
	}, func(ctx context.Context, w repo.RepositoryWriter) error {
		// Get client options for hostname and username
		clientOpts := r.ClientOptions()

		for i, path := range paths {
			// Create spinner for this backup
			spin := ui.NewSpinner(fmt.Sprintf("Creating backup [%d/%d]: %s", i+1, len(paths), ui.ShortenPath(path)))
			spin.Start()

			sourceInfo, err := snapshot.ParseSourceInfo(path, clientOpts.Hostname, clientOpts.Username)
			if err != nil {
				spin.Fail(fmt.Sprintf("Failed to parse source info: %v", err))
				return fmt.Errorf("failed to parse source info for %s: %w", path, err)
			}

			// Get or create policy for this path
			policyTree, err := policy.TreeForSource(ctx, w, sourceInfo)
			if err != nil {
				spin.Fail(fmt.Sprintf("Failed to get policy: %v", err))
				return fmt.Errorf("failed to get policy for %s: %w", path, err)
			}

			// Create local filesystem
			entry, err := localfs.NewEntry(path)
			if err != nil {
				spin.Fail(fmt.Sprintf("Failed to access path: %v", err))
				return fmt.Errorf("failed to create filesystem entry for %s: %w", path, err)
			}

			spin.UpdateMessage(fmt.Sprintf("Uploading backup [%d/%d]: %s", i+1, len(paths), ui.ShortenPath(path)))

			// Create uploader
			u := upload.NewUploader(w)
			u.Progress = &upload.CountingUploadProgress{}

			manifest, err := u.Upload(ctx, entry, policyTree, sourceInfo)
			if err != nil {
				spin.Fail(fmt.Sprintf("Failed to upload: %v", err))
				return fmt.Errorf("failed to upload %s: %w", path, err)
			}

			if description != "" {
				manifest.Description = description
			}

			_, err = snapshot.SaveSnapshot(ctx, w, manifest)
			if err != nil {
				spin.Fail(fmt.Sprintf("Failed to save snapshot: %v", err))
				return fmt.Errorf("failed to save snapshot: %w", err)
			}

			spin.Success(fmt.Sprintf("Snapshot created: %s", manifest.ID))
		}

		return nil
	})
}

// RestoreSnapshot restores a snapshot to the target directory
func (sm *SnapshotManager) RestoreSnapshot(snapshotID, targetDir string) error {
	ctx := context.Background()
	r, err := sm.km.openRepository(ctx)
	if err != nil {
		return err
	}
	defer r.Close(ctx)

	// Find the snapshot by ID
	manifestID, err := sm.resolveSnapshotID(ctx, r, snapshotID)
	if err != nil {
		return err
	}

	snap, err := snapshot.LoadSnapshot(ctx, r, manifestID)
	if err != nil {
		return fmt.Errorf("failed to load snapshot: %w", err)
	}

	// Create target directory if it doesn't exist
	if err := os.MkdirAll(targetDir, 0755); err != nil {
		return fmt.Errorf("failed to create target directory: %w", err)
	}

	// Create filesystem output - ensure full restore, not shallow
	output := &restore.FilesystemOutput{
		TargetPath:           targetDir,
		OverwriteDirectories: true,
		OverwriteFiles:       true,
		OverwriteSymlinks:    true,
		SkipOwners:           true,
		SkipPermissions:      true,
	}

	err = output.Init(ctx)
	if err != nil {
		return fmt.Errorf("failed to initialize output: %w", err)
	}

	// Create a filesystem entry from the snapshot
	rootEntry, err := snapshotfs.SnapshotRoot(r, snap)
	if err != nil {
		return fmt.Errorf("failed to create snapshot root entry: %w", err)
	}
	defer rootEntry.Close()

	// Create progress bar
	totalSize := snap.Stats.TotalFileSize
	progressBar := ui.NewSimpleProgressBar(fmt.Sprintf("Restoring snapshot %s", snapshotID), totalSize)

	// Restore with options for full deep restore (no placeholders)
	stats, err := restore.Entry(ctx, r, output, rootEntry, restore.Options{
		Parallel:               4,
		RestoreDirEntryAtDepth: math.MaxInt32, // Unlimited depth for full restore
		MinSizeForPlaceholder:  0,             // Default value - not used when depth is unlimited
		ProgressCallback: func(ctx context.Context, s restore.Stats) {
			progressBar.Update(s.RestoredTotalFileSize)
			progressBar.Print()
		},
	})
	if err != nil {
		return fmt.Errorf("failed to restore: %w", err)
	}

	// Finish the progress bar
	progressBar.Finish()

	ui.Summaryf("Restore completed: %d files, %d directories, %s",
		stats.RestoredFileCount, stats.RestoredDirCount, ui.FormatSize(stats.RestoredTotalFileSize))
	return nil
}

// DeleteSnapshot deletes a specific snapshot or all snapshots if allFlag is true.
//
// When allFlag is true, hostname and username optionally filter which snapshots are deleted.
func (sm *SnapshotManager) DeleteSnapshot(snapshotID string, allFlag bool, hostname, username string) error {
	ctx := context.Background()
	r, err := sm.km.openRepository(ctx)
	if err != nil {
		return err
	}
	defer r.Close(ctx)

	return repo.WriteSession(ctx, r, repo.WriteSessionOptions{
		Purpose: "delete-snapshot",
	}, func(ctx context.Context, w repo.RepositoryWriter) error {
		if allFlag {
			// Get all sources and snapshots
			sources, err := snapshot.ListSources(ctx, r)
			if err != nil {
				return fmt.Errorf("failed to list sources: %w", err)
			}

			// Filter sources by hostname and username
			if hostname != "" || username != "" {
				filtered := sources[:0]
				for _, source := range sources {
					if hostname != "" && source.Host != hostname {
						continue
					}
					if username != "" && source.UserName != username {
						continue
					}
					filtered = append(filtered, source)
				}
				sources = filtered
			}

			var allSnapshots []*snapshot.Manifest
			for _, source := range sources {
				snapshots, err := snapshot.ListSnapshots(ctx, r, source)
				if err != nil {
					continue
				}
				allSnapshots = append(allSnapshots, snapshots...)
			}

			if len(allSnapshots) == 0 {
				ui.Info("No snapshots to delete.")
				return nil
			}

			ui.Warning("The following snapshots will be deleted:")
			for _, snap := range allSnapshots {
				ui.Itemf("- %s (%s)", snap.ID, snap.Source.Path)
			}

			fmt.Print(ui.Prompt("Are you sure you want to delete ALL snapshots? Type 'yes' to confirm: "))
			var input string
			fmt.Scanln(&input)
			if input != "yes" {
				ui.Info("Aborted.")
				return nil
			}

			// Delete all snapshots
			for _, snap := range allSnapshots {
				err := w.DeleteManifest(ctx, snap.ID)
				if err != nil {
					ui.Errorf("Failed to delete snapshot %s: %v", snap.ID, err)
				} else {
					ui.Successf("Deleted snapshot %s", snap.ID)
				}
			}
			return nil
		}

		// Delete single snapshot
		manifestID, err := sm.resolveSnapshotID(ctx, r, snapshotID)
		if err != nil {
			return err
		}

		err = w.DeleteManifest(ctx, manifestID)
		if err != nil {
			return fmt.Errorf("failed to delete snapshot: %w", err)
		}

		ui.Successf("Deleted snapshot %s", manifestID)
		return nil
	})
}

// DeleteBackupGroup deletes all snapshots from a backup group
func (sm *SnapshotManager) DeleteBackupGroup(backupName string) error {
	ctx := context.Background()
	r, err := sm.km.openRepository(ctx)
	if err != nil {
		return err
	}
	defer r.Close(ctx)

	return repo.WriteSession(ctx, r, repo.WriteSessionOptions{
		Purpose: "delete-backup-group",
	}, func(ctx context.Context, w repo.RepositoryWriter) error {
		// Get all snapshots
		sources, err := snapshot.ListSources(ctx, r)
		if err != nil {
			return fmt.Errorf("failed to list sources: %w", err)
		}

		var matchingSnapshots []*snapshot.Manifest
		for _, source := range sources {
			snapshots, err := snapshot.ListSnapshots(ctx, r, source)
			if err != nil {
				continue
			}
			for _, snap := range snapshots {
				summary := manifestToSummary(snap)
				name := extractBackupName(summary)
				if name == backupName {
					matchingSnapshots = append(matchingSnapshots, snap)
				}
			}
		}

		if len(matchingSnapshots) == 0 {
			ui.Infof("No snapshots found for backup group '%s'.", backupName)
			return nil
		}

		ui.Warningf("The following %d snapshot(s) from backup group '%s' will be deleted:", len(matchingSnapshots), backupName)
		for _, snap := range matchingSnapshots {
			ui.Itemf("- %s (%s@%s:%s)", snap.ID, snap.Source.UserName, snap.Source.Host, snap.Source.Path)
		}

		fmt.Print(ui.Promptf("Are you sure you want to delete all snapshots from '%s'? Type 'yes' to confirm: ", backupName))
		var input string
		fmt.Scanln(&input)
		if input != "yes" {
			ui.Info("Aborted.")
			return nil
		}

		// Collect unique sources affected by the deletion
		affectedSources := make(map[string]snapshot.SourceInfo)
		for _, snap := range matchingSnapshots {
			sourceKey := fmt.Sprintf("%s@%s:%s", snap.Source.UserName, snap.Source.Host, snap.Source.Path)
			affectedSources[sourceKey] = snap.Source
		}

		// Delete matching snapshots
		for _, snap := range matchingSnapshots {
			err := w.DeleteManifest(ctx, snap.ID)
			if err != nil {
				ui.Errorf("Failed to delete snapshot %s: %v", snap.ID, err)
			} else {
				ui.Successf("Deleted snapshot %s", snap.ID)
			}
		}

		// Clean up orphaned source policies for sources with no remaining snapshots
		for _, source := range affectedSources {
			remaining, err := snapshot.ListSnapshots(ctx, r, source)
			if err != nil {
				ui.Errorf("Failed to check remaining snapshots for %s: %v", source.Path, err)
				continue
			}
			if len(remaining) > 0 {
				continue
			}

			_, err = policy.GetDefinedPolicy(ctx, r, source)
			if err == policy.ErrPolicyNotFound {
				ui.Infof("Source %s is now empty (no policy to remove).", source.Path)
				continue
			}
			if err != nil {
				ui.Errorf("Failed to check policy for %s: %v", source.Path, err)
				continue
			}

			if err := policy.RemovePolicy(ctx, w, source); err != nil {
				ui.Errorf("Failed to remove policy for %s: %v", source.Path, err)
			} else {
				ui.Successf("Removed policy for empty source %s", source.Path)
			}
		}

		return nil
	})
}

// extractBackupName extracts the backup name from a snapshot summary
func extractBackupName(snap SnapshotSummary) string {
	desc := snap.Description
	if strings.HasPrefix(desc, "Automated backup: ") {
		return strings.TrimPrefix(desc, "Automated backup: ")
	}
	if strings.HasPrefix(desc, "Manual backup: ") {
		return strings.TrimPrefix(desc, "Manual backup: ")
	}
	if desc != "" {
		return desc
	}
	return snap.Source
}

// DiffSnapshots compares two snapshots (simplified implementation)
func (sm *SnapshotManager) DiffSnapshots(snapshot1, snapshot2 string) (string, error) {
	ctx := context.Background()
	r, err := sm.km.openRepository(ctx)
	if err != nil {
		return "", err
	}
	defer r.Close(ctx)

	// Find both snapshots
	manifestID1, err := sm.resolveSnapshotID(ctx, r, snapshot1)
	if err != nil {
		return "", fmt.Errorf("failed to resolve snapshot %s: %w", snapshot1, err)
	}

	manifestID2, err := sm.resolveSnapshotID(ctx, r, snapshot2)
	if err != nil {
		return "", fmt.Errorf("failed to resolve snapshot %s: %w", snapshot2, err)
	}

	snap1, err := snapshot.LoadSnapshot(ctx, r, manifestID1)
	if err != nil {
		return "", fmt.Errorf("failed to load snapshot %s: %w", snapshot1, err)
	}

	snap2, err := snapshot.LoadSnapshot(ctx, r, manifestID2)
	if err != nil {
		return "", fmt.Errorf("failed to load snapshot %s: %w", snapshot2, err)
	}

	// Simple comparison of metadata
	var result strings.Builder
	result.WriteString(fmt.Sprintf("Comparing snapshots %s and %s:\n\n", snap1.ID, snap2.ID))
	result.WriteString(fmt.Sprintf("Snapshot 1: %s (%s)\n", snap1.ID, snap1.StartTime.ToTime().Format(time.RFC3339)))
	result.WriteString(fmt.Sprintf("  Size: %s, Files: %d, Dirs: %d\n",
		ui.FormatSize(snap1.Stats.TotalFileSize), snap1.Stats.TotalFileCount, snap1.Stats.TotalDirectoryCount))

	result.WriteString(fmt.Sprintf("Snapshot 2: %s (%s)\n", snap2.ID, snap2.StartTime.ToTime().Format(time.RFC3339)))
	result.WriteString(fmt.Sprintf("  Size: %s, Files: %d, Dirs: %d\n",
		ui.FormatSize(snap2.Stats.TotalFileSize), snap2.Stats.TotalFileCount, snap2.Stats.TotalDirectoryCount))

	// Calculate differences
	sizeDiff := snap2.Stats.TotalFileSize - snap1.Stats.TotalFileSize
	filesDiff := int64(snap2.Stats.TotalFileCount) - int64(snap1.Stats.TotalFileCount)
	dirsDiff := int64(snap2.Stats.TotalDirectoryCount) - int64(snap1.Stats.TotalDirectoryCount)

	result.WriteString(fmt.Sprintf("\nDifferences:\n"))
	result.WriteString(fmt.Sprintf("  Size: %+d bytes (%s)\n", sizeDiff, ui.FormatSize(sizeDiff)))
	result.WriteString(fmt.Sprintf("  Files: %+d\n", filesDiff))
	result.WriteString(fmt.Sprintf("  Directories: %+d\n", dirsDiff))

	return result.String(), nil
}

// RestoreBackupGroup restores all snapshots from a backup group to target directory
func (sm *SnapshotManager) RestoreBackupGroup(backupName, targetDir string) error {
	ctx := context.Background()
	r, err := sm.km.openRepository(ctx)
	if err != nil {
		return err
	}
	defer r.Close(ctx)

	// Get all sources
	sources, err := snapshot.ListSources(ctx, r)
	if err != nil {
		return fmt.Errorf("failed to list sources: %w", err)
	}

	var groupSnapshots []*snapshot.Manifest

	// Find all snapshots that match the backup name
	for _, source := range sources {
		snapshots, err := snapshot.ListSnapshots(ctx, r, source)
		if err != nil {
			continue
		}

		for _, snap := range snapshots {
			desc := snap.Description
			if strings.HasPrefix(desc, "Automated backup: ") {
				desc = strings.TrimPrefix(desc, "Automated backup: ")
			}
			if strings.HasPrefix(desc, "Manual backup: ") {
				desc = strings.TrimPrefix(desc, "Manual backup: ")
			}
			if desc == "" {
				desc = snap.Source.Path
			}

			if desc == backupName {
				groupSnapshots = append(groupSnapshots, snap)
			}
		}
	}

	if len(groupSnapshots) == 0 {
		return fmt.Errorf("no snapshots found for backup group: %s", backupName)
	}

	// Group by source path and get latest for each
	pathGroups := make(map[string][]*snapshot.Manifest)
	for _, snap := range groupSnapshots {
		pathGroups[snap.Source.Path] = append(pathGroups[snap.Source.Path], snap)
	}

	// Create target directory
	if err := os.MkdirAll(targetDir, 0755); err != nil {
		return fmt.Errorf("failed to create target directory: %w", err)
	}

	// Calculate total size for progress tracking
	totalSize := int64(0)
	var restoreList []struct {
		snapshot *snapshot.Manifest
		target   string
	}

	for sourcePath, pathSnaps := range pathGroups {
		// Sort by time and get the latest
		sort.Slice(pathSnaps, func(i, j int) bool {
			return pathSnaps[i].StartTime.ToTime().After(pathSnaps[j].StartTime.ToTime())
		})

		latest := pathSnaps[0]
		totalSize += latest.Stats.TotalFileSize

		baseName := filepath.Base(sourcePath)
		if baseName == "." || baseName == "/" {
			baseName = "root"
		}

		restoreTarget := filepath.Join(targetDir, baseName)
		restoreList = append(restoreList, struct {
			snapshot *snapshot.Manifest
			target   string
		}{latest, restoreTarget})
	}

	// Create overall progress bar for the entire backup group
	progressBar := ui.NewSimpleProgressBar(fmt.Sprintf("Restoring backup group '%s'", backupName), totalSize)
	restoredSize := int64(0)

	// Restore latest snapshot for each path
	for _, item := range restoreList {
		snap := item.snapshot
		restoreTarget := item.target

		// Create filesystem output
		output := &restore.FilesystemOutput{
			TargetPath:           restoreTarget,
			OverwriteDirectories: true,
			OverwriteFiles:       true,
			OverwriteSymlinks:    true,
			SkipOwners:           true,
			SkipPermissions:      true,
		}

		err := output.Init(ctx)
		if err != nil {
			return fmt.Errorf("failed to initialize output: %w", err)
		}

		// Create a filesystem entry from the snapshot
		rootEntry, err := snapshotfs.SnapshotRoot(r, snap)
		if err != nil {
			return fmt.Errorf("failed to create snapshot root entry: %w", err)
		}

		// Restore with progress tracking
		_, err = restore.Entry(ctx, r, output, rootEntry, restore.Options{
			Parallel:               4,
			RestoreDirEntryAtDepth: math.MaxInt32,
			MinSizeForPlaceholder:  0,
			ProgressCallback: func(ctx context.Context, s restore.Stats) {
				progressBar.Update(restoredSize + s.RestoredTotalFileSize)
				progressBar.Print()
			},
		})

		rootEntry.Close()

		if err != nil {
			return fmt.Errorf("failed to restore snapshot %s: %w", snap.ID, err)
		}

		// Update total restored size
		restoredSize += snap.Stats.TotalFileSize
		progressBar.Update(restoredSize)
		progressBar.Print()
	}

	// Finish the progress bar
	progressBar.Finish()

	ui.Successf("Successfully restored backup group '%s' to %s", backupName, targetDir)
	return nil
}

// resolveSnapshotID resolves a partial snapshot ID to a full manifest ID
func (sm *SnapshotManager) resolveSnapshotID(ctx context.Context, r repo.Repository, partialID string) (manifest.ID, error) {
	// Get all sources
	sources, err := snapshot.ListSources(ctx, r)
	if err != nil {
		return "", fmt.Errorf("failed to list sources: %w", err)
	}

	var matches []manifest.ID

	for _, source := range sources {
		snapshots, err := snapshot.ListSnapshots(ctx, r, source)
		if err != nil {
			continue
		}

		for _, snap := range snapshots {
			if strings.HasPrefix(string(snap.ID), partialID) {
				matches = append(matches, snap.ID)
			}
		}
	}

	if len(matches) == 0 {
		return "", fmt.Errorf("no snapshots found matching ID: %s", partialID)
	}

	if len(matches) > 1 {
		return "", fmt.Errorf("multiple snapshots match ID %s", partialID)
	}

	return matches[0], nil
}
