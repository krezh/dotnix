package snapshot

import (
	"fmt"
	"os"
	"strings"
	"time"

	"kopia-manager/internal/manager"
	"kopia-manager/internal/ui"
	"kopia-manager/internal/util"

	"charm.land/log/v2"

	"github.com/spf13/cobra"
)

var deleteAll bool

// DeleteCmd deletes a specific snapshot or backup group with --all
var DeleteCmd = &cobra.Command{
	Use:   "delete [snapshot-id-or-backup-name]",
	Short: "Delete a specific snapshot or all snapshots from a backup group with --all",
	Long: `Delete snapshots from the repository.

Without --all flag: Deletes a specific snapshot by ID
With --all flag: Deletes all snapshots, optionally filtered by --host, --user,
  --before and --after

The --before and --after flags filter snapshots by their start time. They can
be combined with --host/--user to narrow the selection. Date-only values
(e.g. 2024-01-31) are treated as inclusive of that whole day.

Examples:
  km delete 12e9406f405955816e93                     # Delete specific snapshot
  km delete --all downloads                          # Delete all snapshots from "downloads" backup group
  km delete --all                                    # Delete ALL snapshots (requires confirmation)
  km delete --all --host default --user attic        # Delete all snapshots for a specific host/user
  km delete --all --before 2024-01-01                # Delete snapshots started before 2024-01-01
  km delete --all --after 2024-01-01 --before 2024-02-01  # Delete snapshots from Jan 2024`,
	Args: cobra.MaximumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()

		if deleteAll {
			hostname, _ := cmd.Flags().GetString("host")
			username, _ := cmd.Flags().GetString("user")

			beforeStr, _ := cmd.Flags().GetString("before")
			afterStr, _ := cmd.Flags().GetString("after")

			var before, after time.Time
			if beforeStr != "" {
				t, err := util.ParseDateTime(beforeStr)
				if err != nil {
					log.Fatal("Invalid --before value", "error", err)
				}
				before = t
			}
			if afterStr != "" {
				t, err := util.ParseDateTime(afterStr)
				if err != nil {
					log.Fatal("Invalid --after value", "error", err)
				}
				after = t
			}

			if len(args) == 0 {
				// Delete all snapshots, optionally filtered by host/user/time
				if err := km.DeleteSnapshot("", true, hostname, username, before, after); err != nil {
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

		if err := km.DeleteSnapshot(snapshotID, false, "", "", time.Time{}, time.Time{}); err != nil {
			log.Fatal("Delete failed", "error", err)
		}
	},
}

func init() {
	DeleteCmd.Flags().BoolVarP(&deleteAll, "all", "a", false, "Delete all snapshots")
	DeleteCmd.Flags().StringP("host", "H", "", "Filter snapshots by hostname")
	DeleteCmd.Flags().StringP("user", "U", "", "Filter snapshots by username")
	DeleteCmd.Flags().String("before", "", "Only delete snapshots started before this date/time (e.g. 2024-01-31 or 2024-01-31T23:59:59)")
	DeleteCmd.Flags().String("after", "", "Only delete snapshots started after this date/time (e.g. 2024-01-01)")
}
