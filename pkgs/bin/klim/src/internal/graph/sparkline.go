package graph

import (
	"fmt"
	"math"
	"strings"

	"klim/pkg/types"
)

var sparkChars = []rune{'▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'}

// GenerateSparkline creates a compact sparkline visualization of memory usage.
func GenerateSparkline(history []types.MetricPoint, maxPoints int) string {
	if len(history) == 0 {
		return "─"
	}

	// Sample data if we have too many points
	data := sampleData(history, maxPoints)

	// Find min and max for scaling
	min, max := minMax(data)
	if min == max {
		return strings.Repeat(string(sparkChars[len(sparkChars)/2]), len(data))
	}

	var sb strings.Builder
	sb.Grow(len(data))

	for _, point := range data {
		valueMiB := point.Value / (1024 * 1024)
		normalized := (valueMiB - min) / (max - min)

		index := int(normalized * float64(len(sparkChars)-1))
		if index < 0 {
			index = 0
		} else if index >= len(sparkChars) {
			index = len(sparkChars) - 1
		}

		sb.WriteRune(sparkChars[index])
	}

	return sb.String()
}

// GenerateGraph creates a full ASCII graph of memory usage.
func GenerateGraph(history []types.MetricPoint, recommendedMiB, peakMiB float64, duration string) string {
	if len(history) == 0 {
		return "No data available"
	}

	const (
		height = 8
		width  = 50
	)

	// Sample data to fit width
	data := sampleData(history, width)

	// Convert to MiB
	values := make([]float64, len(data))
	for i, point := range data {
		values[i] = point.Value / (1024 * 1024)
	}

	min, max := minMaxSlice(values)

	// Ensure max includes the recommended value for scale
	if recommendedMiB > max {
		max = recommendedMiB * 1.1
	}

	var sb strings.Builder

	// Title
	sb.WriteString(fmt.Sprintf("Memory Usage (last %s)\n", duration))

	// Draw graph from top to bottom
	for row := height - 1; row >= 0; row-- {
		// Calculate the value this row represents
		rowValue := min + (max-min)*float64(row)/float64(height-1)

		// Y-axis label
		sb.WriteString(fmt.Sprintf("%4.0f ┤", rowValue))

		// Plot points
		for col := 0; col < len(values); col++ {
			value := values[col]

			// Determine what to draw
			if math.Abs(value-rowValue) < (max-min)/float64(height) {
				sb.WriteString("█")
			} else if value > rowValue {
				sb.WriteString("│")
			} else {
				sb.WriteString(" ")
			}
		}
		sb.WriteString("\n")
	}

	// X-axis
	sb.WriteString("     └")
	sb.WriteString(strings.Repeat("─", width))
	sb.WriteString("\n")

	// Time labels
	sb.WriteString("      0")
	sb.WriteString(strings.Repeat(" ", width-10))
	sb.WriteString(fmt.Sprintf("%s\n", duration))

	// Summary
	sb.WriteString(fmt.Sprintf("Peak: %.0fMi | Recommended: %.0fMi (peak + buffer)\n", peakMiB, recommendedMiB))

	return sb.String()
}

// sampleData samples the data to fit the target number of points.
func sampleData(data []types.MetricPoint, targetPoints int) []types.MetricPoint {
	if len(data) <= targetPoints {
		return data
	}

	step := float64(len(data)) / float64(targetPoints)
	sampled := make([]types.MetricPoint, targetPoints)

	for i := 0; i < targetPoints; i++ {
		index := int(float64(i) * step)
		if index >= len(data) {
			index = len(data) - 1
		}
		sampled[i] = data[index]
	}

	return sampled
}

// minMax finds the min and max values in MiB from metric points.
func minMax(data []types.MetricPoint) (float64, float64) {
	if len(data) == 0 {
		return 0, 0
	}

	min := data[0].Value / (1024 * 1024)
	max := min

	for _, point := range data {
		valueMiB := point.Value / (1024 * 1024)
		if valueMiB < min {
			min = valueMiB
		}
		if valueMiB > max {
			max = valueMiB
		}
	}

	return min, max
}

// minMaxSlice finds the min and max values in a float64 slice.
func minMaxSlice(values []float64) (float64, float64) {
	if len(values) == 0 {
		return 0, 0
	}

	min := values[0]
	max := values[0]

	for _, v := range values {
		if v < min {
			min = v
		}
		if v > max {
			max = v
		}
	}

	return min, max
}
