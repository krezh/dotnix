package services

import (
	"kopia-manager/internal/manager"
	"kopia-manager/internal/ui"

	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

// StartBackupCmd triggers systemd backup service for specified backup.
var StartBackupCmd = &cobra.Command{
	Use:   "start-backup <backup-name>",
	Short: "Trigger systemd backup service for specified backup",
	Long: `Trigger a systemd backup service to start immediately.

Examples:
  km start-backup downloads
  km start-backup wow

Use 'km services' to see available backup services.`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		backupName := args[0]

		if err := km.TriggerBackupService(backupName); err != nil {
			log.Fatalf("Failed to start backup '%s': %v", backupName, err)
		}

		ui.Successf("Started systemd service: kopia-backup-%s.service", backupName)
		ui.Help("Use 'km services' to check status")
		ui.Help("Use 'km logs' to view backup logs")
	},
}

func init() {
	// Provide completion for start-backup command
	if StartBackupCmd != nil && StartBackupCmd.ValidArgsFunction == nil {
		StartBackupCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
			if len(args) == 0 {
				km := manager.NewKopiaManager()
				services, err := km.ListBackupServices()
				if err != nil {
					return nil, cobra.ShellCompDirectiveNoFileComp
				}
				return services, cobra.ShellCompDirectiveNoFileComp
			}
			return nil, cobra.ShellCompDirectiveDefault
		}
	}
}
