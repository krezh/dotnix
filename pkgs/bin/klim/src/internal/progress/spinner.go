package progress

import (
	"fmt"
	"sort"
	"sync"
	"time"

	"github.com/fatih/color"
)

var spinnerFrames = []string{"⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"}

// Tracker tracks progress of pod analysis.
type Tracker struct {
	mu            sync.Mutex
	processing    map[string]string // pod name -> namespace
	completed     int
	total         int
	frameIndex    int
	lastLineCount int
	done          chan struct{}
	verbose       bool
	maxConcurrent int
}

// NewTracker creates a new progress tracker.
func NewTracker(total, maxConcurrent int, verbose bool) *Tracker {
	return &Tracker{
		processing:    make(map[string]string),
		total:         total,
		done:          make(chan struct{}),
		verbose:       verbose,
		maxConcurrent: maxConcurrent,
	}
}

// UpdateTotal updates the total count for the tracker.
func (t *Tracker) UpdateTotal(total int) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.total = total
}

// Start starts the progress display.
func (t *Tracker) Start() {
	if t.verbose {
		return // No spinner in verbose mode
	}

	go func() {
		ticker := time.NewTicker(100 * time.Millisecond)
		defer ticker.Stop()

		for {
			select {
			case <-ticker.C:
				t.render()
			case <-t.done:
				t.clearLines()
				return
			}
		}
	}()
}

// StartProcessing marks a pod as being processed.
func (t *Tracker) StartProcessing(namespace, podName string) {
	if t.verbose {
		return
	}

	t.mu.Lock()
	defer t.mu.Unlock()
	t.processing[podName] = namespace
}

// FinishProcessing marks a pod as completed.
func (t *Tracker) FinishProcessing(podName string) {
	if t.verbose {
		return
	}

	t.mu.Lock()
	defer t.mu.Unlock()
	delete(t.processing, podName)
	t.completed++
}

// Stop stops the progress display.
func (t *Tracker) Stop() {
	if t.verbose {
		return
	}

	close(t.done)
	time.Sleep(150 * time.Millisecond) // Let it finish rendering
}

// render draws the current state.
func (t *Tracker) render() {
	t.mu.Lock()
	defer t.mu.Unlock()

	// Clear previous lines
	if t.lastLineCount > 0 {
		for i := 0; i < t.lastLineCount; i++ {
			fmt.Print("\033[A\033[2K") // Move up and clear line
		}
	}

	// Spinner and progress
	cyan := color.New(color.FgCyan)
	green := color.New(color.FgGreen)

	spinner := spinnerFrames[t.frameIndex]
	t.frameIndex = (t.frameIndex + 1) % len(spinnerFrames)

	cyan.Printf("%s Analyzing pods: ", spinner)
	green.Printf("%d/%d completed\n", t.completed, t.total)

	lineCount := 1 // Progress line

	// Show currently processing pods (sorted for stability)
	if len(t.processing) > 0 {
		// Collect and sort pods for stable display
		type podEntry struct {
			namespace string
			name      string
		}
		var pods []podEntry
		for pod, ns := range t.processing {
			pods = append(pods, podEntry{namespace: ns, name: pod})
		}
		sort.Slice(pods, func(i, j int) bool {
			if pods[i].namespace != pods[j].namespace {
				return pods[i].namespace < pods[j].namespace
			}
			return pods[i].name < pods[j].name
		})

		// Display up to maxConcurrent pods
		displayCount := len(pods)
		if displayCount > t.maxConcurrent {
			displayCount = t.maxConcurrent
		}
		for i := 0; i < displayCount; i++ {
			fmt.Printf("  %s %s/%s\n", spinner, pods[i].namespace, pods[i].name)
			lineCount++
		}
	}

	t.lastLineCount = lineCount
}

// clearLines clears all rendered lines.
func (t *Tracker) clearLines() {
	if t.lastLineCount > 0 {
		for i := 0; i < t.lastLineCount; i++ {
			fmt.Print("\033[A\033[2K") // Move up and clear line
		}
	}
}

// GetProcessingList returns a snapshot of currently processing pods.
func (t *Tracker) GetProcessingList() []string {
	t.mu.Lock()
	defer t.mu.Unlock()

	var result []string
	for pod, ns := range t.processing {
		result = append(result, fmt.Sprintf("%s/%s", ns, pod))
	}
	return result
}

// Summary returns a summary message.
func (t *Tracker) Summary() string {
	t.mu.Lock()
	defer t.mu.Unlock()

	if t.completed == 0 {
		return "No pods analyzed"
	}

	return fmt.Sprintf("Analyzed %d/%d pods", t.completed, t.total)
}
