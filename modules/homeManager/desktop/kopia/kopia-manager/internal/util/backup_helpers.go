package util

import (
	"os"
	"sort"
	"strings"

	"kopia-manager/internal/manager"
)

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
