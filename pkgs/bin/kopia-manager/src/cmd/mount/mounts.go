package mount

import (
	"fmt"
	"os/exec"
	"path/filepath"
	"strings"

	"kopia-manager/internal/manager"
	"kopia-manager/internal/ui"

	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

// MountCmd mounts a snapshot as a filesystem using FUSE.
var MountCmd = &cobra.Command{
	Use:   "mount [snapshot-id] [mount-point]",
	Short: "Mount a snapshot as a filesystem using FUSE",
	Long: `Mount a snapshot as a filesystem using FUSE.

The snapshot-id can be:
- A specific snapshot ID (e.g., 'k1a2b3c4d5e6f7g8h9i0')
- 'all' to mount all snapshots in a unified view

Examples:
  km mount k1a2b3c4d5e6f7g8h9i0 /mnt/backup
  km mount all /mnt/all-backups`,
	Args: cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		snapshotID := args[0]
		mountPoint := args[1]
		browse, _ := cmd.Flags().GetBool("browse")

		absMount, err := filepath.Abs(mountPoint)
		if err != nil {
			log.Fatal("Failed to resolve mount point path", "error", err)
		}

		if err := km.MountSnapshot(snapshotID, absMount); err != nil {
			log.Fatal("Mount failed", "error", err)
		}

		if browse {
			if err := openFileManager(absMount); err != nil {
				log.Warn("Failed to open file manager", "error", err)
			}
		}
	},
}

// UnmountCmd unmounts a previously mounted snapshot.
var UnmountCmd = &cobra.Command{
	Use:   "unmount [mount-point]",
	Short: "Unmount a previously mounted snapshot",
	Long: `Unmount a previously mounted snapshot filesystem.

Examples:
  km unmount /mnt/backup
  km unmount /mnt/all-backups`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		mountPoint := args[0]

		if err := km.UnmountSnapshot(mountPoint); err != nil {
			log.Fatal("Unmount failed", "error", err)
		}
	},
}

// ListMountsCmd lists active kopia FUSE mounts.
var ListMountsCmd = &cobra.Command{
	Use:   "list-mounts",
	Short: "List active kopia FUSE mounts",
	Run: func(cmd *cobra.Command, args []string) {
		mounts, err := listActiveMounts()
		if err != nil {
			log.Fatal("Failed to list mounts", "error", err)
		}

		if len(mounts) == 0 {
			ui.Info("No active kopia mounts found.")
			return
		}

		headers := []string{"Mount Point", "Source", "Type"}
		var rows [][]string

		for _, mount := range mounts {
			rows = append(rows, []string{mount.MountPoint, mount.Source, mount.Type})
		}

		fmt.Print(ui.RenderTable("Active Kopia Mounts", headers, rows))
	},
}

func init() {
	// Local flag wiring for mount command; guard to avoid double-registration.
	if MountCmd.Flags().Lookup("browse") == nil {
		MountCmd.Flags().BoolP("browse", "b", false, "Open file manager after mounting")
	}
}

// openFileManager attempts to open the file manager at the given path.
func openFileManager(path string) error {
	var cmd *exec.Cmd

	// Try different file managers based on availability
	if _, err := exec.LookPath("xdg-open"); err == nil {
		cmd = exec.Command("xdg-open", path)
	} else if _, err := exec.LookPath("nautilus"); err == nil {
		cmd = exec.Command("nautilus", path)
	} else if _, err := exec.LookPath("dolphin"); err == nil {
		cmd = exec.Command("dolphin", path)
	} else if _, err := exec.LookPath("thunar"); err == nil {
		cmd = exec.Command("thunar", path)
	} else {
		return fmt.Errorf("no supported file manager found")
	}

	return cmd.Start()
}

// MountInfo represents information about an active mount.
type MountInfo struct {
	MountPoint string
	Source     string
	Type       string
}

// listActiveMounts returns a list of active kopia-related mounts.
func listActiveMounts() ([]MountInfo, error) {
	cmd := exec.Command("mount")
	output, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("failed to run mount command: %w", err)
	}

	var mounts []MountInfo
	lines := strings.Split(string(output), "\n")

	for _, line := range lines {
		// Look for kopia FUSE mounts only
		if !strings.Contains(line, "kopia") {
			continue
		}

		fields := strings.Fields(line)
		if len(fields) >= 3 {
			// Parse mount line: source on mountpoint type options
			for i, field := range fields {
				if field == "on" && i+1 < len(fields) {
					mountPoint := fields[i+1]
					source := fields[0]
					mountType := "fuse"

					if i+3 < len(fields) && fields[i+2] == "type" {
						mountType = fields[i+3]
					}

					mounts = append(mounts, MountInfo{
						MountPoint: mountPoint,
						Source:     source,
						Type:       mountType,
					})
					break
				}
			}
		}
	}

	return mounts, nil
}
