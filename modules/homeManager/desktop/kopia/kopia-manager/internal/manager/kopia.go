package manager

import (
	"context"
	"fmt"
	"math"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"kopia-manager/internal/ui"

	"github.com/charmbracelet/log"

	"github.com/kopia/kopia/fs"
	"github.com/kopia/kopia/fs/localfs"
	"github.com/kopia/kopia/repo"
	"github.com/kopia/kopia/repo/maintenance"
	"github.com/kopia/kopia/repo/manifest"
	"github.com/kopia/kopia/snapshot"
	"github.com/kopia/kopia/snapshot/policy"
	"github.com/kopia/kopia/snapshot/restore"
	"github.com/kopia/kopia/snapshot/snapshotfs"
	"github.com/kopia/kopia/snapshot/upload"
)

// formatSize formats bytes into human-readable size with appropriate units
func formatSize(bytes int64) string {
	if bytes == 0 {
		return "0 B"
	}

	const unit = 1024
	if bytes < unit {
		return fmt.Sprintf("%d B", bytes)
	}

	div, exp := int64(unit), 0
	for n := bytes / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}

	units := []string{"KB", "MB", "GB", "TB", "PB"}
	return fmt.Sprintf("%.2f %s", float64(bytes)/float64(div), units[exp])
}

// NewKopiaManager creates a new KopiaManager instance
func NewKopiaManager() *KopiaManager {
	homeDir, _ := os.UserHomeDir()
	return &KopiaManager{
		ConfigPath:   filepath.Join(homeDir, ".config", "kopia", "repository.config"),
		PasswordPath: filepath.Join(homeDir, ".config", "kopia", "repository.password"),
	}
}

// openRepository opens a connection to the repository
func (km *KopiaManager) openRepository(ctx context.Context) (repo.Repository, error) {
	password, err := os.ReadFile(km.PasswordPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read password file: %w", err)
	}

	r, err := repo.Open(ctx, km.ConfigPath, strings.TrimSpace(string(password)), &repo.Options{})
	if err != nil {
		return nil, fmt.Errorf("failed to open repository: %w", err)
	}

	return r, nil
}

// GetStatus returns the repository status
func (km *KopiaManager) GetStatus() (string, error) {
	ctx := context.Background()
	r, err := km.openRepository(ctx)
	if err != nil {
		return "", err
	}
	defer r.Close(ctx)

	clientOpts := r.ClientOptions()

	// Get repository statistics
	sources, err := snapshot.ListSources(ctx, r)
	if err != nil {
		return "", fmt.Errorf("failed to list sources: %w", err)
	}

	// Count total snapshots
	totalSnapshots := 0
	for _, source := range sources {
		snapshots, err := snapshot.ListSnapshots(ctx, r, source)
		if err != nil {
			continue
		}
		totalSnapshots += len(snapshots)
	}

	// Create table with repository status
	table := ui.NewTableBuilder(" Repository Status ")
	table.AddColumn("Property", ui.Dynamic)
	table.AddColumn("Value", ui.Dynamic)

	table.AddRow("Config file", km.ConfigPath)
	table.AddRow("Description", clientOpts.Description)
	table.AddRow("Hostname", clientOpts.Hostname)
	table.AddRow("Username", clientOpts.Username)
	table.AddRow("Read-only", fmt.Sprintf("%v", clientOpts.ReadOnly))
	table.AddRow("Sources", fmt.Sprintf("%d", len(sources)))
	table.AddRow("Snapshots", fmt.Sprintf("%d", totalSnapshots))

	return table.Build(), nil
}

// ListSnapshots returns all snapshots
func (km *KopiaManager) ListSnapshots() ([]SnapshotSummary, error) {
	ctx := context.Background()
	r, err := km.openRepository(ctx)
	if err != nil {
		return nil, err
	}
	defer r.Close(ctx)

	// Get all sources
	sources, err := snapshot.ListSources(ctx, r)
	if err != nil {
		return nil, fmt.Errorf("failed to list sources: %w", err)
	}

	var allSnapshots []SnapshotSummary

	for _, source := range sources {
		snapshots, err := snapshot.ListSnapshots(ctx, r, source)
		if err != nil {
			continue
		}

		for _, snap := range snapshots {
			allSnapshots = append(allSnapshots, manifestToSummary(snap))
		}
	}

	// Sort by start time (newest first)
	sort.Slice(allSnapshots, func(i, j int) bool {
		return allSnapshots[i].StartTime.After(allSnapshots[j].StartTime)
	})

	return allSnapshots, nil
}

