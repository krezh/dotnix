package ui

import (
	"fmt"
	"os"
	"strings"
	"sync"
	"time"

	"charm.land/bubbles/v2/progress"
	"charm.land/lipgloss/v2"
	"github.com/charmbracelet/x/ansi"
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

// (Styles now centralized in styles.go)

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
		fmt.Print(ansi.HideCursor)
		fmt.Println(ProgressTitleStyle.Render(spb.title))
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
	fmt.Printf("\r%s%s %s", ansi.EraseLine(0), progressBar, ProgressInfoStyle.Render(info))
}

// Finish completes the progress bar and adds a newline
func (spb *SimpleProgressBar) Finish() {
	spb.Update(spb.total)

	// Print title if not yet printed
	if !spb.titlePrinted && spb.title != "" {
		fmt.Print(ansi.HideCursor)
		fmt.Println(ProgressTitleStyle.Render(spb.title))
		spb.titlePrinted = true
	}

	// Render final progress bar at 100%
	progressBar := spb.prog.ViewAs(1.0)
	info := fmt.Sprintf("%s / %s", FormatSize(spb.total), FormatSize(spb.total))

	// Print final progress bar with newline
	fmt.Printf("\r%s%s %s\n", ansi.EraseLine(0), progressBar, ProgressInfoStyle.Render(info))
	fmt.Print(ansi.ShowCursor)
}

// Spinner represents a simple spinner for indeterminate operations
type Spinner struct {
	message     string
	done        chan bool
	running     bool
	mu          sync.Mutex
	startTime   time.Time
	showElapsed bool
	frame       int
	logMode     bool
	logs        []string
	maxLogs     int
	lineCount   int // Track lines for clearing
}

// Spinner frames - simple animated dots
var spinnerFrames = []string{"⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"}

// newSpinnerBase creates a spinner with common initialization
func newSpinnerBase(message string, logMode bool, maxLogs int) *Spinner {
	return &Spinner{
		message:     message,
		done:        make(chan bool),
		startTime:   time.Now(),
		showElapsed: true,
		logMode:     logMode,
		logs:        make([]string, 0),
		maxLogs:     maxLogs,
	}
}

// NewSpinner creates a new spinner with the given message
func NewSpinner(message string) *Spinner {
	return newSpinnerBase(message, false, 5)
}

// NewSpinnerWithLogs creates a spinner with scrolling log functionality
func NewSpinnerWithLogs(message string, maxLogs int) *Spinner {
	return newSpinnerBase(message, true, maxLogs)
}

// Start begins the spinner animation
func (s *Spinner) Start() {
	s.mu.Lock()
	if s.running {
		s.mu.Unlock()
		return
	}
	s.running = true
	s.startTime = time.Now()
	s.mu.Unlock()

	fmt.Print(ansi.HideCursor)
	go s.spin()
}

// buildOutput creates the spinner output string
func (s *Spinner) buildOutput() string {
	spinnerChar := spinnerFrames[s.frame]
	styledSpinner := SpinnerStyle.Render(spinnerChar)
	output := fmt.Sprintf("%s %s", styledSpinner, ProgressTitleStyle.Render(s.message))

	if s.showElapsed {
		elapsed := time.Since(s.startTime)
		output += " " + ProgressInfoStyle.Render(fmt.Sprintf("(%s)", FormatDuration(elapsed)))
	}

	return output
}

// clearLines clears n lines from the terminal
func clearLines(n int) {
	for i := 0; i < n; i++ {
		if i == 0 {
			fmt.Print("\r" + ansi.EraseLine(2)) // Erase entire line
		} else {
			fmt.Print(ansi.CursorUp(1) + ansi.EraseLine(2)) // Move up and erase
		}
	}
	fmt.Print("\r")
}

// spin is the internal goroutine that animates the spinner
func (s *Spinner) spin() {
	ticker := time.NewTicker(100 * time.Millisecond)
	defer ticker.Stop()

	for {
		select {
		case <-s.done:
			return
		case <-ticker.C:
			s.mu.Lock()

			// Advance frame and render
			s.frame = (s.frame + 1) % len(spinnerFrames)
			s.render()

			s.mu.Unlock()
		}
	}
}

// render outputs the spinner to the terminal
func (s *Spinner) render() {
	// In log mode, we need to clear all lines when logs change
	// In simple mode, use in-place update to avoid flicker
	if s.logMode && s.lineCount > 0 {
		clearLines(s.lineCount)
	}

	// Print spinner line
	output := s.buildOutput()
	if s.logMode {
		fmt.Print(output)
	} else {
		// In simple mode, use carriage return for smoother updates
		fmt.Print("\r" + output)
	}

	// Print logs in log mode
	if s.logMode {
		for _, log := range s.logs {
			fmt.Print("\n  " + ProgressInfoStyle.Render(log))
		}
		s.lineCount = len(s.logs) + 1
	} else {
		s.lineCount = 1
	}
}

// UpdateMessage updates the spinner message
func (s *Spinner) UpdateMessage(message string) {
	s.mu.Lock()
	s.message = message
	s.mu.Unlock()
}

// AddLog adds a log entry to the scrolling log (only works in log mode)
func (s *Spinner) AddLog(logEntry string) {
	s.mu.Lock()
	defer s.mu.Unlock()

	if !s.logMode {
		return
	}

	s.logs = append(s.logs, logEntry)

	// Keep only the last maxLogs entries
	if len(s.logs) > s.maxLogs {
		s.logs = s.logs[len(s.logs)-s.maxLogs:]
	}
}

// Stop stops the spinner
func (s *Spinner) Stop() {
	s.mu.Lock()
	if !s.running {
		s.mu.Unlock()
		return
	}
	s.running = false
	s.mu.Unlock()

	s.done <- true
	clearLines(s.lineCount)
	fmt.Print(ansi.ShowCursor)
}

// printCompletionMessage formats and prints a completion message with elapsed time
func (s *Spinner) printCompletionMessage(icon, message string, style lipgloss.Style) {
	elapsed := time.Since(s.startTime)
	msg := fmt.Sprintf("%s (%s)", message, FormatDuration(elapsed))
	fmt.Printf("%s %s\n", style.Render(icon), ProgressInfoStyle.Render(msg))
}

// Success stops the spinner and prints a success message
func (s *Spinner) Success(message string) {
	s.Stop()
	s.printCompletionMessage("✓", message, SuccessStyle)
}

// Fail stops the spinner and prints an error message
func (s *Spinner) Fail(message string) {
	s.Stop()
	s.printCompletionMessage("✗", message, ErrorStyle)
}
