package snapshot

import (
	"fmt"

	"kopia-manager/internal/manager"

	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

// EstimateCmd estimates backup size for specified paths
var EstimateCmd = &cobra.Command{
	Use:   "estimate [paths...]",
	Short: "Estimate backup size for specified paths",
	Args:  cobra.MinimumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		uploadSpeed, _ := cmd.Flags().GetInt("upload-speed")

		estimate, err := km.EstimateBackupSize(args, uploadSpeed)
		if err != nil {
			log.Fatal("Failed to estimate backup size", "error", err)
		}

		fmt.Print(estimate)
	},
}

func init() {
	// Wire local flag for this command
	EstimateCmd.Flags().IntP("upload-speed", "s", 10, "Upload speed in MB/s for time estimation")
}
