package system

import (
	"bufio"
	"bytes"
	"fmt"
	"os/exec"
	"regexp"
	"strings"
)

// ServiceStatus represents the status of a systemd service.
type ServiceStatus string

const (
	StatusActive   ServiceStatus = "active"
	StatusInactive ServiceStatus = "inactive"
	StatusFailed   ServiceStatus = "failed"
	StatusUnknown  ServiceStatus = "unknown"
)

// Service represents a systemd service.
type Service struct {
	Name        string
	Description string
	Status      ServiceStatus
	Enabled     bool
	Running     bool
}

// GetServices retrieves a list of systemd services.
func GetServices(filter string) ([]Service, error) {
	cmd := exec.Command("systemctl", "list-units", "--type=service", "--all", "--no-pager", "--plain")
	output, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("failed to list services: %w", err)
	}

	var services []Service
	scanner := bufio.NewScanner(bytes.NewReader(output))

	// Skip header
	for scanner.Scan() {
		line := scanner.Text()
		if strings.Contains(line, "UNIT") {
			break
		}
	}

	// Parse service lines
	for scanner.Scan() {
		line := scanner.Text()
		if line == "" || strings.HasPrefix(line, "LOAD") {
			continue
		}

		fields := strings.Fields(line)
		if len(fields) < 4 {
			continue
		}

		name := fields[0]
		if !strings.HasSuffix(name, ".service") {
			continue
		}

		status := parseServiceStatus(fields[2])
		running := fields[3] == "running"

		// Apply filter
		if filter != "" && filter != "all" {
			activeState := strings.ToLower(fields[2])
			if filter == "active" && activeState != "active" {
				continue
			}
			if filter == "failed" && activeState != "failed" {
				continue
			}
			if filter == "running" && !running {
				continue
			}
			if filter == "stopped" && running {
				continue
			}
		}

		// Get description (rest of the line after first 4 fields)
		description := ""
		if len(fields) > 4 {
			description = strings.Join(fields[4:], " ")
		}

		// Check if enabled
		enabled := isServiceEnabled(name)

		services = append(services, Service{
			Name:        name,
			Description: description,
			Status:      status,
			Enabled:     enabled,
			Running:     running,
		})
	}

	return services, nil
}

// GetServiceDetails retrieves detailed information about a service.
func GetServiceDetails(serviceName string) (*Service, error) {
	// Sanitize service name
	if !isValidServiceName(serviceName) {
		return nil, fmt.Errorf("invalid service name: %s", serviceName)
	}

	cmd := exec.Command("systemctl", "status", serviceName, "--no-pager")
	output, err := cmd.Output()

	// systemctl status returns non-zero for inactive services
	if err != nil && output == nil {
		return nil, fmt.Errorf("failed to get service status: %w", err)
	}

	service := &Service{
		Name:    serviceName,
		Enabled: isServiceEnabled(serviceName),
	}

	// Parse output
	scanner := bufio.NewScanner(bytes.NewReader(output))
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())

		if strings.HasPrefix(line, "Active:") {
			parts := strings.Fields(line)
			if len(parts) >= 2 {
				service.Status = parseServiceStatus(parts[1])
				service.Running = strings.Contains(line, "running")
			}
		} else if strings.HasPrefix(line, "Description:") {
			service.Description = strings.TrimPrefix(line, "Description:")
			service.Description = strings.TrimSpace(service.Description)
		}
	}

	return service, nil
}

// GetServiceLogs retrieves the last n lines of logs for a service.
func GetServiceLogs(serviceName string, lines int) (string, error) {
	// Sanitize service name
	if !isValidServiceName(serviceName) {
		return "", fmt.Errorf("invalid service name: %s", serviceName)
	}

	cmd := exec.Command("journalctl", "-u", serviceName, "-n", fmt.Sprintf("%d", lines), "--no-pager")
	output, err := cmd.Output()
	if err != nil {
		return "", fmt.Errorf("failed to get service logs: %w", err)
	}

	return string(output), nil
}

// StreamServiceLogs streams service logs in real-time.
// Returns a command that can be read from and a channel that signals when to stop.
func StreamServiceLogs(serviceName string, lines int) (*exec.Cmd, error) {
	// Sanitize service name
	if !isValidServiceName(serviceName) {
		return nil, fmt.Errorf("invalid service name: %s", serviceName)
	}

	// Use -f to follow logs, -n to show last N lines
	cmd := exec.Command("journalctl", "-u", serviceName, "-n", fmt.Sprintf("%d", lines), "-f", "--no-pager")
	return cmd, nil
}

