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

// DeleteCmd deletes a specific snapshot or all snapshots with --all
var DeleteCmd = &cobra.Command{
	Use:   "delete [snapshot-id]",
	Short: "Delete a specific snapshot or all snapshots with --all",
	Args:  cobra.MaximumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()

		if deleteAll {
			if err := km.DeleteSnapshot("", true); err != nil {
				log.Fatal("Delete all failed", "error", err)
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
}
