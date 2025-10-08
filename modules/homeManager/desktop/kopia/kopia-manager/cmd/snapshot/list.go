package snapshot

import (
	"fmt"
	"sort"
	"strings"

	"kopia-manager/internal/manager"
	"kopia-manager/internal/ui"

	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

// ListCmd lists all snapshots grouped by backup name
var ListCmd = &cobra.Command{
	Use:   "list",
	Short: "List all snapshots",
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		snapshots, err := km.ListSnapshots()
		if err != nil {
			log.Fatal("Failed to list snapshots", "error", err)
		}

		if len(snapshots) == 0 {
			fmt.Println("No snapshots found.")
			return
		}

		// Helper to extract backup name from description or path
		getBackupName := func(snap manager.SnapshotSummary) string {
			desc := snap.Description
			if strings.HasPrefix(desc, "Automated backup: ") {
				return strings.TrimPrefix(desc, "Automated backup: ")
			}
			if strings.HasPrefix(desc, "Manual backup: ") {
				return strings.TrimPrefix(desc, "Manual backup: ")
			}
			if desc != "" {
				return desc
			}
			return snap.Source
		}

		// Group snapshots by backup name
		groups := make(map[string][]manager.SnapshotSummary)
		for _, snap := range snapshots {
			name := getBackupName(snap)
			groups[name] = append(groups[name], snap)
		}

		// Sort group names
		var groupNames []string
		for name := range groups {
			groupNames = append(groupNames, name)
		}
		sort.Strings(groupNames)

		for _, name := range groupNames {
			snaps := groups[name]
			// Sort by time descending
			sort.Slice(snaps, func(i, j int) bool {
				return snaps[i].StartTime.After(snaps[j].StartTime)
			})

			// Create table for this group
			title := fmt.Sprintf(" %s (%d snapshot%s) ", name, len(snaps), func() string {
				if len(snaps) == 1 {
					return ""
				}
				return "s"
			}())

			table := ui.NewTableBuilder(title)
			table.AddColumn("ID", ui.Dynamic)
			table.AddColumn("Path", 30)
			table.AddColumn("Time", 19)
			table.AddColumn("Size", ui.Dynamic)

			for _, snap := range snaps {
				table.AddRow(
					snap.ID,
					snap.Source,
					snap.StartTime.Format("2006-01-02 15:04:05"),
					formatSize(snap.TotalSize),
				)
			}

			fmt.Print(table.Build())
		}
	},
}

// local size formatter to avoid depending on unexported internals
func formatSize(bytes int64) string {
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
