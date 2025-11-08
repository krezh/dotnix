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
  km restore 12e9406f405955816e93 /restore/path    # Restore specific snapshot
  km restore --all downloads /restore/path        # Restore backup group

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
			if err := km.RestoreBackupGroup(identifier, targetDir); err != nil {
				log.Fatal("Restore failed", "error", err)
			}
		} else {
			// Restore single snapshot
			if err := km.RestoreSnapshot(identifier, targetDir); err != nil {
				log.Fatal("Restore failed", "error", err)
			}
		}
	},
}

func init() {
	RestoreCmd.Flags().BoolP("all", "a", false, "Restore all snapshots from backup group")
	RestoreCmd.Flags().StringP("host", "H", "", "Filter snapshots by hostname")
	RestoreCmd.Flags().StringP("user", "U", "", "Filter snapshots by username")
}
