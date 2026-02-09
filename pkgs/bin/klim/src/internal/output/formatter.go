package output

import (
	"encoding/csv"
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/olekukonko/tablewriter"
	"gopkg.in/yaml.v3"

	"klim/internal/graph"
	"klim/internal/recommendations"
	"klim/pkg/types"
)

// Formatter handles output formatting.
type Formatter struct {
	format string
	file   string
}

// NewFormatter creates a new output formatter.
func NewFormatter(format, file string) *Formatter {
	return &Formatter{
		format: format,
		file:   file,
	}
}

// Output writes recommendations in the specified format.
func (f *Formatter) Output(recs []types.Recommendation) error {
	var output string
	var err error

	switch f.format {
	case "json":
		output, err = f.formatJSON(recs)
	case "yaml":
		output, err = f.formatYAML(recs)
	case "csv":
		return f.formatCSV(recs)
	case "html":
		output, err = f.formatHTML(recs)
	case "table":
		output, err = f.formatTable(recs)
	default:
		return fmt.Errorf("unsupported output format: %s", f.format)
	}

	if err != nil {
		return err
	}

	if f.file != "" {
		return os.WriteFile(f.file, []byte(output), 0644)
	}

	fmt.Println(output)
	return nil
}

// formatJSON formats recommendations as JSON.
func (f *Formatter) formatJSON(recs []types.Recommendation) (string, error) {
	data, err := json.MarshalIndent(recs, "", "  ")
	if err != nil {
		return "", fmt.Errorf("failed to marshal JSON: %w", err)
	}
	return string(data), nil
}

// formatYAML formats recommendations as YAML.
func (f *Formatter) formatYAML(recs []types.Recommendation) (string, error) {
	data, err := yaml.Marshal(recs)
	if err != nil {
		return "", fmt.Errorf("failed to marshal YAML: %w", err)
	}
	return string(data), nil
}

// formatCSV formats recommendations as CSV.
func (f *Formatter) formatCSV(recs []types.Recommendation) error {
	writer := csv.NewWriter(os.Stdout)
	if f.file != "" {
		file, err := os.Create(f.file)
		if err != nil {
			return fmt.Errorf("failed to create CSV file: %w", err)
		}
		defer file.Close()
		writer = csv.NewWriter(file)
	}

	// Write header
	header := []string{
		"Namespace",
		"Workload",
		"Kind",
		"Container",
		"Current Limit",
		"Recommended Limit",
		"Change %",
	}
	if err := writer.Write(header); err != nil {
		return fmt.Errorf("failed to write CSV header: %w", err)
	}

	// Write data
	for _, rec := range recs {
		row := []string{
			rec.Namespace,
			rec.WorkloadName,
			rec.WorkloadKind,
			rec.Container,
			recommendations.FormatResourceQuantity(rec.CurrentMemory),
			recommendations.FormatResourceQuantity(rec.RecommendedMemory),
			formatChangeString(rec.MemoryChange, rec.CurrentMemory.Unit != ""),
		}
		if err := writer.Write(row); err != nil {
			return fmt.Errorf("failed to write CSV row: %w", err)
		}
	}

	writer.Flush()
	return writer.Error()
}

// formatTable formats recommendations as an ASCII table.
func (f *Formatter) formatTable(recs []types.Recommendation) (string, error) {
	var builder strings.Builder
	table := tablewriter.NewTable(&builder)

	table.Header(
		"Namespace",
		"Workload",
		"Container",
		"History",
		"Current Limit",
		"Rec. Limit",
		"Δ%",
	)

	for _, rec := range recs {
		sparkline := graph.GenerateSparkline(rec.MemoryHistory, 12)

		table.Append([]interface{}{
			rec.Namespace,
			fmt.Sprintf("%s/%s", rec.WorkloadKind, rec.WorkloadName),
			rec.Container,
			sparkline,
			recommendations.FormatResourceQuantity(rec.CurrentMemory),
			recommendations.FormatResourceQuantity(rec.RecommendedMemory),
			colorChange(rec.MemoryChange, rec.Severity, rec.CurrentMemory),
		})
	}

	if err := table.Render(); err != nil {
		return "", fmt.Errorf("failed to render table: %w", err)
	}
	return builder.String(), nil
}

// formatHTML formats recommendations as HTML.
func (f *Formatter) formatHTML(recs []types.Recommendation) (string, error) {
	var builder strings.Builder

	builder.WriteString(`<!DOCTYPE html>
<html>
<head>
    <title>Klim Resource Recommendations</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #4CAF50; color: white; }
        tr:nth-child(even) { background-color: #f2f2f2; }
        .critical { color: red; font-weight: bold; }
        .warning { color: orange; font-weight: bold; }
        .info { color: green; }
    </style>
</head>
<body>
    <h1>Klim Resource Recommendations</h1>
    <table>
        <tr>
            <th>Namespace</th>
            <th>Workload</th>
            <th>Kind</th>
            <th>Container</th>
            <th>Current Limit</th>
            <th>Recommended Limit</th>
            <th>Change %</th>
        </tr>
`)

	for _, rec := range recs {
		builder.WriteString(fmt.Sprintf(`        <tr>
            <td>%s</td>
            <td>%s</td>
            <td>%s</td>
            <td>%s</td>
            <td>%s</td>
            <td>%s</td>
            <td class="%s">%s</td>
        </tr>
`,
			rec.Namespace,
			rec.WorkloadName,
			rec.WorkloadKind,
			rec.Container,
			recommendations.FormatResourceQuantity(rec.CurrentMemory),
			recommendations.FormatResourceQuantity(rec.RecommendedMemory),
			rec.Severity,
			formatChangeString(rec.MemoryChange, rec.CurrentMemory.Unit != ""),
		))
	}

	builder.WriteString(`    </table>
</body>
</html>`)

	return builder.String(), nil
}

// formatChangeString formats a percentage change, returning "N/A" if no current memory set.
func formatChangeString(change float64, hasCurrentMemory bool) string {
	if !hasCurrentMemory {
		return "N/A"
	}
	return fmt.Sprintf("%.1f%%", change)
}

// formatChange formats a percentage change with a sign.
func formatChange(change float64) string {
	if change > 0 {
		return fmt.Sprintf("+%.1f%%", change)
	}
	return fmt.Sprintf("%.1f%%", change)
}

// colorChange formats and colors a percentage change based on severity.
func colorChange(change float64, severity string, currentMemory types.ResourceQuantity) string {
	// If current limit is not set, show N/A
	if currentMemory.Unit == "" {
		return "N/A"
	}

	changeStr := formatChange(change)

	switch severity {
	case "critical":
		return fmt.Sprintf("\033[31m%s\033[0m", changeStr)
	case "warning":
		return fmt.Sprintf("\033[33m%s\033[0m", changeStr)
	case "info":
		return fmt.Sprintf("\033[32m%s\033[0m", changeStr)
	default:
		return changeStr
	}
}
