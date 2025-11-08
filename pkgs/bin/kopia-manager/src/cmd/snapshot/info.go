package snapshot

import (
	"fmt"

	"kopia-manager/internal/manager"

	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

// InfoCmd shows detailed information about a snapshot
var InfoCmd = &cobra.Command{
	Use:   "info [snapshot-id]",
	Short: "Show detailed information about a snapshot",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		snapshotID := args[0]

		info, err := km.GetSnapshotInfo(snapshotID)
		if err != nil {
			log.Error("Failed to get snapshot info", "error", err)
			return
		}

		fmt.Print(info)
	},
}

func init() {
	InfoCmd.Flags().StringP("host", "H", "", "Filter snapshots by hostname")
	InfoCmd.Flags().StringP("user", "U", "", "Filter snapshots by username")
}
