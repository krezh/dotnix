package repo

import (
	"fmt"
	"strings"

	"kopia-manager/internal/manager"

	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

// PolicyCmd shows backup policy for a given path and provides path completion
var PolicyCmd = &cobra.Command{
	Use:   "policy [path]",
	Short: "Show backup policy for a path",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := manager.NewKopiaManager()
		path := args[0]

		policy, err := km.ShowPolicy(path)
		if err != nil {
			log.Fatal("Failed to get policy", "error", err)
		}

		fmt.Print(policy)
	},
	ValidArgsFunction: func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
		km := manager.NewKopiaManager()
		snapshots, err := km.ListSnapshots()
		if err != nil {
			return nil, cobra.ShellCompDirectiveNoFileComp
		}
		// Deduplicate paths
		pathSet := make(map[string]struct{})
		for _, snap := range snapshots {
			pathSet[snap.Source] = struct{}{}
		}
		var completions []string
		for path := range pathSet {
			if strings.HasPrefix(path, toComplete) {
				completions = append(completions, path)
			}
		}
		return completions, cobra.ShellCompDirectiveNoFileComp
	},
}
