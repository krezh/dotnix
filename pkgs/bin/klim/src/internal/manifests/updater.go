package manifests

import (
	"fmt"
	"math"
	"os"
	"strings"

	"github.com/fatih/color"
	"klim/pkg/types"
)

// HelmReleaseUpdater updates HelmRelease manifests with new resource values.
type HelmReleaseUpdater struct{}

// NewHelmReleaseUpdater creates a new HelmRelease updater.
func NewHelmReleaseUpdater() *HelmReleaseUpdater {
	return &HelmReleaseUpdater{}
}

// Update modifies the HelmRelease YAML with new resource recommendations.
func (u *HelmReleaseUpdater) Update(filePath string, recommendations []types.Recommendation) (string, error) {
	data, err := os.ReadFile(filePath)
	if err != nil {
		return "", fmt.Errorf("failed to read file: %w", err)
	}

	content := string(data)
	lines := strings.Split(content, "\n")

	// Group recommendations by container
	recsByContainer := make(map[string]types.Recommendation)
	for _, rec := range recommendations {
		recsByContainer[rec.Container] = rec
	}

	// Track which containers we've found
	modified := false
	inContainers := false
	currentContainer := ""
	inResources := false
	inLimits := false
	inRequests := false

	for i, line := range lines {
		trimmed := strings.TrimSpace(line)

		// State transitions
		if strings.HasPrefix(trimmed, "containers:") {
			inContainers = true
			currentContainer = ""
			inResources = false
			inLimits = false
			inRequests = false
		} else if inContainers && strings.HasSuffix(trimmed, ":") && !strings.HasPrefix(trimmed, "-") {
			// Potential container name
			testName := strings.TrimSuffix(trimmed, ":")
			// Only update currentContainer if this is a container we have recommendations for
			if _, exists := recsByContainer[testName]; exists {
				currentContainer = testName
				inResources = false
				inLimits = false
				inRequests = false
			}
			// Don't clear currentContainer for other keys - just ignore them
		}

		if currentContainer != "" && strings.HasPrefix(trimmed, "resources:") {
			inResources = true
			inLimits = false
			inRequests = false
		} else if inResources && strings.HasPrefix(trimmed, "limits:") {
			inLimits = true
			inRequests = false
		} else if inResources && strings.HasPrefix(trimmed, "requests:") {
			inLimits = false
			inRequests = true
		} else if inLimits && strings.HasPrefix(trimmed, "memory:") {
			// Update the memory limit
			if currentContainer != "" {
				rec := recsByContainer[currentContainer]
				newValue := formatResourceQuantity(rec.RecommendedMemory)

				// Preserve indentation
				indent := len(line) - len(strings.TrimLeft(line, " "))
				lineIndent := strings.Repeat(" ", indent)
				lines[i] = fmt.Sprintf("%smemory: %s", lineIndent, newValue)
				modified = true
			}
		} else if inRequests && strings.HasPrefix(trimmed, "memory:") {
			// Update the memory request if it needs to be lowered
			if currentContainer != "" {
				rec := recsByContainer[currentContainer]
				if rec.RequestLowered {
					newValue := formatResourceQuantity(rec.RecommendedRequest)

					// Preserve indentation
					indent := len(line) - len(strings.TrimLeft(line, " "))
					lineIndent := strings.Repeat(" ", indent)
					lines[i] = fmt.Sprintf("%smemory: %s", lineIndent, newValue)
					modified = true
				}
			}
		}
	}

	if !modified {
		return "", fmt.Errorf("no matching containers found in manifest")
	}

	return strings.Join(lines, "\n"), nil
}

// formatResourceQuantity formats a resource quantity as a string.
func formatResourceQuantity(rq types.ResourceQuantity) string {
	if rq.Unit == "" {
		return ""
	}
	// Always round up to nearest integer
	return fmt.Sprintf("%d%s", int64(math.Ceil(rq.Value)), rq.Unit)
}

// ApplyChanges writes the updated YAML back to the file.
func (u *HelmReleaseUpdater) ApplyChanges(filePath, updatedContent string) error {
	return os.WriteFile(filePath, []byte(updatedContent), 0644)
}

// GenerateDiff creates a simple diff between original and updated content.
func GenerateDiff(filePath, original, updated string) string {
	var diff strings.Builder

	// Color setup
	red := color.New(color.FgRed)
	green := color.New(color.FgGreen)
	cyan := color.New(color.FgCyan)

	cyan.Fprintf(&diff, "--- %s\n", filePath)
	cyan.Fprintf(&diff, "+++ %s (updated)\n\n", filePath)

	origLines := strings.Split(original, "\n")
	updatedLines := strings.Split(updated, "\n")

	maxLines := len(origLines)
	if len(updatedLines) > maxLines {
		maxLines = len(updatedLines)
	}

	contextLines := 3
	inDiff := false
	linesSinceChange := 0

	for i := 0; i < maxLines; i++ {
		var origLine, updatedLine string

		if i < len(origLines) {
			origLine = origLines[i]
		}
		if i < len(updatedLines) {
			updatedLine = updatedLines[i]
		}

		if origLine != updatedLine {
			// Print context before the change
			if !inDiff {
				start := i - contextLines
				if start < 0 {
					start = 0
				}
				for j := start; j < i; j++ {
					if j < len(origLines) {
						diff.WriteString(fmt.Sprintf("  %s\n", origLines[j]))
					}
				}
			}

			if origLine != "" && updatedLine != "" {
				red.Fprintf(&diff, "- %s\n", origLine)
				green.Fprintf(&diff, "+ %s\n", updatedLine)
			} else if origLine != "" {
				red.Fprintf(&diff, "- %s\n", origLine)
			} else if updatedLine != "" {
				green.Fprintf(&diff, "+ %s\n", updatedLine)
			}

			inDiff = true
			linesSinceChange = 0
		} else if inDiff {
			// Print context after the change
			if linesSinceChange < contextLines {
				diff.WriteString(fmt.Sprintf("  %s\n", origLine))
				linesSinceChange++
			} else {
				inDiff = false
				diff.WriteString("\n")
			}
		}
	}

	return diff.String()
}
