package snapshot

import (
	"fmt"

	"kopia-manager/internal/manager"

	"github.com/charmbracelet/log"

	"github.com/spf13/cobra"
)

// diffCmd compares two snapshots and prints the diff
var DiffCmd = &cobra.Command{
	Use:   "diff [snapshot-id-1] [snapshot-id-2]",
	Short: "Compare two snapshots",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		snapshot1 := args[0]
		snapshot2 := args[1]

		diff, err := km.DiffSnapshots(snapshot1, snapshot2)
		if err != nil {
			log.Error("Failed to diff snapshots", "error", err)
			return
		}

		fmt.Print(diff)
	},
}
