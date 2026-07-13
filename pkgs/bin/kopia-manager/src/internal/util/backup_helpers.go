package util

import (
	"fmt"
	"os"
	"sort"
	"strings"
	"time"

	"kopia-manager/internal/manager"
)

// dateTimeLayouts are the accepted layouts for --before/--after flags.
// Date-only inputs are interpreted as the end of that day (inclusive).
var dateTimeLayouts = []string{
	time.RFC3339,
	"2006-01-02 15:04:05",
	"2006-01-02T15:04:05",
	"2006-01-02",
	"01/02/2006",
	"01/02/2006 15:04:05",
}

// ParseDateTime parses a user-supplied date/time string into a time.Time.
// If only a date is given (no time component), it is expanded to the end of
// that day so that --before/--after behave inclusively for whole days.
func ParseDateTime(value string) (time.Time, error) {
	var lastErr error
	for _, layout := range dateTimeLayouts {
		t, err := time.Parse(layout, value)
		if err != nil {
			lastErr = err
			continue
		}
		// Date-only layout: no time component was specified.
		if layout == "2006-01-02" || layout == "01/02/2006" {
			t = t.Add(24*time.Hour - time.Nanosecond)
		}
		return t, nil
	}
	return time.Time{}, fmt.Errorf("unrecognized date/time %q (use e.g. 2006-01-02 or 2006-01-02T15:04:05): %w", value, lastErr)
}

// ExtractBackupName extracts the backup name from a snapshot's description or source path
func ExtractBackupName(snap manager.SnapshotSummary) string {
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

// FormatHostUserGroupKey formats a hostname and username into a group key for display.
func FormatHostUserGroupKey(hostname, username string) string {
	if hostname == "" {
		hostname = "unknown"
	}
	if username == "" {
		username = "unknown"
	}
	return fmt.Sprintf("%s@%s", username, hostname)
}

// GetCurrentHostname returns the current system hostname
func GetCurrentHostname() string {
	hostname, err := os.Hostname()
	if err != nil {
		return ""
	}
	return hostname
}

// GetAvailableBackupNames returns unique backup names extracted from snapshot descriptions
func GetAvailableBackupNames(km *manager.KopiaManager, hostname, username string) []string {
	if hostname == "" {
		hostname = GetCurrentHostname()
	}

	snapshots, err := km.ListSnapshots(hostname, username)
	if err != nil {
		return []string{}
	}

	backupNames := make(map[string]bool)
	for _, snap := range snapshots {
		name := ExtractBackupName(snap)
		backupNames[name] = true
	}

	var names []string
	for name := range backupNames {
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}

// GetAvailableSnapshotIDs returns all snapshot IDs for completion
func GetAvailableSnapshotIDs(km *manager.KopiaManager, hostname, username string) []string {
	if hostname == "" {
		hostname = GetCurrentHostname()
	}

	snapshots, err := km.ListSnapshots(hostname, username)
	if err != nil {
		return []string{}
	}

	var ids []string
	for _, snap := range snapshots {
		ids = append(ids, snap.ID)
	}
	sort.Strings(ids)
	return ids
}

// GetAvailableBackupGroups groups snapshots by their logical backup name
func GetAvailableBackupGroups(km *manager.KopiaManager, hostname, username string) []string {
	if hostname == "" {
		hostname = GetCurrentHostname()
	}

	snapshots, err := km.ListSnapshots(hostname, username)
	if err != nil {
		return []string{}
	}

	groups := make(map[string]bool)
	for _, snap := range snapshots {
		name := ExtractBackupName(snap)
		groups[name] = true
	}

	var groupNames []string
	for name := range groups {
		groupNames = append(groupNames, name)
	}
	sort.Strings(groupNames)
	return groupNames
}
