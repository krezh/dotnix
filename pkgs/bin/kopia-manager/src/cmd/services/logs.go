package services

import (
	"fmt"

	"kopia-manager/internal/manager"

	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

// LogsCmd shows recent backup logs.
var LogsCmd = &cobra.Command{
	Use:   "logs",
	Short: "Show recent backup logs",
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		lines, _ := cmd.Flags().GetInt("lines")
		follow, _ := cmd.Flags().GetBool("follow")

		logs, err := km.GetServiceLogs(lines, follow)
		if err != nil {
			log.Fatal("Failed to get logs", "error", err)
		}
		fmt.Print(logs)
	},
}

func init() {
	// Wire logs flags
	if LogsCmd.Flags().Lookup("lines") == nil {
		LogsCmd.Flags().IntP("lines", "n", 50, "Number of log lines to show")
	}
	if LogsCmd.Flags().Lookup("follow") == nil {
		LogsCmd.Flags().BoolP("follow", "f", false, "Follow log output (not supported in this implementation)")
	}
}
