package manager

import (
	"fmt"
	"os/exec"
	"strings"
)

// ServiceManager handles systemd service operations
type ServiceManager struct {
	// No KopiaManager needed - uses systemd directly
}

// NewServiceManager creates a new ServiceManager
func NewServiceManager() *ServiceManager {
	return &ServiceManager{}
}

// GetServicesStatus returns the status of systemd kopia-related services
func (sm *ServiceManager) GetServicesStatus() (string, error) {
	// List of kopia-related systemd services to check
	services := []string{
		"kopia-init.service",
		"kopia-maintenance.service",
	}

	var output strings.Builder
	output.WriteString("Systemd Services Status:\n")
	output.WriteString("========================\n\n")

	// Check for backup services
	backupServices, err := sm.getBackupServices()
	if err == nil {
		services = append(services, backupServices...)
	}

	// Check each service
	for _, service := range services {
		status, err := sm.getServiceStatus(service)
		if err != nil {
			output.WriteString(fmt.Sprintf("❌ %s: Failed to get status (%v)\n", service, err))
		} else {
			output.WriteString(status)
		}
		output.WriteString("\n")
	}

	// Check timers
	output.WriteString("\nSystemd Timers Status:\n")
	output.WriteString("======================\n\n")

	timers := []string{
		"kopia-maintenance.timer",
	}

	// Check for backup timers
	backupTimers, err := sm.getBackupTimers()
	if err == nil {
		timers = append(timers, backupTimers...)
	}

	for _, timer := range timers {
		status, err := sm.getTimerStatus(timer)
		if err != nil {
			output.WriteString(fmt.Sprintf("❌ %s: Failed to get status (%v)\n", timer, err))
		} else {
			output.WriteString(status)
		}
		output.WriteString("\n")
	}

	return output.String(), nil
}

// GetServiceLogs returns recent logs for kopia-related services
func (sm *ServiceManager) GetServiceLogs(lines int, follow bool) (string, error) {
	if lines <= 0 {
		lines = 50 // Default number of lines
	}

	var args []string
	args = append(args, "--user", "-n", fmt.Sprintf("%d", lines))

	if follow {
		args = append(args, "-f")
	}

	// Add kopia-related unit patterns
	args = append(args, "-u", "kopia-*")

	cmd := exec.Command("journalctl", args...)

	if follow {
		// For follow mode, we need to handle it differently
		return "", fmt.Errorf("follow mode not supported in this implementation - use 'journalctl --user -f -u kopia-*' directly")
	}

	output, err := cmd.Output()
	if err != nil {
		return "", fmt.Errorf("failed to get logs: %w", err)
	}

	return string(output), nil
}

// TriggerBackupService starts a systemd backup service
func (sm *ServiceManager) TriggerBackupService(backupName string) error {
	serviceName := fmt.Sprintf("kopia-backup-%s.service", backupName)

	// First check if service exists by getting available services
	availableServices, err := sm.ListBackupServices()
	if err != nil {
		return fmt.Errorf("failed to check available services: %w", err)
	}

	// Check if the requested backup service exists
	serviceExists := false
	for _, service := range availableServices {
		if service == backupName {
			serviceExists = true
			break
		}
	}

	if !serviceExists {
		return fmt.Errorf("backup service '%s' not found. Use 'km services' to see available backup services", backupName)
	}

	// Start the service
	cmd := exec.Command("systemctl", "--user", "start", serviceName)
	output, err := cmd.CombinedOutput()
	if err != nil {
		outputStr := strings.TrimSpace(string(output))
		if outputStr != "" {
			return fmt.Errorf("failed to start service %s: %w\nOutput: %s", serviceName, err, outputStr)
		}
		return fmt.Errorf("failed to start service %s: %w", serviceName, err)
	}

	return nil
}

// ListBackupServices returns a list of available backup service names
func (sm *ServiceManager) ListBackupServices() ([]string, error) {
	cmd := exec.Command("systemctl", "--user", "list-units", "--type=service", "--all", "--no-legend")
	output, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("failed to list services: %w", err)
	}

	var services []string
	lines := strings.Split(string(output), "\n")
	for _, line := range lines {
		if strings.Contains(line, "kopia-backup-") && strings.Contains(line, ".service") {
			fields := strings.Fields(line)
			for _, field := range fields {
				if strings.HasPrefix(field, "kopia-backup-") && strings.HasSuffix(field, ".service") {
					backupName := strings.TrimPrefix(field, "kopia-backup-")
					backupName = strings.TrimSuffix(backupName, ".service")
					services = append(services, backupName)
					break
				}
			}
		}
	}

	return services, nil
}

