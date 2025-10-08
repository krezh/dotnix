package snapshot

import (
	"fmt"
	"time"

	"kopia-manager/internal/manager"

	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

// BackupCmd creates a backup of specified paths
var BackupCmd = &cobra.Command{
	Use:   "backup [paths...]",
	Short: "Create a backup of specified paths",
	Args:  cobra.MinimumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		description := fmt.Sprintf("Manual backup: %s", time.Now().Format("2006-01-02 15:04:05"))

		log.Info("Starting backup", "paths", args)

		if err := km.CreateBackup(args, description); err != nil {
			log.Fatal("Backup failed", "error", err)
		}

		log.Info("Backup completed successfully")
	},
}