// GetSnapshotInfo returns detailed information about a specific snapshot
func (km *KopiaManager) GetSnapshotInfo(snapshotID string) (string, error) {
	ctx := context.Background()
	r, err := km.openRepository(ctx)
	if err != nil {
		return "", err
	}
	defer r.Close(ctx)

	// Find the snapshot by ID (supports partial IDs)
	manifestID, err := km.resolveSnapshotID(ctx, r, snapshotID)
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
	result.WriteString(fmt.Sprintf("Total Size:          %s\n", formatSize(snap.Stats.TotalFileSize)))
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
func (km *KopiaManager) CreateBackup(paths []string, description string) error {
	ctx := context.Background()
	r, err := km.openRepository(ctx)
	if err != nil {
		return err
	}
	defer r.Close(ctx)

	return repo.WriteSession(ctx, r, repo.WriteSessionOptions{
		Purpose: "snapshot",
	}, func(ctx context.Context, w repo.RepositoryWriter) error {
		// Get client options for hostname and username
		clientOpts := r.ClientOptions()

		for _, path := range paths {
			sourceInfo, err := snapshot.ParseSourceInfo(path, clientOpts.Hostname, clientOpts.Username)
			if err != nil {
				return fmt.Errorf("failed to parse source info for %s: %w", path, err)
			}

			// Get or create policy for this path
			policyTree, err := policy.TreeForSource(ctx, w, sourceInfo)
			if err != nil {
				return fmt.Errorf("failed to get policy for %s: %w", path, err)
			}

			// Create local filesystem
			entry, err := localfs.NewEntry(path)
			if err != nil {
				return fmt.Errorf("failed to create filesystem entry for %s: %w", path, err)
			}

			// Create uploader
			u := upload.NewUploader(w)
			u.Progress = &upload.CountingUploadProgress{}

			manifest, err := u.Upload(ctx, entry, policyTree, sourceInfo)
			if err != nil {
				return fmt.Errorf("failed to upload %s: %w", path, err)
			}

			if description != "" {
				manifest.Description = description
			}

			_, err = snapshot.SaveSnapshot(ctx, w, manifest)
			if err != nil {
				return fmt.Errorf("failed to save snapshot: %w", err)
			}

			fmt.Printf("Snapshot created: %s\n", manifest.ID)
		}

		return nil
	})
}

// RestoreSnapshot restores a snapshot to the target directory
func (km *KopiaManager) RestoreSnapshot(snapshotID, targetDir string) error {
	ctx := context.Background()
	r, err := km.openRepository(ctx)
	if err != nil {
		return err
	}
	defer r.Close(ctx)

	// Find the snapshot by ID
	manifestID, err := km.resolveSnapshotID(ctx, r, snapshotID)
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

	// Restore with options for full deep restore (no placeholders)
	stats, err := restore.Entry(ctx, r, output, rootEntry, restore.Options{
		Parallel:               4,
		RestoreDirEntryAtDepth: math.MaxInt32, // Unlimited depth for full restore
		MinSizeForPlaceholder:  0,             // Default value - not used when depth is unlimited
		ProgressCallback: func(ctx context.Context, s restore.Stats) {
			fmt.Printf("Restored %d files, %s\n", s.RestoredFileCount, formatSize(s.RestoredTotalFileSize))
		},
	})
	if err != nil {
		return fmt.Errorf("failed to restore: %w", err)
	}

	fmt.Printf("Restore completed: %d files, %d directories, %s\n",
		stats.RestoredFileCount, stats.RestoredDirCount, formatSize(stats.RestoredTotalFileSize))
	return nil
}

// DeleteSnapshot deletes a specific snapshot or all snapshots if allFlag is true
func (km *KopiaManager) DeleteSnapshot(snapshotID string, allFlag bool) error {
	ctx := context.Background()
	r, err := km.openRepository(ctx)
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

			var allSnapshots []*snapshot.Manifest
			for _, source := range sources {
				snapshots, err := snapshot.ListSnapshots(ctx, r, source)
				if err != nil {
					continue
				}
				allSnapshots = append(allSnapshots, snapshots...)
			}

			if len(allSnapshots) == 0 {
				fmt.Println("No snapshots to delete.")
				return nil
			}

			fmt.Println("The following snapshots will be deleted:")
			for _, snap := range allSnapshots {
				fmt.Printf("- %s (%s)\n", snap.ID, snap.Source.Path)
			}

			fmt.Print("Are you sure you want to delete ALL snapshots? Type 'yes' to confirm: ")
			var input string
			fmt.Scanln(&input)
			if input != "yes" {
				fmt.Println("Aborted.")
				return nil
			}

			// Delete all snapshots
			for _, snap := range allSnapshots {
				err := w.DeleteManifest(ctx, snap.ID)
				if err != nil {
					fmt.Fprintf(os.Stderr, "Failed to delete snapshot %s: %v\n", snap.ID, err)
				} else {
					fmt.Printf("Deleted snapshot %s\n", snap.ID)
				}
			}
			return nil
		}

		// Delete single snapshot
		manifestID, err := km.resolveSnapshotID(ctx, r, snapshotID)
		if err != nil {
			return err
		}

		err = w.DeleteManifest(ctx, manifestID)
		if err != nil {
			return fmt.Errorf("failed to delete snapshot: %w", err)
		}

		fmt.Printf("Deleted snapshot %s\n", manifestID)
		return nil
	})
}

