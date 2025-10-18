package capture

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"time"

	"recshot/internal/config"
	"recshot/internal/notify"
	"recshot/internal/process"
)

// Capturer handles screenshot and recording operations
type Capturer struct {
	config   *config.Config
	notifier *notify.Notifier
	procMgr  *process.Manager
}

// New creates a new capturer
func New(cfg *config.Config, notifier *notify.Notifier, procMgr *process.Manager) *Capturer {
	return &Capturer{
		config:   cfg,
		notifier: notifier,
		procMgr:  procMgr,
	}
}

// HyprctlWindow represents window data returned by hyprctl
type HyprctlWindow struct {
	At   [2]int `json:"at"`
	Size [2]int `json:"size"`
}

// HyprctlWorkspace represents workspace data returned by hyprctl
type HyprctlWorkspace struct {
	Monitor string `json:"monitor"`
}

// TakeScreenshot captures a screenshot based on the configured mode
func (c *Capturer) TakeScreenshot(filename string) error {
	var geometry string
	var err error

	geometry, err = c.getGeometry(c.config.Mode)

	if err != nil {
		return fmt.Errorf("failed to get capture geometry: %w", err)
	}

	return c.executeGrim(geometry, filename)
}

// StartRecording starts video recording based on the configured mode
func (c *Capturer) StartRecording(filename string) error {
	geometry, monitor, err := c.getRecordingParams(c.config.Mode)
	if err != nil {
		return fmt.Errorf("failed to get recording params: %w", err)
	}

	mode := strings.TrimPrefix(c.config.Mode, "video-")
	c.notifier.Send(fmt.Sprintf("Recording %s, run again to stop", mode))

	return c.executeWlScreenrec(geometry, monitor, filename)
}

// CheckRecording checks if a recording is active and stops it if found
func (c *Capturer) CheckRecording() (string, bool, error) {
	pids, err := c.procMgr.FindByName("wl-screenrec")
	if err != nil {
		return "", false, err
	}

	if len(pids) == 0 {
		return "", false, nil
	}

	// Find output file from command line
	var outputFile string
	for _, pid := range pids {
		cmdline, err := c.procMgr.ReadCmdline(pid)
		if err != nil {
			continue
		}

		args := strings.Fields(cmdline)
		for i, arg := range args {
			if arg == "-f" && i+1 < len(args) {
				outputFile = args[i+1]
				break
			}
		}
	}

	// Stop all recording processes
	for _, pid := range pids {
		c.procMgr.Kill(pid, syscall.SIGINT)
	}

	c.notifier.SendSuccess("Screen recording saved")

	// Small delay to ensure file is completely written
	time.Sleep(500 * time.Millisecond)

	return outputFile, true, nil
}

// getGeometry returns geometry for screenshot modes
func (c *Capturer) getGeometry(mode string) (string, error) {
	switch mode {
	case "image-area":
		return c.captureArea()
	case "image-window":
		return c.getActiveWindow()
	case "image-screen":
		return "", nil
	default:
		return "", fmt.Errorf("invalid mode: %s", mode)
	}
}

// getRecordingParams returns geometry and monitor for recording modes
func (c *Capturer) getRecordingParams(mode string) (geometry, monitor string, err error) {
	switch mode {
	case "video-area":
		geometry, err = c.runCommand("hypr-slurp")
	case "video-window":
		geometry, err = c.getActiveWindow()
	case "video-screen":
		monitor, err = c.getActiveMonitor()
	default:
		err = fmt.Errorf("invalid mode: %s", mode)
	}
	return
}

// captureArea handles area selection with hyprpicker
func (c *Capturer) captureArea() (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, "hyprpicker", "-r", "-z")
	if err := cmd.Start(); err != nil {
		return "", fmt.Errorf("failed to start hyprpicker: %w", err)
	}

	time.Sleep(200 * time.Millisecond)
	geometry, err := c.runCommandWithContext(ctx, "hypr-slurp")

	// Always cleanup hyprpicker
	c.procMgr.KillByName("hyprpicker", syscall.SIGTERM)

	if err != nil {
		fmt.Println("Selection cancelled by user")
		c.notifier.Send("Selection cancelled")
		os.Exit(1)
	}

	return geometry, err
}

// getActiveWindow gets the geometry of the active window
func (c *Capturer) getActiveWindow() (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, "hyprctl", "-j", "activewindow")
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return "", fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return "", fmt.Errorf("failed to start hyprctl: %w", err)
	}

	var window HyprctlWindow
	reader := bufio.NewReader(stdout)
	if err := json.NewDecoder(reader).Decode(&window); err != nil {
		cmd.Wait()
		return "", fmt.Errorf("failed to decode window data: %w", err)
	}

	if err := cmd.Wait(); err != nil {
		return "", fmt.Errorf("hyprctl command failed: %w", err)
	}

	return fmt.Sprintf("%d,%d %dx%d", window.At[0], window.At[1], window.Size[0], window.Size[1]), nil
}

// getActiveMonitor gets the name of the active monitor
func (c *Capturer) getActiveMonitor() (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, "hyprctl", "-j", "activeworkspace")
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return "", fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return "", fmt.Errorf("failed to start hyprctl: %w", err)
	}

	var workspace HyprctlWorkspace
	reader := bufio.NewReader(stdout)
	if err := json.NewDecoder(reader).Decode(&workspace); err != nil {
		cmd.Wait()
		return "", fmt.Errorf("failed to decode workspace data: %w", err)
	}

	if err := cmd.Wait(); err != nil {
		return "", fmt.Errorf("hyprctl command failed: %w", err)
	}

	return workspace.Monitor, nil
}

// executeGrim runs grim with the specified parameters
func (c *Capturer) executeGrim(geometry, filename string) error {
	if geometry != "" {
		return exec.Command("grim", "-g", geometry, filename).Run()
	}
	return exec.Command("grim", filename).Run()
}

// executeWlScreenrec runs wl-screenrec with the specified parameters
func (c *Capturer) executeWlScreenrec(geometry, monitor, filename string) error {
	args := []string{"wl-screenrec", "--low-power=off", "--max-fps=60", "--encode-resolution=1920x1080"}
	if geometry != "" {
		args = append(args, "-g", geometry)
	}
	if monitor != "" {
		args = append(args, "-o", monitor)
	}
	args = append(args, "-f", filename)
	return exec.Command(args[0], args[1:]...).Start()
}

// runCommand executes a command and returns its output
func (c *Capturer) runCommand(name string, args ...string) (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	return c.runCommandWithContext(ctx, name, args...)
}

// runCommandWithContext executes a command with context and returns its output
func (c *Capturer) runCommandWithContext(ctx context.Context, name string, args ...string) (string, error) {
	cmd := exec.CommandContext(ctx, name, args...)
	output, err := cmd.Output()
	if err != nil {
		return "", fmt.Errorf("command %s failed: %w", name, err)
	}
	return strings.TrimSpace(string(output)), nil
}
