package repo

import (
	"kopia-manager/internal/manager"

	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

// MaintenanceCmd runs repository maintenance with an optional --unsafe flag.
var MaintenanceCmd = &cobra.Command{
	Use:   "maintenance",
	Short: "Run repository maintenance",
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		unsafe, _ := cmd.Flags().GetBool("unsafe")

		log.Info("Starting maintenance", "unsafe", unsafe)

		if err := km.RunMaintenance(unsafe); err != nil {
			log.Fatal("Maintenance failed", "error", err)
		}

		log.Info("Maintenance completed successfully")
	},
}

// VerifyCmd verifies repository integrity
var VerifyCmd = &cobra.Command{
	Use:   "verify",
	Short: "Verify repository integrity",
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()

		log.Info("Starting verification")

		if err := km.VerifyRepository(); err != nil {
			log.Fatal("Verification failed", "error", err)
		}

		log.Info("Verification completed")
	},
}

func init() {
	// Wire flags locally to maintenance command
	if MaintenanceCmd.Flags().Lookup("unsafe") == nil {
		MaintenanceCmd.Flags().BoolP("unsafe", "u", false, "Run unsafe maintenance (skip safety checks)")
	}
}
