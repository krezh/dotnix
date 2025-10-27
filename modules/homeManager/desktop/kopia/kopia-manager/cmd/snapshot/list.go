package snapshot

import (
	"fmt"
	"os"
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
	Short: "List snapshots from this computer (use --all for all hosts)",
	Run: func(cmd *cobra.Command, args []string) {
		all, _ := cmd.Flags().GetBool("all")
		km := manager.NewKopiaManager()

		// Determine which hostname to filter by
		hostname := ""
		if !all {
			var err error
			hostname, err = os.Hostname()
			if err != nil {
				log.Warn("Could not get hostname, showing all snapshots", "error", err)
				hostname = "" // Show all if we can't determine hostname
			}
		}

		// List snapshots with optional hostname filter
		snapshots, err := km.ListSnapshots(hostname)
		if err != nil {
			log.Fatal("Failed to list snapshots", "error", err)
		}

		if len(snapshots) == 0 {
			if hostname != "" {
				ui.Infof("No snapshots found for host '%s'. Use --all to see snapshots from other hosts.", hostname)
			} else {
				ui.Info("No snapshots found.")
			}
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
			title := fmt.Sprintf("%s (%d snapshot%s)", name, len(snaps), func() string {
				if len(snaps) == 1 {
					return ""
				}
				return "s"
			}())

			headers := []string{"ID", "Path", "Time", "Size"}
			var rows [][]string

			for _, snap := range snaps {
				rows = append(rows, []string{
					snap.ID,
					ui.ShortenPath(snap.Source),
					snap.StartTime.Format("2006-01-02 15:04:05"),
					ui.FormatSize(snap.TotalSize),
				})
			}

			fmt.Println(ui.RenderTable(title, headers, rows))
		}
	},
}

func init() {
	ListCmd.Flags().BoolP("all", "A", false, "Show snapshots from all hosts")
}
