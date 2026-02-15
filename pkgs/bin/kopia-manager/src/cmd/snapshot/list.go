package snapshot

import (
	"fmt"
	"os"
	"sort"
	"strings"

	"kopia-manager/internal/manager"
	"kopia-manager/internal/ui"
	"kopia-manager/internal/util"

	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

// ListCmd lists all snapshots grouped by host+user combination
var ListCmd = &cobra.Command{
	Use:   "list",
	Short: "List snapshots from this computer (use --all for all hosts)",
	Run: func(cmd *cobra.Command, args []string) {
		all, _ := cmd.Flags().GetBool("all")
		host, _ := cmd.Flags().GetString("host")
		user, _ := cmd.Flags().GetString("user")
		km := manager.NewKopiaManager()

		// Determine which hostname to filter by
		hostname := host
		if hostname == "" {
			if all {
				// --all specified without --host: show all hosts
				hostname = ""
			} else {
				// No flags specified: use current system hostname
				var err error
				hostname, err = os.Hostname()
				if err != nil {
					log.Warn("Could not get hostname, showing all snapshots", "error", err)
					hostname = ""
				}
			}
		}

		snapshots, err := km.ListSnapshots(hostname, user)
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

		// Group snapshots by backup name + host@user combination
		groups := make(map[string][]manager.SnapshotSummary)
		for _, snap := range snapshots {
			backupName := util.ExtractBackupName(snap)
			hostUserKey := util.FormatHostUserGroupKey(snap.Hostname, snap.Username)
			groupKey := backupName + "|" + hostUserKey
			groups[groupKey] = append(groups[groupKey], snap)
		}

		// Sort group keys
		var groupKeys []string
		for key := range groups {
			groupKeys = append(groupKeys, key)
		}
		sort.Strings(groupKeys)

		for _, key := range groupKeys {
			snaps := groups[key]
			// Sort by time descending
			sort.Slice(snaps, func(i, j int) bool {
				return snaps[i].StartTime.After(snaps[j].StartTime)
			})

			parts := strings.SplitN(key, "|", 2)
			backupName, hostUserKey := parts[0], parts[1]

			pluralSuffix := "s"
			if len(snaps) == 1 {
				pluralSuffix = ""
			}
			title := fmt.Sprintf("%s @ %s (%d snapshot%s)", backupName, hostUserKey, len(snaps), pluralSuffix)

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
	ListCmd.Flags().StringP("host", "H", "", "Filter snapshots by hostname")
	ListCmd.Flags().StringP("user", "U", "", "Filter snapshots by username")
}
