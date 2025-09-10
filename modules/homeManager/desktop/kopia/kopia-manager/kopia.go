package main

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
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

// KopiaManager handles Kopia operations
type KopiaManager struct {
	ConfigPath   string
	PasswordPath string
}

// NewKopiaManager creates a new KopiaManager instance
func NewKopiaManager() *KopiaManager {
	homeDir, _ := os.UserHomeDir()
	return &KopiaManager{
		ConfigPath:   filepath.Join(homeDir, ".config", "kopia", "repository.config"),
		PasswordPath: filepath.Join(homeDir, ".config", "kopia", "repository.password"),
	}
}

// setupEnv sets up the required environment variables for Kopia
func (km *KopiaManager) setupEnv() error {
	password, err := os.ReadFile(km.PasswordPath)
	if err != nil {
		return fmt.Errorf("failed to read password file: %w", err)
	}

	os.Setenv("KOPIA_CONFIG_PATH", km.ConfigPath)
	os.Setenv("KOPIA_PASSWORD", strings.TrimSpace(string(password)))
	return nil
}

// runKopiaCommand executes a Kopia command with proper environment setup
func (km *KopiaManager) runKopiaCommand(args ...string) ([]byte, error) {
	if err := km.setupEnv(); err != nil {
		return nil, err
	}

	cmd := exec.Command("kopia", args...)
	return cmd.Output()
}

// runKopiaCommandInteractive executes a Kopia command with live output
func (km *KopiaManager) runKopiaCommandInteractive(args ...string) error {
	if err := km.setupEnv(); err != nil {
		return err
	}

	cmd := exec.Command("kopia", args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	return cmd.Run()
}

// GetStatus returns the repository status
func (km *KopiaManager) GetStatus() (string, error) {
	output, err := km.runKopiaCommand("repository", "status")
	if err != nil {
		return "", err
	}
	return string(output), nil
}

// ListSnapshots returns all snapshots
func (km *KopiaManager) ListSnapshots() ([]KopiaSnapshot, error) {
	output, err := km.runKopiaCommand("snapshot", "list", "--json")
	if err != nil {
		return nil, err
	}

	var snapshots []KopiaSnapshot
	if err := json.Unmarshal(output, &snapshots); err != nil {
		return nil, err
	}
	return snapshots, nil
}

// GetSnapshotInfo returns detailed information about a specific snapshot
func (km *KopiaManager) GetSnapshotInfo(snapshotID string) (string, error) {
	output, err := km.runKopiaCommand("snapshot", "list", "--show-identical", snapshotID)
	if err != nil {
		return "", err
	}
	return string(output), nil
}

// CreateBackup creates a backup of the specified paths
func (km *KopiaManager) CreateBackup(paths []string, description string) error {
	args := append([]string{"snapshot", "create"}, paths...)
	if description != "" {
		args = append(args, "--description", description)
	}

	return km.runKopiaCommandInteractive(args...)
}

// RestoreSnapshot restores a snapshot to the target directory
func (km *KopiaManager) RestoreSnapshot(snapshotID, targetDir string) error {
	return km.runKopiaCommandInteractive("snapshot", "restore", snapshotID, targetDir)
}

// DeleteSnapshot deletes a specific snapshot
func (km *KopiaManager) DeleteSnapshot(snapshotID string) error {
	// If snapshotID is partial, find the full ID
	fullID, err := km.resolveSnapshotID(snapshotID)
	if err != nil {
		return fmt.Errorf("failed to resolve snapshot ID: %w", err)
	}

	return km.runKopiaCommandInteractive("snapshot", "delete", "--delete", fullID)
}

// resolveSnapshotID resolves a partial snapshot ID to a full ID
func (km *KopiaManager) resolveSnapshotID(partialID string) (string, error) {
	snapshots, err := km.ListSnapshots()
	if err != nil {
		return "", err
	}

	var matches []string
	for _, snap := range snapshots {
		if strings.HasPrefix(snap.ID, partialID) {
			matches = append(matches, snap.ID)
		}
	}

	if len(matches) == 0 {
		return "", fmt.Errorf("no snapshots found matching ID: %s", partialID)
	}

	if len(matches) > 1 {
		return "", fmt.Errorf("multiple snapshots match ID %s: %v", partialID, matches)
	}

	return matches[0], nil
}

// RunMaintenance runs repository maintenance
func (km *KopiaManager) RunMaintenance() error {
	return km.runKopiaCommandInteractive("maintenance", "run", "--full")
}

// VerifyRepository verifies repository integrity
func (km *KopiaManager) VerifyRepository() error {
	return km.runKopiaCommandInteractive("snapshot", "verify", "--verify-files-percent=10")
}

// RunGarbageCollection runs garbage collection
func (km *KopiaManager) RunGarbageCollection() error {
	return km.runKopiaCommandInteractive("maintenance", "run", "--full", "--safety=none")
}

// EstimateBackupSize estimates the size of a backup for given paths
func (km *KopiaManager) EstimateBackupSize(paths []string, uploadSpeed int) (string, error) {
	var results strings.Builder
	for _, path := range paths {
		uploadSpeedFlag := fmt.Sprintf("--upload-speed=%d", uploadSpeed)
		output, err := km.runKopiaCommand("snapshot", "estimate", uploadSpeedFlag, path)
		if err != nil {
			return "", err
		}
		results.WriteString(fmt.Sprintf("Estimate for %s:\n%s\n", path, string(output)))
	}
	return results.String(), nil
}

// ShowPolicy shows the backup policy for a path
func (km *KopiaManager) ShowPolicy(path string) (string, error) {
	output, err := km.runKopiaCommand("policy", "show", path)
	if err != nil {
		return "", err
	}
	return string(output), nil
}

// DiffSnapshots compares two snapshots
func (km *KopiaManager) DiffSnapshots(snapshot1, snapshot2 string) (string, error) {
	// Get snapshots to find their root object IDs
	snapshots, err := km.ListSnapshots()
	if err != nil {
		return "", fmt.Errorf("failed to list snapshots: %w", err)
	}

	var rootObj1, rootObj2 string
	for _, snap := range snapshots {
		if strings.HasPrefix(snap.ID, snapshot1) {
			rootObj1 = snap.RootEntry.Obj
		}
		if strings.HasPrefix(snap.ID, snapshot2) {
			rootObj2 = snap.RootEntry.Obj
		}
	}

	if rootObj1 == "" || rootObj2 == "" {
		return "", fmt.Errorf("could not find root objects for snapshots %s and %s", snapshot1, snapshot2)
	}

	output, err := km.runKopiaCommand("diff", rootObj1, rootObj2)
	if err != nil {
		return "", err
	}
	return string(output), nil
}

// MountSnapshot mounts a snapshot as a filesystem
func (km *KopiaManager) MountSnapshot(snapshotID, mountPoint string) error {
	// Create mount point if it doesn't exist
	if err := os.MkdirAll(mountPoint, 0755); err != nil {
		return fmt.Errorf("failed to create mount point: %w", err)
	}

	if err := km.setupEnv(); err != nil {
		return err
	}

	// Start mount in background
	cmd := exec.Command("kopia", "mount", snapshotID, mountPoint)
	return cmd.Start()
}

// UnmountSnapshot unmounts a previously mounted snapshot
func (km *KopiaManager) UnmountSnapshot(mountPoint string) error {
	// Try fusermount first (for FUSE mounts)
	cmd := exec.Command("fusermount", "-u", mountPoint)
	if err := cmd.Run(); err == nil {
		return nil
	}

	// Fall back to umount
	cmd = exec.Command("umount", mountPoint)
	return cmd.Run()
}
