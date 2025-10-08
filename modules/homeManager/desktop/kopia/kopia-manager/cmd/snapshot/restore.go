package snapshot

import (
	"kopia-manager/internal/manager"

	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

// RestoreCmd restores a specific snapshot or an entire backup group
var RestoreCmd = &cobra.Command{
	Use:   "restore [snapshot-id-or-backup-name] [target-directory]",
	Short: "Restore a snapshot or latest snapshots from a backup group to target directory",
	Long: `Restore a snapshot or backup group to a target directory.

Without --all flag: Restores a specific snapshot by ID
With --all flag: Restores all snapshots from a backup group

Examples:
  kopia-manager restore 12e9406f405955816e93 /restore/path    # Restore specific snapshot
  kopia-manager restore --all downloads /restore/path        # Restore backup group

Tab completion:
  - Without --all: Shows snapshot IDs
  - With --all: Shows backup group names`,
	Args: cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		identifier := args[0]
		targetDir := args[1]
		restoreAll, _ := cmd.Flags().GetBool("all")

		if restoreAll {
			// Restore all snapshots from a backup group
			log.Info("Starting restore of backup group", "group", identifier, "target", targetDir)

			if err := km.RestoreBackupGroup(identifier, targetDir); err != nil {
				log.Fatal("Restore failed", "error", err)
			}

			log.Info("Backup group restore completed successfully")
		} else {
			// Restore single snapshot
			log.Info("Starting restore", "snapshot", identifier, "target", targetDir)

			if err := km.RestoreSnapshot(identifier, targetDir); err != nil {
				log.Fatal("Restore failed", "error", err)
			}

			log.Info("Restore completed successfully")
		}
	},
}

func init() {
	// Minimal local flag wiring for this command
	RestoreCmd.Flags().BoolP("all", "a", false, "Restore all snapshots from backup group")
}
