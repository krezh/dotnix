package cmd

import (
	"kopia-manager/cmd/mount"
	"kopia-manager/cmd/repo"
	"kopia-manager/cmd/services"
	"kopia-manager/cmd/snapshot"
	"kopia-manager/internal/manager"

	"github.com/spf13/cobra"
)

var (
	// Global flags
	configFile   string
	passwordFile string
)

// Root command
var rootCmd = &cobra.Command{
	Use:   manager.AppName,
	Short: "Kopia backup manager with Go library interface",
	Long:  `A comprehensive tool for managing Kopia backups using the official Go library.`,
}

// Execute runs the root command
func Execute() error {
	return rootCmd.Execute()
}

// addIfMissing safely adds subcommands only if they aren't already registered
func addIfMissing(parent *cobra.Command, cmds ...*cobra.Command) {
	exists := func(name string) bool {
		for _, c := range parent.Commands() {
			if c.Name() == name {
				return true
			}
		}
		return false
	}

	for _, cmd := range cmds {
		if cmd == nil {
			continue
		}
		if !exists(cmd.Name()) {
			parent.AddCommand(cmd)
		}
	}
}

// init wires commands and root flags.
// Guards are used to avoid double-registration during incremental refactors.
func init() {
	// Root-level persistent flags (guarded to avoid redefinition)
	if rootCmd.PersistentFlags().Lookup("config") == nil {
		rootCmd.PersistentFlags().StringVarP(&configFile, "config", "c", "", "Config file path")
	}
	if rootCmd.PersistentFlags().Lookup("password-file") == nil {
		rootCmd.PersistentFlags().StringVarP(&passwordFile, "password-file", "p", "", "Password file path")
	}

	// Wire subcommands (guarded to avoid duplicates)
	addIfMissing(
		rootCmd,
		repo.StatusCmd,
		snapshot.ListCmd,
		snapshot.BackupCmd,
		snapshot.RestoreCmd,
		snapshot.DeleteCmd,
		snapshot.InfoCmd,
		snapshot.DiffCmd,
		mount.MountCmd,
		mount.UnmountCmd,
		repo.PolicyCmd,
		snapshot.EstimateCmd,
		repo.MaintenanceCmd,
		repo.VerifyCmd,
		services.ServicesCmd,
		services.StartBackupCmd,
		services.LogsCmd,
		services.ListServicesCmd,
		mount.ListMountsCmd,
		completionCmd,
	)

	// Completion for restore command
	if snapshot.RestoreCmd != nil && snapshot.RestoreCmd.ValidArgsFunction == nil {
		snapshot.RestoreCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
			if len(args) == 0 {
				restoreAll, _ := cmd.Flags().GetBool("all")
				if restoreAll {
					return getAvailableBackupGroupsWithFlags(cmd), cobra.ShellCompDirectiveNoFileComp
				}
				return getAvailableSnapshotIDsWithFlags(cmd), cobra.ShellCompDirectiveNoFileComp
			}
			// Second arg is target directory
			return nil, cobra.ShellCompDirectiveDefault
		}
	}

	// Completion for delete command (snapshot IDs or backup groups with --all)
	if snapshot.DeleteCmd != nil && snapshot.DeleteCmd.ValidArgsFunction == nil {
		snapshot.DeleteCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
			if len(args) == 0 {
				deleteAll, _ := cmd.Flags().GetBool("all")
				if deleteAll {
					return getAvailableBackupGroupsWithFlags(cmd), cobra.ShellCompDirectiveNoFileComp
				}
				return getAvailableSnapshotIDsWithFlags(cmd), cobra.ShellCompDirectiveNoFileComp
			}
			return nil, cobra.ShellCompDirectiveNoFileComp
		}
	}

	// Completion for info command (snapshot IDs)
	if snapshot.InfoCmd != nil && snapshot.InfoCmd.ValidArgsFunction == nil {
		snapshot.InfoCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
			if len(args) == 0 {
				return getAvailableSnapshotIDsWithFlags(cmd), cobra.ShellCompDirectiveNoFileComp
			}
			return nil, cobra.ShellCompDirectiveNoFileComp
		}
	}

	// Completion for diff command (snapshot IDs for both arguments)
	if snapshot.DiffCmd != nil && snapshot.DiffCmd.ValidArgsFunction == nil {
		snapshot.DiffCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
			if len(args) < 2 {
				return getAvailableSnapshotIDsWithFlags(cmd), cobra.ShellCompDirectiveNoFileComp
			}
			return nil, cobra.ShellCompDirectiveNoFileComp
		}
	}

	// Completion for mount command (snapshot IDs + special "all")
	if mount.MountCmd != nil && mount.MountCmd.ValidArgsFunction == nil {
		mount.MountCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
			if len(args) == 0 {
				// First arg: snapshot ID or "all"
				snapshots := getAvailableSnapshotIDs()
				return append([]string{"all"}, snapshots...), cobra.ShellCompDirectiveNoFileComp
			}
			// Second arg is mount point (directory)
			return nil, cobra.ShellCompDirectiveDefault
		}
	}

	// Completion for unmount command (directory paths)
	if mount.UnmountCmd != nil && mount.UnmountCmd.ValidArgsFunction == nil {
		mount.UnmountCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
			// Suggest directories for unmount
			return nil, cobra.ShellCompDirectiveDefault
		}
	}

	// Completion for start-backup command (backup service names)
	if services.StartBackupCmd != nil && services.StartBackupCmd.ValidArgsFunction == nil {
		services.StartBackupCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
			if len(args) == 0 {
				km := manager.NewKopiaManager()
				svcs, err := km.ListBackupServices()
				if err != nil {
					return nil, cobra.ShellCompDirectiveNoFileComp
				}
				return svcs, cobra.ShellCompDirectiveNoFileComp
			}
			return nil, cobra.ShellCompDirectiveDefault
		}
	}

	// Completion for backup command (paths)
	if snapshot.BackupCmd != nil && snapshot.BackupCmd.ValidArgsFunction == nil {
		snapshot.BackupCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
			// Suggest directories/files for backup
			return nil, cobra.ShellCompDirectiveDefault
		}
	}

	// Completion for estimate command (paths)
	if snapshot.EstimateCmd != nil && snapshot.EstimateCmd.ValidArgsFunction == nil {
		snapshot.EstimateCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
			// Suggest directories/files for estimation
			return nil, cobra.ShellCompDirectiveDefault
		}
	}
}