// resolveSnapshotID resolves a partial snapshot ID to a full manifest ID
func (km *KopiaManager) resolveSnapshotID(ctx context.Context, r repo.Repository, partialID string) (manifest.ID, error) {
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

// RunMaintenance runs repository maintenance
func (km *KopiaManager) RunMaintenance(unsafe bool) error {
	ctx := context.Background()
	r, err := km.openRepository(ctx)
	if err != nil {
		return err
	}
	defer r.Close(ctx)

	// Cast to DirectRepository to access maintenance functions
	dr, ok := r.(repo.DirectRepository)
	if !ok {
		return fmt.Errorf("repository does not support direct access")
	}

	return maintenance.RunExclusive(ctx, dr.(repo.DirectRepositoryWriter), maintenance.ModeFull, false, func(ctx context.Context, runParams maintenance.RunParameters) error {
		safety := maintenance.SafetyFull
		if unsafe {
			safety = maintenance.SafetyNone
		}

		return maintenance.Run(ctx, runParams, safety)
	})
}

// VerifyRepository verifies repository integrity
func (km *KopiaManager) VerifyRepository() error {
	ctx := context.Background()
	r, err := km.openRepository(ctx)
	if err != nil {
		return err
	}
	defer r.Close(ctx)

	// Get all sources and verify their snapshots
	sources, err := snapshot.ListSources(ctx, r)
	if err != nil {
		return fmt.Errorf("failed to list sources: %w", err)
	}

	var totalSnapshots, verifiedSnapshots int

	for _, source := range sources {
		snapshots, err := snapshot.ListSnapshots(ctx, r, source)
		if err != nil {
			fmt.Printf("Warning: failed to list snapshots for source %s: %v\n", source, err)
			continue
		}

		for _, snap := range snapshots {
			totalSnapshots++
			_, err := r.VerifyObject(ctx, snap.RootObjectID())
			if err != nil {
				fmt.Printf("Error verifying snapshot %s: %v\n", snap.ID, err)
			} else {
				verifiedSnapshots++
				fmt.Printf("Verified snapshot %s\n", snap.ID)
			}
		}
	}

	fmt.Printf("Verification completed: %d/%d snapshots verified successfully\n", verifiedSnapshots, totalSnapshots)
	return nil
}

// EstimateBackupSize estimates the size of a backup for given paths
func (km *KopiaManager) EstimateBackupSize(paths []string, uploadSpeed int) (string, error) {
	ctx := context.Background()
	table := ui.NewTableBuilder(" Backup Size Estimates ")
	table.AddColumn("Path", ui.Dynamic)
	table.AddColumn("Files", 8)
	table.AddColumn("Size", ui.Dynamic)
	table.AddColumn("Est. Upload", ui.Dynamic)
	table.AddColumn("Est. Time", ui.Dynamic)

	for _, path := range paths {
		log.Info("Calculating size for path", "path", path)
		totalSize, totalFiles, err := km.calculatePathSize(ctx, path)
		if err != nil {
			log.Warn("Failed to fully calculate size for path", "path", path, "error", err)
			log.Info("Continuing with partial results")
			// Add a row with error indication
			table.AddRow(
				path,
				"Error",
				"N/A",
				"N/A",
				"N/A",
			)
			continue
		}

		// Estimate upload time based on upload speed (in MB/s)
		var estimatedTime time.Duration
		if uploadSpeed > 0 {
			estimatedSeconds := totalSize / (int64(uploadSpeed) * 1024 * 1024)
			estimatedTime = time.Duration(estimatedSeconds) * time.Second
		}

		table.AddRow(
			path,
			fmt.Sprintf("%d", totalFiles),
			formatSize(totalSize),
			formatSize(totalSize)+" (no dedup)",
			fmt.Sprintf("%v", estimatedTime),
		)
	}

	return table.Build(), nil
}

// calculatePathSize recursively calculates the total size and file count for a path
func (km *KopiaManager) calculatePathSize(ctx context.Context, path string) (int64, int64, error) {
	entry, err := localfs.NewEntry(path)
	if err != nil {
		return 0, 0, fmt.Errorf("failed to access path: %w", err)
	}

	var totalSize int64
	var totalFiles int64
	err = km.walkEntry(ctx, entry, &totalSize, &totalFiles)
	if err != nil {
		return 0, 0, fmt.Errorf("failed to walk path: %w", err)
	}

	return totalSize, totalFiles, nil
}

// walkEntry recursively walks filesystem entries and accumulates size and file count
func (km *KopiaManager) walkEntry(ctx context.Context, entry fs.Entry, totalSize *int64, totalFiles *int64) error {
	if entry.IsDir() {
		dir, ok := entry.(fs.Directory)
		if !ok {
			return fmt.Errorf("directory entry does not implement Directory interface")
		}

		// Iterate through directory entries
		return fs.IterateEntries(ctx, dir, func(ctx context.Context, child fs.Entry) error {
			// Recursively process each child entry
			err := km.walkEntry(ctx, child, totalSize, totalFiles)
			if err != nil {
				// Check if this is a permission error or similar access issue
				if km.isAccessError(err) {
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
func (km *KopiaManager) isAccessError(err error) bool {
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

// ShowPolicy shows the backup policy for a path
func (km *KopiaManager) ShowPolicy(path string) (string, error) {
	ctx := context.Background()
	r, err := km.openRepository(ctx)
	if err != nil {
		return "", err
	}
	defer r.Close(ctx)

	// Get client options to build complete source info
	clientOpts := r.ClientOptions()
	sourceInfo, err := snapshot.ParseSourceInfo(path, clientOpts.Hostname, clientOpts.Username)
	if err != nil {
		return "", fmt.Errorf("failed to parse source info: %w", err)
	}

	// Get detailed policy information including sources
	effective, definition, _, err := policy.GetEffectivePolicy(ctx, r, sourceInfo)
	if err != nil {
		return "", fmt.Errorf("failed to get policy: %w", err)
	}

	// Check if there's a policy specifically defined for this path
	if err != nil && err != policy.ErrPolicyNotFound {
		return "", fmt.Errorf("failed to check for defined policy: %w", err)
	}

	// Helper function to check if a setting is defined for this target
	isDefinedForTarget := func(defSource snapshot.SourceInfo) bool {
		return defSource.Path == sourceInfo.Path && defSource.Host == sourceInfo.Host && defSource.UserName == sourceInfo.UserName
	}

	// Create table using TableBuilder
	title := fmt.Sprintf(" Policy for %s@%s:%s ", sourceInfo.UserName, sourceInfo.Host, sourceInfo.Path)
	table := ui.NewTableBuilder(title)
	table.AddColumn("Setting", 18)
	table.AddColumn("Value", 45)
	table.AddColumn("Source", 9)

	// Compression
	compSource := "Inherited"
	if definition != nil && definition.CompressionPolicy.CompressorName.Path != "" {
		if isDefinedForTarget(definition.CompressionPolicy.CompressorName) {
			compSource = "Defined"
		}
	}
	table.AddRow("Compression", string(effective.CompressionPolicy.CompressorName), compSource)

	// Retention policy
	annual := 0
	if effective.RetentionPolicy.KeepAnnual != nil {
		annual = effective.RetentionPolicy.KeepAnnual.OrDefault(0)
	}
	monthly := 0
	if effective.RetentionPolicy.KeepMonthly != nil {
		monthly = effective.RetentionPolicy.KeepMonthly.OrDefault(0)
	}
	weekly := 0
	if effective.RetentionPolicy.KeepWeekly != nil {
		weekly = effective.RetentionPolicy.KeepWeekly.OrDefault(0)
	}
	daily := 0
	if effective.RetentionPolicy.KeepDaily != nil {
		daily = effective.RetentionPolicy.KeepDaily.OrDefault(0)
	}
	hourly := 0
	if effective.RetentionPolicy.KeepHourly != nil {
		hourly = effective.RetentionPolicy.KeepHourly.OrDefault(0)
	}
	latest := 0
	if effective.RetentionPolicy.KeepLatest != nil {
		latest = effective.RetentionPolicy.KeepLatest.OrDefault(0)
	}

	retentionSource := "Inherited"
	if definition != nil && definition.RetentionPolicy.KeepDaily.Path != "" {
		if isDefinedForTarget(definition.RetentionPolicy.KeepDaily) {
			retentionSource = "Defined"
		}
	}
	retentionValue := fmt.Sprintf("%dy/%dm/%dw/%dd/%dh/%dl", annual, monthly, weekly, daily, hourly, latest)
	table.AddRow("Retention", retentionValue, retentionSource)

	// Files policy
	if len(effective.FilesPolicy.IgnoreRules) > 0 {
		ignoreSource := "Inherited"
		if definition != nil && definition.FilesPolicy.IgnoreRules.Path != "" {
			if isDefinedForTarget(definition.FilesPolicy.IgnoreRules) {
				ignoreSource = "Defined"
			}
		}
		// Show first rule, then add additional rows for remaining rules
		for i, rule := range effective.FilesPolicy.IgnoreRules {
			setting := "Ignore Rules"
			if i > 0 {
				setting = "" // Empty for continuation rows
			}
			source := ignoreSource
			if i > 0 {
				source = "" // Empty for continuation rows
			}
			table.AddRow(setting, rule, source)
		}
	}

	return table.Build(), nil
}

// DiffSnapshots compares two snapshots (simplified implementation)
func (km *KopiaManager) DiffSnapshots(snapshot1, snapshot2 string) (string, error) {
	ctx := context.Background()
	r, err := km.openRepository(ctx)
	if err != nil {
		return "", err
	}
	defer r.Close(ctx)

	// Find both snapshots
	manifestID1, err := km.resolveSnapshotID(ctx, r, snapshot1)
	if err != nil {
		return "", fmt.Errorf("failed to resolve snapshot %s: %w", snapshot1, err)
	}

	manifestID2, err := km.resolveSnapshotID(ctx, r, snapshot2)
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
		formatSize(snap1.Stats.TotalFileSize), snap1.Stats.TotalFileCount, snap1.Stats.TotalDirectoryCount))

	result.WriteString(fmt.Sprintf("Snapshot 2: %s (%s)\n", snap2.ID, snap2.StartTime.ToTime().Format(time.RFC3339)))
	result.WriteString(fmt.Sprintf("  Size: %s, Files: %d, Dirs: %d\n",
		formatSize(snap2.Stats.TotalFileSize), snap2.Stats.TotalFileCount, snap2.Stats.TotalDirectoryCount))

	// Calculate differences
	sizeDiff := snap2.Stats.TotalFileSize - snap1.Stats.TotalFileSize
	filesDiff := int64(snap2.Stats.TotalFileCount) - int64(snap1.Stats.TotalFileCount)
	dirsDiff := int64(snap2.Stats.TotalDirectoryCount) - int64(snap1.Stats.TotalDirectoryCount)

	result.WriteString(fmt.Sprintf("\nDifferences:\n"))
	result.WriteString(fmt.Sprintf("  Size: %+d bytes (%s)\n", sizeDiff, formatSize(sizeDiff)))
	result.WriteString(fmt.Sprintf("  Files: %+d\n", filesDiff))
	result.WriteString(fmt.Sprintf("  Directories: %+d\n", dirsDiff))

	return result.String(), nil
}

// GetServicesStatus returns the status of systemd kopia-related services
func (km *KopiaManager) GetServicesStatus() (string, error) {
	// List of kopia-related systemd services to check
	services := []string{
		"kopia-init.service",
		"kopia-maintenance.service",
	}

	var output strings.Builder
	output.WriteString("Systemd Services Status:\n")
	output.WriteString("========================\n\n")

	// Check for backup services
	backupServices, err := km.getBackupServices()
	if err == nil {
		services = append(services, backupServices...)
	}

	// Check each service
	for _, service := range services {
		status, err := km.getServiceStatus(service)
		if err != nil {
			output.WriteString(fmt.Sprintf("❌ %s: Failed to get status (%v)\n", service, err))
		} else {
			output.WriteString(status)
		}
		output.WriteString("\n")
	}

	// Check timers
	output.WriteString("\nSystemd Timers Status:\n")
	output.WriteString("======================\n\n")

	timers := []string{
		"kopia-maintenance.timer",
	}

	// Check for backup timers
	backupTimers, err := km.getBackupTimers()
	if err == nil {
		timers = append(timers, backupTimers...)
	}

	for _, timer := range timers {
		status, err := km.getTimerStatus(timer)
		if err != nil {
			output.WriteString(fmt.Sprintf("❌ %s: Failed to get status (%v)\n", timer, err))
		} else {
			output.WriteString(status)
		}
		output.WriteString("\n")
	}

	return output.String(), nil
}

// getServiceStatus gets the status of a specific systemd service
func (km *KopiaManager) getServiceStatus(serviceName string) (string, error) {
	cmd := exec.Command("systemctl", "--user", "status", serviceName)
	output, err := cmd.Output()

	if err != nil {
		// Service might not exist or be inactive, get basic info
		cmd = exec.Command("systemctl", "--user", "is-active", serviceName)
		activeOutput, _ := cmd.Output()
		activeState := strings.TrimSpace(string(activeOutput))

		cmd = exec.Command("systemctl", "--user", "is-enabled", serviceName)
		enabledOutput, _ := cmd.Output()
		enabledState := strings.TrimSpace(string(enabledOutput))

		statusIcon := "❌"
		if activeState == "active" {
			statusIcon = "✅"
		} else if activeState == "inactive" {
			statusIcon = "⏸️"
		}

		return fmt.Sprintf("%s %s: %s (enabled: %s)", statusIcon, serviceName, activeState, enabledState), nil
	}

	// Parse the systemctl status output for key information
	lines := strings.Split(string(output), "\n")
	var serviceLine, activeLine, memoryLine string

	for _, line := range lines {
		line = strings.TrimSpace(line)
		if strings.Contains(line, "●") && strings.Contains(line, serviceName) {
			serviceLine = line
		} else if strings.Contains(line, "Active:") {
			activeLine = line
		} else if strings.Contains(line, "Memory:") {
			memoryLine = line
		}
	}

	statusIcon := "❌"
	if strings.Contains(activeLine, "active (") {
		statusIcon = "✅"
	} else if strings.Contains(activeLine, "inactive") {
		statusIcon = "⏸️"
	}

	result := fmt.Sprintf("%s %s", statusIcon, serviceLine)
	if activeLine != "" {
		result += fmt.Sprintf("\n   %s", activeLine)
	}
	if memoryLine != "" {
		result += fmt.Sprintf("\n   %s", memoryLine)
	}

	return result, nil
}

// getTimerStatus gets the status of a specific systemd timer
func (km *KopiaManager) getTimerStatus(timerName string) (string, error) {
	cmd := exec.Command("systemctl", "--user", "status", timerName)
	output, err := cmd.Output()

	if err != nil {
		// Timer might not exist or be inactive
		cmd = exec.Command("systemctl", "--user", "is-active", timerName)
		activeOutput, _ := cmd.Output()
		activeState := strings.TrimSpace(string(activeOutput))

		statusIcon := "❌"
		if activeState == "active" {
			statusIcon = "✅"
		}

		return fmt.Sprintf("%s %s: %s", statusIcon, timerName, activeState), nil
	}

	// Parse the systemctl status output
	lines := strings.Split(string(output), "\n")
	var timerLine, activeLine, triggerLine string

	for _, line := range lines {
		line = strings.TrimSpace(line)
		if strings.Contains(line, "●") && strings.Contains(line, timerName) {
			timerLine = line
		} else if strings.Contains(line, "Active:") {
			activeLine = line
		} else if strings.Contains(line, "Trigger:") {
			triggerLine = line
		}
	}

	statusIcon := "❌"
	if strings.Contains(activeLine, "active (") {
		statusIcon = "✅"
	}

	result := fmt.Sprintf("%s %s", statusIcon, timerLine)
	if activeLine != "" {
		result += fmt.Sprintf("\n   %s", activeLine)
	}
	if triggerLine != "" {
		result += fmt.Sprintf("\n   %s", triggerLine)
	}

	return result, nil
}

// getBackupServices discovers backup services dynamically
func (km *KopiaManager) getBackupServices() ([]string, error) {
	cmd := exec.Command("systemctl", "--user", "list-units", "--type=service", "--all", "--no-legend")
	output, err := cmd.Output()
	if err != nil {
		return nil, err
	}

	var services []string
	lines := strings.Split(string(output), "\n")
	for _, line := range lines {
		if strings.Contains(line, "kopia-backup-") && strings.Contains(line, ".service") {
			fields := strings.Fields(line)
			if len(fields) > 0 {
				services = append(services, fields[0])
			}
		}
	}

	return services, nil
}

// getBackupTimers discovers backup timers dynamically
func (km *KopiaManager) getBackupTimers() ([]string, error) {
	cmd := exec.Command("systemctl", "--user", "list-units", "--type=timer", "--all", "--no-legend")
	output, err := cmd.Output()
	if err != nil {
		return nil, err
	}

	var timers []string
	lines := strings.Split(string(output), "\n")
	for _, line := range lines {
		if strings.Contains(line, "kopia-backup-") && strings.Contains(line, ".timer") {
			fields := strings.Fields(line)
			if len(fields) > 0 {
				timers = append(timers, fields[0])
			}
		}
	}

	return timers, nil
}

// GetServiceLogs returns recent logs for kopia-related services
func (km *KopiaManager) GetServiceLogs(lines int, follow bool) (string, error) {
	if lines <= 0 {
		lines = 50 // Default number of lines
	}

	var args []string
	args = append(args, "--user", "-n", fmt.Sprintf("%d", lines))

	if follow {
		args = append(args, "-f")
	}

	// Add kopia-related unit patterns
	args = append(args, "-u", "kopia-*")

	cmd := exec.Command("journalctl", args...)

	if follow {
		// For follow mode, we need to handle it differently
		return "", fmt.Errorf("follow mode not supported in this implementation - use 'journalctl --user -f -u kopia-*' directly")
	}

	output, err := cmd.Output()
	if err != nil {
		return "", fmt.Errorf("failed to get logs: %w", err)
	}

	return string(output), nil
}

// MountSnapshot mounts a snapshot using kopia CLI
func (km *KopiaManager) MountSnapshot(snapshotID, mountPoint string) error {
	// Create mount point if it doesn't exist
	if err := os.MkdirAll(mountPoint, 0755); err != nil {
		return fmt.Errorf("failed to create mount point: %w", err)
	}

	// Build kopia mount command
	args := []string{"mount"}

	if km.ConfigPath != "" {
		args = append(args, "--config-file", km.ConfigPath)
	}

	args = append(args, snapshotID, mountPoint)

	// Set password via environment variable if provided
	var env []string
	env = append(env, os.Environ()...)
	env = append(env, "KOPIA_CHECK_FOR_UPDATES=false")

	if km.PasswordPath != "" {
		// Read password from file and set as environment variable
		passwordBytes, err := os.ReadFile(km.PasswordPath)
		if err == nil {
			password := strings.TrimSpace(string(passwordBytes))
			env = append(env, "KOPIA_PASSWORD="+password)
		}
	}

	fmt.Printf("Mounting snapshot %s at %s...\n", snapshotID, mountPoint)
	fmt.Printf("Command: kopia %s\n", strings.Join(args, " "))
	fmt.Println("Note: This will start the mount process in the background.")
	fmt.Println("Use 'kopia-manager unmount' to unmount when done.")

	// Execute kopia mount command with proper process handling
	cmd := exec.Command("kopia", args...)
	cmd.Env = env

	// Redirect stdout/stderr to prevent hanging
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	// Start the mount process in background
	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start mount: %w", err)
	}

	// Wait a bit longer for mount to establish
	time.Sleep(5 * time.Second)

	// Try to verify mount status, but don't fail if we can't verify
	if err := km.checkMountStatus(mountPoint); err != nil {
		fmt.Printf("Warning: Could not verify mount status: %v\n", err)
		fmt.Printf("The mount process (PID %d) has been started.\n", cmd.Process.Pid)
		fmt.Println("Check mount status manually with 'mount | grep kopia' or 'mountpoint <path>'")
	} else {
		fmt.Printf("Successfully mounted snapshot %s at %s\n", snapshotID, mountPoint)
	}

	return nil
}

// UnmountSnapshot unmounts a mounted snapshot
func (km *KopiaManager) UnmountSnapshot(mountPoint string) error {
	// Use fusermount to unmount FUSE filesystems
	cmd := exec.Command("fusermount", "-u", mountPoint)
	output, err := cmd.CombinedOutput()

	if err != nil {
		// Try umount as fallback
		cmd = exec.Command("umount", mountPoint)
		output, err = cmd.CombinedOutput()

		if err != nil {
			return fmt.Errorf("failed to unmount %s: %w\nOutput: %s", mountPoint, err, string(output))
		}
	}

	fmt.Printf("Successfully unmounted %s\n", mountPoint)
	return nil
}

// checkMountStatus verifies that a mount point is active
func (km *KopiaManager) checkMountStatus(mountPoint string) error {
	// First try mountpoint command
	cmd := exec.Command("mountpoint", "-q", mountPoint)
	if err := cmd.Run(); err == nil {
		return nil // Mount is active
	}

	// Fallback: check if mount point has any files (indicating successful mount)
	entries, err := os.ReadDir(mountPoint)
	if err != nil {
		return fmt.Errorf("cannot read mount point %s: %w", mountPoint, err)
	}

	// If directory is not empty, it might be mounted
	if len(entries) > 0 {
		return nil
	}

	// Last resort: check mount command output
	cmd = exec.Command("mount")
	output, err := cmd.Output()
	if err != nil {
		return fmt.Errorf("mount point %s verification failed", mountPoint)
	}

	if strings.Contains(string(output), mountPoint) {
		return nil
	}

	return fmt.Errorf("mount point %s is not active", mountPoint)
}

// RestoreBackupGroup restores all snapshots from a backup group to target directory
func (km *KopiaManager) RestoreBackupGroup(backupName, targetDir string) error {
	ctx := context.Background()
	r, err := km.openRepository(ctx)
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

	// Restore latest snapshot for each path
	for sourcePath, pathSnaps := range pathGroups {
		// Sort by time and get the latest
		sort.Slice(pathSnaps, func(i, j int) bool {
			return pathSnaps[i].StartTime.ToTime().After(pathSnaps[j].StartTime.ToTime())
		})

		latest := pathSnaps[0]
		baseName := filepath.Base(sourcePath)
		if baseName == "." || baseName == "/" {
			baseName = "root"
		}

		restoreTarget := filepath.Join(targetDir, baseName)
		fmt.Printf("Restoring snapshot %s (%s) to %s...\n", latest.ID, sourcePath, restoreTarget)

		err := km.RestoreSnapshot(string(latest.ID), restoreTarget)
		if err != nil {
			return fmt.Errorf("failed to restore snapshot %s: %w", latest.ID, err)
		}
	}

	fmt.Printf("Successfully restored backup group '%s' to %s\n", backupName, targetDir)
	return nil
}

// TriggerBackupService starts a systemd backup service
func (km *KopiaManager) TriggerBackupService(backupName string) error {
	serviceName := fmt.Sprintf("kopia-backup-%s.service", backupName)

	// First check if service exists by getting available services
	availableServices, err := km.ListBackupServices()
	if err != nil {
		return fmt.Errorf("failed to check available services: %w", err)
	}

	// Check if the requested backup service exists
	serviceExists := false
	for _, service := range availableServices {
		if service == backupName {
			serviceExists = true
			break
		}
	}

	if !serviceExists {
		return fmt.Errorf("backup service '%s' not found. Use 'kopia-manager services' to see available backup services", backupName)
	}

	// Start the service
	cmd := exec.Command("systemctl", "--user", "start", serviceName)
	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("failed to start service %s: %w\nOutput: %s", serviceName, err, string(output))
	}

	return nil
}

// ListBackupServices returns a list of available backup service names
func (km *KopiaManager) ListBackupServices() ([]string, error) {
	cmd := exec.Command("systemctl", "--user", "list-units", "--type=service", "--all", "--no-legend")
	output, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("failed to list services: %w", err)
	}

	var services []string
	lines := strings.Split(string(output), "\n")
	for _, line := range lines {
		if strings.Contains(line, "kopia-backup-") && strings.Contains(line, ".service") {
			fields := strings.Fields(line)
			if len(fields) > 0 {
				serviceName := fields[0]
				// Extract backup name from service name
				if strings.HasPrefix(serviceName, "kopia-backup-") && strings.HasSuffix(serviceName, ".service") {
					backupName := strings.TrimPrefix(serviceName, "kopia-backup-")
					backupName = strings.TrimSuffix(backupName, ".service")
					services = append(services, backupName)
				}
			}
		}
	}

	return services, nil
}
