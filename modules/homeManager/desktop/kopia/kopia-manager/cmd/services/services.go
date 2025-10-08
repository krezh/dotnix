package services

import (
	"fmt"
	"os/exec"
	"strings"
	"time"

	"kopia-manager/internal/manager"
	"kopia-manager/internal/ui"

	"github.com/charmbracelet/log"

	"github.com/spf13/cobra"
)

// servicesCmd shows status of systemd services and timers.
var ServicesCmd = &cobra.Command{
	Use:   "services",
	Short: "Show status of systemd services and timers",
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		status, err := km.GetServicesStatus()
		if err != nil {
			log.Fatal("Failed to get services status", "error", err)
		}
		fmt.Print(status)
	},
}

// startBackupCmd triggers systemd backup service for specified backup.
var StartBackupCmd = &cobra.Command{
	Use:   "start-backup <backup-name>",
	Short: "Trigger systemd backup service for specified backup",
	Long: `Trigger a systemd backup service to start immediately.

Examples:
  kopia-manager start-backup downloads
  kopia-manager start-backup wow

Use 'kopia-manager services' to see available backup services.`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		backupName := args[0]

		log.Info("Triggering backup service", "backup", backupName)

		if err := km.TriggerBackupService(backupName); err != nil {
			log.Fatal("Failed to start backup service", "error", err, "backup", backupName)
		}

		fmt.Printf("Started systemd service: kopia-backup-%s.service\n", backupName)
		fmt.Println("Use 'kopia-manager services' to check status")
		fmt.Println("Use 'kopia-manager logs' to view backup logs")

		log.Info("Backup service triggered successfully", "backup", backupName)
	},
}

// logsCmd shows recent backup logs.
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

// listServicesCmd lists available backup services and their timers.
var ListServicesCmd = &cobra.Command{
	Use:   "list-services",
	Short: "List available backup services and their timers",
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		services, err := km.ListBackupServices()
		if err != nil {
			log.Fatal("Failed to list backup services", "error", err)
		}

		if len(services) == 0 {
			fmt.Println("No backup services found.")
			return
		}

		table := ui.NewTableBuilder(" Available Backup Services ")
		table.AddColumn("Service Name", ui.Dynamic)
		table.AddColumn("Timer Status", ui.Dynamic)
		table.AddColumn("Last Run", ui.Dynamic)
		table.AddColumn("Next Run", ui.Dynamic)

		for _, service := range services {
			// Get last run time
			lastRun := getServiceLastRun(fmt.Sprintf("kopia-backup-%s.service", service))

			// Get timer status and next run
			timerName := fmt.Sprintf("kopia-backup-%s.timer", service)
			timerStatus := getServiceActiveStatus(timerName)
			nextRun := getTimerNextRun(timerName)

			// Note: Column ordering mirrors original behavior
			table.AddRow(service, lastRun, timerStatus, nextRun)
		}

		fmt.Print(table.Build())
	},
}

func init() {
	// Wire logs flags locally
	if LogsCmd.Flags().Lookup("lines") == nil {
		LogsCmd.Flags().IntP("lines", "n", 50, "Number of log lines to show")
	}
	if LogsCmd.Flags().Lookup("follow") == nil {
		LogsCmd.Flags().BoolP("follow", "f", false, "Follow log output (not supported in this implementation)")
	}

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

// getServiceActiveStatus returns a colored active/inactive string for a systemd unit.
func getServiceActiveStatus(serviceName string) string {
	cmd := exec.Command("systemctl", "--user", "is-active", serviceName)
	output, err := cmd.Output()
	if err != nil {
		return "inactive"
	}
	status := strings.TrimSpace(string(output))
	if status == "active" {
		return "\033[32mactive\033[0m" // Green
	} else if status == "inactive" {
		return "\033[90minactive\033[0m" // Gray
	}
	return status
}

// getServiceLastRun extracts last run time for a service and formats it relatively.
func getServiceLastRun(serviceName string) string {
	cmd := exec.Command("systemctl", "--user", "show", serviceName, "--property=ExecMainStartTimestamp")
	output, err := cmd.Output()
	if err != nil {
		return "Never"
	}

	line := strings.TrimSpace(string(output))
	if !strings.HasPrefix(line, "ExecMainStartTimestamp=") {
		return "Never"
	}

	timestamp := strings.TrimPrefix(line, "ExecMainStartTimestamp=")
	if timestamp == "" || timestamp == "0" {
		return "Never"
	}

	// Parse the timestamp format: "Mon 2006-01-02 15:04:05 MST"
	if t, err := time.Parse("Mon 2006-01-02 15:04:05 MST", timestamp); err == nil {
		now := time.Now()
		duration := now.Sub(t)

		// Format relative time
		if duration < time.Minute {
			return "Just now"
		} else if duration < time.Hour {
			return fmt.Sprintf("%.0fm ago", duration.Minutes())
		} else if duration < 24*time.Hour {
			return fmt.Sprintf("%.0fh ago", duration.Hours())
		} else {
			days := int(duration.Hours() / 24)
			return fmt.Sprintf("%dd ago", days)
		}
	}

	// Fallback: show just the date part
	if strings.Contains(timestamp, " ") {
		parts := strings.Fields(timestamp)
		if len(parts) >= 3 {
			return fmt.Sprintf("%s %s", parts[0], parts[2][:5])
		}
	}

	return "Unknown"
}

// getTimerNextRun parses the next run time for a timer and formats it succinctly.
func getTimerNextRun(timerName string) string {
	// Check if timer exists and is active first
	cmd := exec.Command("systemctl", "--user", "is-active", timerName)
	if err := cmd.Run(); err != nil {
		return "N/A"
	}

	// Get timer status with list-timers
	cmd = exec.Command("systemctl", "--user", "list-timers", timerName, "--no-legend")
	output, err := cmd.Output()
	if err != nil {
		return "N/A"
	}

	outputStr := strings.TrimSpace(string(output))
	if outputStr == "" {
		return "N/A"
	}

	// Parse systemd timer output with a heuristic to handle variable spacing
	// Format columns: NEXT LEFT LAST PASSED UNIT ACTIVATES
	fields := strings.Fields(outputStr)
	if len(fields) < 6 {
		return "N/A"
	}

	// Find where NEXT ends and LEFT begins by looking for timezone
	var nextParts []string
	var leftField string

	for i, field := range fields {
		if field == "CEST" || field == "CET" {
			// Everything up to and including timezone is NEXT
			nextParts = fields[0 : i+1]
			// The field after timezone is LEFT
			if i+1 < len(fields) {
				leftField = fields[i+1]
			}
			break
		}
	}

	// Format NEXT more readably: "Tue 2025-10-07 00:00:00 CEST" -> "Tue 00:00"
	var nextCol string
	if len(nextParts) >= 4 {
		dayOfWeek := nextParts[0]
		timePart := nextParts[2]
		if strings.Contains(timePart, ":") {
			timeShort := timePart[:5] // HH:MM
			nextCol = fmt.Sprintf("%s %s", dayOfWeek, timeShort)
		}
	} else {
		nextCol = strings.Join(nextParts, " ")
	}

	// Format the output combining LEFT if available
	if leftField != "" && leftField != "-" {
		return fmt.Sprintf("%s (in %s)", nextCol, leftField)
	}

	return nextCol
}
