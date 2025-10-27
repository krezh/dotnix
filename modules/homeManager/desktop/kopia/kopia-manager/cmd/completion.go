package cmd

import (
	"os"
	"sort"
	"strings"

	"kopia-manager/internal/manager"

	"github.com/spf13/cobra"
)

// completionCmd generates shell completion scripts
var completionCmd = &cobra.Command{
	Use:   "completion [bash|zsh|fish|powershell]",
	Short: "Generate completion script",
	Long: `To load completions:

Bash:
$ source <(kopia-manager completion bash)

Zsh:
$ source <(kopia-manager completion zsh)

Fish:
$ kopia-manager completion fish | source

PowerShell:
PS> kopia-manager completion powershell | Out-String | Invoke-Expression
`,
	DisableFlagsInUseLine: true,
	ValidArgs:             []string{"bash", "zsh", "fish", "powershell"},
	Args:                  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		switch args[0] {
		case "bash":
			cmd.Root().GenBashCompletion(os.Stdout)
		case "zsh":
			cmd.Root().GenZshCompletion(os.Stdout)
		case "fish":
			cmd.Root().GenFishCompletion(os.Stdout, true)
		case "powershell":
			cmd.Root().GenPowerShellCompletion(os.Stdout)
		}
	},
}

// getAvailableBackupNames returns unique backup names extracted from snapshot descriptions.
func getAvailableBackupNames() []string {
	km := manager.NewKopiaManager()
	snapshots, err := km.ListSnapshots("")
	if err != nil {
		return []string{}
	}

	backupNames := make(map[string]bool)
	for _, snap := range snapshots {
		desc := snap.Description
		if strings.HasPrefix(desc, "Automated backup: ") {
			name := strings.TrimPrefix(desc, "Automated backup: ")
			backupNames[name] = true
		} else if strings.HasPrefix(desc, "Manual backup: ") {
			name := strings.TrimPrefix(desc, "Manual backup: ")
			backupNames[name] = true
		} else if desc != "" {
			backupNames[desc] = true
		}
	}

	var names []string
	for name := range backupNames {
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}

// getAvailableSnapshotIDs returns all snapshot IDs for completion.
func getAvailableSnapshotIDs() []string {
	km := manager.NewKopiaManager()
	snapshots, err := km.ListSnapshots("")
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

// getAvailableBackupGroups groups snapshots by their logical backup name.
func getAvailableBackupGroups() []string {
	km := manager.NewKopiaManager()
	snapshots, err := km.ListSnapshots("")
	if err != nil {
		return []string{}
	}

	groups := make(map[string]bool)
	for _, snap := range snapshots {
		desc := snap.Description
		if strings.HasPrefix(desc, "Automated backup: ") {
			name := strings.TrimPrefix(desc, "Automated backup: ")
			groups[name] = true
		} else if strings.HasPrefix(desc, "Manual backup: ") {
			name := strings.TrimPrefix(desc, "Manual backup: ")
			groups[name] = true
		} else if desc != "" {
			groups[desc] = true
		} else {
			// Use source path as fallback
			groups[snap.Source] = true
		}
	}

	var groupNames []string
	for name := range groups {
		groupNames = append(groupNames, name)
	}
	sort.Strings(groupNames)
	return groupNames
}
