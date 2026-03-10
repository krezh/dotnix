package repo

import (
	"fmt"

	"kopia-manager/internal/manager"

	"charm.land/log/v2"
	"github.com/spf13/cobra"
)

// StatusCmd shows repository status
var StatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show repository status",
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		status, err := km.GetStatus()
		if err != nil {
			log.Fatal("Failed to get status", "error", err)
		}
		fmt.Print(status)
	},
}
