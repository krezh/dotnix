package ui

import (
	"fmt"
	"os"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/progress"
	"github.com/charmbracelet/lipgloss"
)

// SimpleProgressBar is a progress bar using bubbles/progress
type SimpleProgressBar struct {
	prog         progress.Model
	current      int64
	total        int64
	title        string
	startTime    time.Time
	titlePrinted bool
	lastUpdate   time.Time
	lastPercent  float64
}

// Progress bar styles using Catppuccin Mocha palette
var (
	progressTitleStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color("#cba6f7")). // Mauve
				Bold(true)

	progressInfoStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color("#bac2de")) // Subtext1
)

// FormatSize formats bytes into human-readable size
func FormatSize(bytes int64) string {
	if bytes == 0 {
		return "0 B"
	}

	const unit = 1024
	if bytes < unit {
		return fmt.Sprintf("%d B", bytes)
	}

	div, exp := int64(unit), 0
	for n := bytes / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}

	units := []string{"KB", "MB", "GB", "TB", "PB"}
	return fmt.Sprintf("%.2f %s", float64(bytes)/float64(div), units[exp])
}

// FormatDuration formats a duration into a human-readable string
func FormatDuration(d time.Duration) string {
	d = d.Round(time.Second)
	h := d / time.Hour
	d -= h * time.Hour
	m := d / time.Minute
	d -= m * time.Minute
	s := d / time.Second

	if h > 0 {
		return fmt.Sprintf("%dh%dm%ds", h, m, s)
	}
	if m > 0 {
		return fmt.Sprintf("%dm%ds", m, s)
	}
	return fmt.Sprintf("%ds", s)
}

// TruncatePath intelligently truncates a file path to fit within maxLen characters
// It preserves the most important parts (end of path) and uses "..." for truncation
// It also replaces home directory with ~ for brevity
// ShortenPath shortens a file path by replacing the home directory with ~
func ShortenPath(path string) string {
	homeDir := os.Getenv("HOME")
	if homeDir != "" && strings.HasPrefix(path, homeDir) {
		return "~" + strings.TrimPrefix(path, homeDir)
	}
	return path
}

// NewSimpleProgressBar creates a simple progress bar with default settings using bubbles/progress
func NewSimpleProgressBar(title string, total int64) *SimpleProgressBar {
	// Create bubbles progress bar with Catppuccin Mocha gradient
	prog := progress.New(
		progress.WithGradient("#a6e3a1", "#74c7ec"), // Green to Sapphire
		progress.WithWidth(40),
		progress.WithoutPercentage(), // We'll show our own stats
	)

	return &SimpleProgressBar{
		prog:      prog,
		current:   0,
		total:     total,
		title:     title,
		startTime: time.Now(),
	}
}

// Update updates the current progress
func (spb *SimpleProgressBar) Update(current int64) {
	spb.current = current
	if spb.current > spb.total {
		spb.current = spb.total
	}
}

// Print prints the progress bar (clearing the line first for in-place updates)
func (spb *SimpleProgressBar) Print() {
	// Print title once at the beginning
	if !spb.titlePrinted && spb.title != "" {
		fmt.Print("\033[?25l") // Hide cursor
		fmt.Println(progressTitleStyle.Render(spb.title))
		spb.titlePrinted = true
	}

	// Calculate percentage
	percent := float64(0)
	if spb.total > 0 {
		percent = float64(spb.current) / float64(spb.total)
	}

	// Throttle updates: only update if enough time has passed or enough progress made
	now := time.Now()
	percentChange := percent - spb.lastPercent
	timeSinceLastUpdate := now.Sub(spb.lastUpdate)

	// Update only if: 1% progress change OR 100ms passed OR first update
	if percentChange < 0.01 && timeSinceLastUpdate < 100*time.Millisecond && !spb.lastUpdate.IsZero() {
		return
	}

	spb.lastUpdate = now
	spb.lastPercent = percent

	// Render progress bar using bubbles
	progressBar := spb.prog.ViewAs(percent)

	// Build info string
	info := fmt.Sprintf("%s / %s", FormatSize(spb.current), FormatSize(spb.total))

	// Calculate speed
	if spb.current > 0 {
		elapsed := time.Since(spb.startTime).Seconds()
		if elapsed > 0 {
			speed := float64(spb.current) / elapsed
			info += fmt.Sprintf(" • %s/s", FormatSize(int64(speed)))
		}
	}

	// Calculate ETA
	if spb.current > 0 && spb.current < spb.total {
		elapsed := time.Since(spb.startTime).Seconds()
		if elapsed > 0 {
			speed := float64(spb.current) / elapsed
			remaining := float64(spb.total - spb.current)
			eta := time.Duration(remaining/speed) * time.Second
			info += fmt.Sprintf(" • ETA: %s", FormatDuration(eta))
		}
	}

	// Clear entire line and print progress bar with info
	fmt.Printf("\r\033[K%s %s", progressBar, progressInfoStyle.Render(info))
}

// Finish completes the progress bar and adds a newline
func (spb *SimpleProgressBar) Finish() {
	spb.Update(spb.total)

	// Print title if not yet printed
	if !spb.titlePrinted && spb.title != "" {
		fmt.Print("\033[?25l") // Hide cursor
		fmt.Println(progressTitleStyle.Render(spb.title))
		spb.titlePrinted = true
	}

	// Render final progress bar at 100%
	progressBar := spb.prog.ViewAs(1.0)
	info := fmt.Sprintf("%s / %s", FormatSize(spb.total), FormatSize(spb.total))

	// Print final progress bar with newline
	fmt.Printf("\r\033[K%s %s\n", progressBar, progressInfoStyle.Render(info))
	fmt.Print("\033[?25h") // Show cursor
}
