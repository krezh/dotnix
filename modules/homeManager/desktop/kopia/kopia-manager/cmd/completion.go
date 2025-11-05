package cmd

import (
	"os"

	"kopia-manager/internal/manager"
	"kopia-manager/internal/util"

	"github.com/spf13/cobra"
)

// completionCmd generates shell completion scripts
var completionCmd = &cobra.Command{
	Use:   "completion [bash|zsh|fish|powershell]",
	Short: "Generate completion script",
	Long: `To load completions:

Bash:
$ source <(km completion bash)

Zsh:
$ source <(km completion zsh)

Fish:
$ km completion fish | source

PowerShell:
PS> km completion powershell | Out-String | Invoke-Expression
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

func getAvailableBackupNames() []string {
	km := manager.NewKopiaManager()
	return util.GetAvailableBackupNames(km, "", "")
}

func getAvailableSnapshotIDs() []string {
	km := manager.NewKopiaManager()
	return util.GetAvailableSnapshotIDs(km, "", "")
}

func getAvailableBackupGroups() []string {
	km := manager.NewKopiaManager()
	return util.GetAvailableBackupGroups(km, "", "")
}

func getAvailableBackupNamesWithFlags(cmd *cobra.Command) []string {
	km := manager.NewKopiaManager()
	host, _ := cmd.Flags().GetString("host")
	user, _ := cmd.Flags().GetString("user")
	return util.GetAvailableBackupNames(km, host, user)
}

func getAvailableSnapshotIDsWithFlags(cmd *cobra.Command) []string {
	km := manager.NewKopiaManager()
	host, _ := cmd.Flags().GetString("host")
	user, _ := cmd.Flags().GetString("user")
	return util.GetAvailableSnapshotIDs(km, host, user)
}

func getAvailableBackupGroupsWithFlags(cmd *cobra.Command) []string {
	km := manager.NewKopiaManager()
	host, _ := cmd.Flags().GetString("host")
	user, _ := cmd.Flags().GetString("user")
	return util.GetAvailableBackupGroups(km, host, user)
}