// StartService starts a systemd service.
func StartService(serviceName string) error {
	if !isValidServiceName(serviceName) {
		return fmt.Errorf("invalid service name: %s", serviceName)
	}

	cmd := exec.Command("systemctl", "start", serviceName)
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to start service: %w", err)
	}
	return nil
}

// StopService stops a systemd service.
func StopService(serviceName string) error {
	if !isValidServiceName(serviceName) {
		return fmt.Errorf("invalid service name: %s", serviceName)
	}

	cmd := exec.Command("systemctl", "stop", serviceName)
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to stop service: %w", err)
	}
	return nil
}

// RestartService restarts a systemd service.
func RestartService(serviceName string) error {
	if !isValidServiceName(serviceName) {
		return fmt.Errorf("invalid service name: %s", serviceName)
	}

	cmd := exec.Command("systemctl", "restart", serviceName)
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to restart service: %w", err)
	}
	return nil
}

// StartServiceWithSudo starts a systemd service using sudo with the provided password.
func StartServiceWithSudo(serviceName string, sudoPassword string) error {
	if !isValidServiceName(serviceName) {
		return fmt.Errorf("invalid service name: %s", serviceName)
	}

	cmd := exec.Command("sudo", "-S", "systemctl", "start", serviceName)
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return fmt.Errorf("failed to get stdin pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start command: %w", err)
	}

	if _, err := stdin.Write([]byte(sudoPassword + "\n")); err != nil {
		return fmt.Errorf("failed to write password: %w", err)
	}
	stdin.Close()

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("failed to start service: %w", err)
	}
	return nil
}

// StopServiceWithSudo stops a systemd service using sudo with the provided password.
func StopServiceWithSudo(serviceName string, sudoPassword string) error {
	if !isValidServiceName(serviceName) {
		return fmt.Errorf("invalid service name: %s", serviceName)
	}

	cmd := exec.Command("sudo", "-S", "systemctl", "stop", serviceName)
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return fmt.Errorf("failed to get stdin pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start command: %w", err)
	}

	if _, err := stdin.Write([]byte(sudoPassword + "\n")); err != nil {
		return fmt.Errorf("failed to write password: %w", err)
	}
	stdin.Close()

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("failed to stop service: %w", err)
	}
	return nil
}

// RestartServiceWithSudo restarts a systemd service using sudo with the provided password.
func RestartServiceWithSudo(serviceName string, sudoPassword string) error {
	if !isValidServiceName(serviceName) {
		return fmt.Errorf("invalid service name: %s", serviceName)
	}

	cmd := exec.Command("sudo", "-S", "systemctl", "restart", serviceName)
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return fmt.Errorf("failed to get stdin pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start command: %w", err)
	}

	if _, err := stdin.Write([]byte(sudoPassword + "\n")); err != nil {
		return fmt.Errorf("failed to write password: %w", err)
	}
	stdin.Close()

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("failed to restart service: %w", err)
	}
	return nil
}

// EnableService enables a systemd service to start on boot.
func EnableService(serviceName string) error {
	if !isValidServiceName(serviceName) {
		return fmt.Errorf("invalid service name: %s", serviceName)
	}

	cmd := exec.Command("systemctl", "enable", serviceName)
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to enable service: %w", err)
	}
	return nil
}

// DisableService disables a systemd service from starting on boot.
func DisableService(serviceName string) error {
	if !isValidServiceName(serviceName) {
		return fmt.Errorf("invalid service name: %s", serviceName)
	}

	cmd := exec.Command("systemctl", "disable", serviceName)
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to disable service: %w", err)
	}
	return nil
}

// isServiceEnabled checks if a service is enabled.
func isServiceEnabled(serviceName string) bool {
	cmd := exec.Command("systemctl", "is-enabled", serviceName)
	output, _ := cmd.Output()
	return strings.TrimSpace(string(output)) == "enabled"
}

// parseServiceStatus converts systemctl status string to ServiceStatus.
func parseServiceStatus(status string) ServiceStatus {
	status = strings.ToLower(status)
	switch status {
	case "active":
		return StatusActive
	case "inactive", "dead":
		return StatusInactive
	case "failed":
		return StatusFailed
	default:
		return StatusUnknown
	}
}

// isValidServiceName validates a service name to prevent command injection.
func isValidServiceName(name string) bool {
	// Allow alphanumeric, hyphens, underscores, dots, and @
	// This matches typical systemd service naming conventions
	matched, _ := regexp.MatchString(`^[a-zA-Z0-9@._-]+\.service$`, name)
	return matched
}