// getServiceStatus gets the status of a specific systemd service
func (sm *ServiceManager) getServiceStatus(serviceName string) (string, error) {
	cmd := exec.Command("systemctl", "--user", "status", serviceName)
	output, _ := cmd.CombinedOutput()

	// Parse the systemctl status output for key information
	lines := strings.Split(string(output), "\n")
	var serviceLine, activeLine, memoryLine string

	for _, line := range lines {
		line = strings.TrimSpace(line)
		if strings.Contains(line, "●") && strings.Contains(line, serviceName) {
			serviceLine = line
		} else if strings.Contains(line, "Active:") {
			activeLine = line
		} else if strings.Contains(line, "Memory:") || strings.Contains(line, "Mem peak:") {
			memoryLine = line
		}
	}

	// If we couldn't parse the output, get basic info
	if serviceLine == "" {
		cmd = exec.Command("systemctl", "--user", "is-active", serviceName)
		activeOutput, _ := cmd.Output()
		activeState := strings.TrimSpace(string(activeOutput))

		statusIcon := "❌"
		if activeState == "active" {
			statusIcon = "✅"
		} else if activeState == "inactive" {
			statusIcon = "⏸️"
		} else if activeState == "failed" {
			statusIcon = "❌"
		}

		return fmt.Sprintf("%s %s: %s", statusIcon, serviceName, activeState), nil
	}

	statusIcon := "❌"
	if strings.Contains(activeLine, "active (") {
		statusIcon = "✅"
	} else if strings.Contains(activeLine, "inactive") {
		statusIcon = "⏸️"
	}

	result := fmt.Sprintf("%s %s", statusIcon, serviceLine)
	if activeLine != "" {
		result += fmt.Sprintf("\n   %s", activeLine)
	}
	if memoryLine != "" {
		result += fmt.Sprintf("\n   %s", memoryLine)
	}

	return result, nil
}

// getTimerStatus gets the status of a specific systemd timer
func (sm *ServiceManager) getTimerStatus(timerName string) (string, error) {
	cmd := exec.Command("systemctl", "--user", "status", timerName)
	output, err := cmd.Output()

	if err != nil {
		// Timer might not exist or be inactive
		cmd = exec.Command("systemctl", "--user", "is-active", timerName)
		activeOutput, _ := cmd.Output()
		activeState := strings.TrimSpace(string(activeOutput))

		statusIcon := "❌"
		if activeState == "active" {
			statusIcon = "✅"
		}

		return fmt.Sprintf("%s %s: %s", statusIcon, timerName, activeState), nil
	}

	// Parse the systemctl status output
	lines := strings.Split(string(output), "\n")
	var timerLine, activeLine, triggerLine string

	for _, line := range lines {
		line = strings.TrimSpace(line)
		if strings.Contains(line, "●") && strings.Contains(line, timerName) {
			timerLine = line
		} else if strings.Contains(line, "Active:") {
			activeLine = line
		} else if strings.Contains(line, "Trigger:") {
			triggerLine = line
		}
	}

	statusIcon := "❌"
	if strings.Contains(activeLine, "active (") {
		statusIcon = "✅"
	}

	result := fmt.Sprintf("%s %s", statusIcon, timerLine)
	if activeLine != "" {
		result += fmt.Sprintf("\n   %s", activeLine)
	}
	if triggerLine != "" {
		result += fmt.Sprintf("\n   %s", triggerLine)
	}

	return result, nil
}

// getBackupServices discovers backup services dynamically and returns full service names
func (sm *ServiceManager) getBackupServices() ([]string, error) {
	backupNames, err := sm.ListBackupServices()
	if err != nil {
		return nil, err
	}

	var services []string
	for _, name := range backupNames {
		services = append(services, fmt.Sprintf("kopia-backup-%s.service", name))
	}

	return services, nil
}

// getBackupTimers discovers backup timers dynamically
func (sm *ServiceManager) getBackupTimers() ([]string, error) {
	cmd := exec.Command("systemctl", "--user", "list-units", "--type=timer", "--all", "--no-legend")
	output, err := cmd.Output()
	if err != nil {
		return nil, err
	}

	var timers []string
	lines := strings.Split(string(output), "\n")
	for _, line := range lines {
		if strings.Contains(line, "kopia-backup-") && strings.Contains(line, ".timer") {
			fields := strings.Fields(line)
			for _, field := range fields {
				if strings.HasPrefix(field, "kopia-backup-") && strings.HasSuffix(field, ".timer") {
					timers = append(timers, field)
					break
				}
			}
		}
	}

	return timers, nil
}
