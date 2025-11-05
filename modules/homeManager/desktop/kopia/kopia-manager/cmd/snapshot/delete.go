package snapshot

import (
	"fmt"
	"os"
	"strings"

	"kopia-manager/internal/manager"
	"kopia-manager/internal/ui"

	"github.com/charmbracelet/log"

	"github.com/spf13/cobra"
)

var deleteAll bool

// DeleteCmd deletes a specific snapshot or backup group with --all
var DeleteCmd = &cobra.Command{
	Use:   "delete [snapshot-id-or-backup-name]",
	Short: "Delete a specific snapshot or all snapshots from a backup group with --all",
	Long: `Delete snapshots from the repository.

Without --all flag: Deletes a specific snapshot by ID
With --all flag: Deletes all snapshots from a backup group

Examples:
  km delete 12e9406f405955816e93    # Delete specific snapshot
  km delete --all downloads          # Delete all snapshots from "downloads" backup group
  km delete --all                    # Delete ALL snapshots (requires confirmation)`,
	Args: cobra.MaximumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()

		if deleteAll {
			if len(args) == 0 {
				// Delete ALL snapshots
				if err := km.DeleteSnapshot("", true); err != nil {
					log.Fatal("Delete all failed", "error", err)
				}
			} else {
				// Delete all snapshots from a specific backup group
				backupName := args[0]
				if err := km.DeleteBackupGroup(backupName); err != nil {
					log.Fatal("Delete backup group failed", "error", err)
				}
			}
			return
		}

		if len(args) != 1 {
			fmt.Fprintln(os.Stderr, ui.ErrorStyle.Render("You must specify a snapshot ID."))
			os.Exit(2)
		}

		snapshotID := args[0]
		fmt.Print(ui.Promptf("Are you sure you want to delete snapshot %s? (y/N): ", snapshotID))
		var response string
		fmt.Scanln(&response)
		if strings.ToLower(response) != "y" {
			ui.Info("Operation cancelled.")
			return
		}

		if err := km.DeleteSnapshot(snapshotID, false); err != nil {
			log.Fatal("Delete failed", "error", err)
		}
	},
}

func init() {
	DeleteCmd.Flags().BoolVarP(&deleteAll, "all", "a", false, "Delete all snapshots")
	DeleteCmd.Flags().StringP("host", "H", "", "Filter snapshots by hostname")
	DeleteCmd.Flags().StringP("user", "U", "", "Filter snapshots by username")
}
