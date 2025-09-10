package capture

import (
	"encoding/json"
	"fmt"
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
	case "image-full":
		return "", nil
	default:
		return "", fmt.Errorf("invalid mode: %s", mode)
	}
}

// getRecordingParams returns geometry and monitor for recording modes
func (c *Capturer) getRecordingParams(mode string) (geometry, monitor string, err error) {
	switch mode {
	case "video-area":
		geometry, err = c.runCommand("slurp")
	case "video-window":
		geometry, err = c.getActiveWindow()
	case "video-full":
		monitor, err = c.getActiveMonitor()
	default:
		err = fmt.Errorf("invalid mode: %s", mode)
	}
	return
}

// captureArea handles area selection with hyprpicker
func (c *Capturer) captureArea() (string, error) {
	exec.Command("hyprpicker", "-r", "-z").Start()
	time.Sleep(200 * time.Millisecond)
	geometry, err := c.runCommand("slurp")
	c.procMgr.KillByName("hyprpicker", syscall.SIGTERM)
	return geometry, err
}

// getActiveWindow gets the geometry of the active window
func (c *Capturer) getActiveWindow() (string, error) {
	cmd := exec.Command("hyprctl", "-j", "activewindow")
	output, err := cmd.StdoutPipe()
	if err != nil {
		return "", err
	}

	if err := cmd.Start(); err != nil {
		return "", err
	}

	var window HyprctlWindow
	if err := json.NewDecoder(output).Decode(&window); err != nil {
		return "", err
	}

	cmd.Wait()
	return fmt.Sprintf("%d,%d %dx%d", window.At[0], window.At[1], window.Size[0], window.Size[1]), nil
}

// getActiveMonitor gets the name of the active monitor
func (c *Capturer) getActiveMonitor() (string, error) {
	cmd := exec.Command("hyprctl", "-j", "activeworkspace")
	output, err := cmd.StdoutPipe()
	if err != nil {
		return "", err
	}

	if err := cmd.Start(); err != nil {
		return "", err
	}

	var workspace HyprctlWorkspace
	if err := json.NewDecoder(output).Decode(&workspace); err != nil {
		return "", err
	}

	cmd.Wait()
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
	args := []string{"wl-screenrec", "--low-power=off"}
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
	output, err := exec.Command(name, args...).Output()
	return strings.TrimSpace(string(output)), err
}
