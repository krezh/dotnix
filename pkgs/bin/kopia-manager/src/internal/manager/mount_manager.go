package manager

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"

	"kopia-manager/internal/ui"
)

// MountManager handles snapshot mount/unmount operations
type MountManager struct {
	km *KopiaManager
}

// NewMountManager creates a new MountManager
func NewMountManager(km *KopiaManager) *MountManager {
	return &MountManager{km: km}
}

// MountSnapshot mounts a snapshot using kopia CLI
func (mm *MountManager) MountSnapshot(snapshotID, mountPoint string) error {
	// Create mount point if it doesn't exist
	if err := os.MkdirAll(mountPoint, 0755); err != nil {
		return fmt.Errorf("failed to create mount point: %w", err)
	}

	// Build kopia mount command
	args := []string{"mount"}

	if mm.km.ConfigPath != "" {
		args = append(args, "--config-file", mm.km.ConfigPath)
	}

	args = append(args, snapshotID, mountPoint)

	// Set password via environment variable if provided
	var env []string
	env = append(env, os.Environ()...)
	env = append(env, "KOPIA_CHECK_FOR_UPDATES=false")

	if mm.km.PasswordPath != "" {
		// Read password from file and set as environment variable
		passwordBytes, err := os.ReadFile(mm.km.PasswordPath)
		if err == nil {
			password := strings.TrimSpace(string(passwordBytes))
			env = append(env, "KOPIA_PASSWORD="+password)
		}
	}

	ui.Infof("Mounting snapshot %s at %s...", snapshotID, mountPoint)
	ui.Infof("Command: kopia %s", strings.Join(args, " "))
	ui.Note("Note: This will start the mount process in the background.")
	ui.Help("Use 'km unmount' to unmount when done.")

	// Execute kopia mount command with proper process handling
	cmd := exec.Command("kopia", args...)
	cmd.Env = env

	// Discard stdout/stderr
	cmd.Stdout = nil
	cmd.Stderr = nil

	// Start the mount process in background
	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start mount: %w", err)
	}

	// Wait for the process in a goroutine to reap it when it exits
	go func() {
		cmd.Wait()
	}()

	// Wait for the mount to establish by watching for IN_MOUNT on the mount point.
	mountErr := mm.waitForMount(mountPoint, 30*time.Second)

	// Try to verify mount status, but don't fail if we can't verify
	if err := mountErr; err != nil {
		ui.Warningf("Warning: Could not verify mount status: %v", err)
		ui.Warningf("The mount process (PID %d) has been started.", cmd.Process.Pid)
		ui.Help("Check mount status manually with 'mount | grep kopia' or 'mountpoint <path>'")
	} else {
		ui.Successf("Successfully mounted snapshot %s at %s", snapshotID, mountPoint)
	}

	return nil
}

// UnmountSnapshot unmounts a mounted snapshot
func (mm *MountManager) UnmountSnapshot(mountPoint string) error {
	// Use fusermount to unmount FUSE filesystems
	cmd := exec.Command("fusermount", "-u", mountPoint)
	output, err := cmd.CombinedOutput()

	if err != nil {
		// Try umount as fallback
		cmd = exec.Command("umount", mountPoint)
		output, err = cmd.CombinedOutput()

		if err != nil {
			outputStr := strings.TrimSpace(string(output))
			if outputStr != "" {
				return fmt.Errorf("failed to unmount %s: %w\nOutput: %s", mountPoint, err, outputStr)
			}
			return fmt.Errorf("failed to unmount %s: %w", mountPoint, err)
		}
	}

	ui.Successf("Successfully unmounted %s", mountPoint)
	return nil
}

// waitForMount blocks until the mount point has a filesystem mounted on it or the timeout expires.
func (mm *MountManager) waitForMount(mountPoint string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		time.Sleep(500 * time.Millisecond)
		if mm.checkMountStatus(mountPoint) == nil {
			return nil
		}
	}
	return fmt.Errorf("mount point %s did not become active within %s", mountPoint, timeout)
}

// checkMountStatus verifies that a mount point is active
func (mm *MountManager) checkMountStatus(mountPoint string) error {
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
