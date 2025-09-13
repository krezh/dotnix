package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/charmbracelet/log"
	"github.com/spf13/cobra"
)

var (
	configFile   string
	passwordFile string
	logger       = log.New(os.Stderr)
)

// Root command
var rootCmd = &cobra.Command{
	Use:   AppName,
	Short: "Kopia backup manager with CLI interface",
	Long:  `A comprehensive tool for managing Kopia backups with command-line interface.`,
}

// Status command
var statusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show repository status",
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()
		status, err := km.GetStatus()
		if err != nil {
			logger.Fatal("Failed to get status", "error", err)
		}
		fmt.Print(status)
	},
}

// List command
var listCmd = &cobra.Command{
	Use:   "list",
	Short: "List all snapshots",
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()
		snapshots, err := km.ListSnapshots()
		if err != nil {
			logger.Fatal("Failed to list snapshots", "error", err)
		}

		if len(snapshots) == 0 {
			fmt.Println("No snapshots found.")
			return
		}

		// Helper to extract backup name from tags, then description, then path
		getBackupName := func(snap KopiaSnapshot) string {
			// Try to extract from description: "Automated backup: <name>" or tag "name:<name>"
			// (If you add tags to Kopia, you can parse them here)
			desc := snap.Description
			if strings.HasPrefix(desc, "Automated backup: ") {
				return strings.TrimPrefix(desc, "Automated backup: ")
			}
			if strings.HasPrefix(desc, "Manual backup: ") {
				return strings.TrimPrefix(desc, "Manual backup: ")
			}
			// fallback: use description or path
			if desc != "" {
				return desc
			}
			return snap.Source.Path
		}

		// Group snapshots by backup name
		groups := make(map[string][]KopiaSnapshot)
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

		// Box drawing characters and fixed widths
		// Fixed column widths
		idWidth := 20
		pathWidth := 30
		timeWidth := 19
		sizeWidth := 15
		vertical := "│"
		// Build a sample row to determine box width
		sampleRow := fmt.Sprintf("%s %-*s %s %-*s %s %-*s %s %-*s %s",
			vertical, idWidth, "ID", vertical, pathWidth, "Path", vertical, timeWidth, "Time", vertical, sizeWidth, "Size", vertical)
		boxWidth := len([]rune(sampleRow))
		topLeft, topRight := "╭", "╮"
		bottomLeft, bottomRight := "╰", "╯"
		horizontal := "─"
		// Build header separator to match boxWidth
		headerSep := "├"
		headerSep += strings.Repeat(horizontal, idWidth+2) + "┬"
		headerSep += strings.Repeat(horizontal, pathWidth+2) + "┬"
		headerSep += strings.Repeat(horizontal, timeWidth+2) + "┬"
		headerSep += strings.Repeat(horizontal, sizeWidth+2) + "┤"
		// Pad headerSep to boxWidth
		if l := len([]rune(headerSep)); l < boxWidth {
			headerSep += strings.Repeat(horizontal, boxWidth-l)
		} else if l > boxWidth {
			headerSep = string([]rune(headerSep)[:boxWidth])
		}
		colHeader := sampleRow

		for _, name := range groupNames {
			snaps := groups[name]
			// Sort by time descending
			sort.Slice(snaps, func(i, j int) bool {
				return snaps[i].StartTime.After(snaps[j].StartTime)
			})
			// Section header with count
			title := fmt.Sprintf(" %s (%d snapshot%s) ", name, len(snaps), func() string {
				if len(snaps) == 1 {
					return ""
				} else {
					return "s"
				}
			}())
			pad := boxWidth - len([]rune(title))
			leftPad := pad / 2
			rightPad := pad - leftPad
			// Build top border to match boxWidth and always end with topRight
			topBorder := fmt.Sprintf("%s%s%s%s%s", topLeft, strings.Repeat(horizontal, leftPad), title, strings.Repeat(horizontal, rightPad), topRight)
			runes := []rune(topBorder)
			if len(runes) < boxWidth {
				// pad with horizontal, then add topRight
				topBorder = string(runes[:len(runes)-1]) + strings.Repeat(horizontal, boxWidth-len(runes)) + topRight
			} else if len(runes) > boxWidth {
				// truncate and ensure last char is topRight
				topBorder = string(runes[:boxWidth-1]) + topRight
			}
			fmt.Println(topBorder)
			fmt.Println(colHeader)
			fmt.Println(headerSep)
			for _, snap := range snaps {
				shortID := snap.ID
				if len(shortID) > idWidth {
					shortID = shortID[:idWidth]
				}
				shortPath := snap.Source.Path
				if len(shortPath) > pathWidth {
					shortPath = shortPath[:pathWidth-3] + "..."
				}
				fmt.Printf("%s %-*s %s %-*s %s %-*s %s %-*s %s\n",
					vertical, idWidth, shortID,
					vertical, pathWidth, shortPath,
					vertical, timeWidth, snap.StartTime.Format("2006-01-02 15:04:05"),
					vertical, sizeWidth, formatSize(snap.Stats.TotalSize),
					vertical)
			}
			// Build bottom border to match boxWidth
			bottomBorder := fmt.Sprintf("%s%s%s", bottomLeft, strings.Repeat(horizontal, boxWidth-2), bottomRight)
			fmt.Printf("%s\n\n", bottomBorder)
		}
	},
}

// Backup command
var backupCmd = &cobra.Command{
	Use:   "backup [paths...]",
	Short: "Create a backup of specified paths",
	Args:  cobra.MinimumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()
		description := fmt.Sprintf("Manual backup: %s", time.Now().Format("2006-01-02 15:04:05"))

		logger.Info("Starting backup", "paths", args)

		if err := km.CreateBackup(args, description); err != nil {
			logger.Fatal("Backup failed", "error", err)
		}

		logger.Info("Backup completed successfully")
	},
}

// Restore command
var restoreCmd = &cobra.Command{
	Use:   "restore [snapshot-id] [target-directory]",
	Short: "Restore a snapshot to target directory",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()
		snapshotID := args[0]
		targetDir := args[1]

		logger.Info("Starting restore", "snapshot", snapshotID, "target", targetDir)

		if err := km.RestoreSnapshot(snapshotID, targetDir); err != nil {
			logger.Fatal("Restore failed", "error", err)
		}

		logger.Info("Restore completed successfully")
	},
}

// Delete command
var deleteAll bool
var deleteCmd = &cobra.Command{
	Use:   "delete [snapshot-id]",
	Short: "Delete a specific snapshot or all snapshots with --all",
	Args:  cobra.MaximumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()
		if deleteAll {
			if err := km.DeleteSnapshot("", true); err != nil {
				logger.Fatal("Delete all failed", "error", err)
			}
			return
		}
		if len(args) != 1 {
			fmt.Fprintln(os.Stderr, "You must specify a snapshot ID.")
			os.Exit(2)
		}
		snapshotID := args[0]
		fmt.Printf("Are you sure you want to delete snapshot %s? (y/N): ", snapshotID)
		var response string
		fmt.Scanln(&response)
		if strings.ToLower(response) != "y" {
			fmt.Println("Operation cancelled.")
			return
		}
		logger.Info("Deleting snapshot", "snapshot", snapshotID)
		if err := km.DeleteSnapshot(snapshotID, false); err != nil {
			logger.Fatal("Delete failed", "error", err)
		}
		logger.Info("Snapshot deleted successfully")
	},
}

// Info command
var infoCmd = &cobra.Command{
	Use:   "info [snapshot-id]",
	Short: "Show detailed information about a snapshot",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()
		snapshotID := args[0]

		info, err := km.GetSnapshotInfo(snapshotID)
		if err != nil {
			logger.Fatal("Failed to get snapshot info", "error", err)
		}

		fmt.Print(info)
	},
}

// Diff command
var diffCmd = &cobra.Command{
	Use:   "diff [snapshot1] [snapshot2]",
	Short: "Compare two snapshots",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()
		snapshot1 := args[0]
		snapshot2 := args[1]

		diff, err := km.DiffSnapshots(snapshot1, snapshot2)
		if err != nil {
			logger.Fatal("Failed to compare snapshots", "error", err)
		}

		fmt.Print(diff)
	},
}

// Mount command
var mountCmd = &cobra.Command{
	Use:   "mount [snapshot-id] [mount-point]",
	Short: "Mount a snapshot as filesystem",
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()
		snapshotID := args[0]
		mountPoint := args[1]

		logger.Info("Mounting snapshot", "snapshot", snapshotID, "mount_point", mountPoint)

		if err := km.MountSnapshot(snapshotID, mountPoint); err != nil {
			logger.Fatal("Mount failed", "error", err)
		}

		logger.Info("Snapshot mounted successfully", "mount_point", mountPoint)
		fmt.Printf("Snapshot mounted at: %s\n", mountPoint)
		fmt.Printf("Use 'kopia-manager unmount %s' to unmount\n", mountPoint)
	},
}

// Unmount command
var unmountCmd = &cobra.Command{
	Use:   "unmount [mount-point]",
	Short: "Unmount a previously mounted snapshot",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()
		mountPoint := args[0]

		logger.Info("Unmounting", "mount_point", mountPoint)

		if err := km.UnmountSnapshot(mountPoint); err != nil {
			logger.Fatal("Unmount failed", "error", err)
		}

		logger.Info("Unmounted successfully")
	},
}

// Policy command
var policyCmd = &cobra.Command{
	Use:   "policy [path]",
	Short: "Show backup policy for a path",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()
		path := args[0]

		policy, err := km.ShowPolicy(path)
		if err != nil {
			logger.Fatal("Failed to get policy", "error", err)
		}

		fmt.Print(policy)
	},
}

// Estimate command
var estimateCmd = &cobra.Command{
	Use:   "estimate [paths...]",
	Short: "Estimate backup size for paths",
	Args:  cobra.MinimumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()
		uploadSpeed, _ := cmd.Flags().GetInt("upload-speed")

		estimate, err := km.EstimateBackupSize(args, uploadSpeed)
		if err != nil {
			logger.Fatal("Failed to estimate backup size", "error", err)
		}

		fmt.Print(estimate)
	},
}

// Maintenance command
var maintenanceCmd = &cobra.Command{
	Use:   "maintenance",
	Short: "Run repository maintenance",
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()

		logger.Info("Starting maintenance")

		if err := km.RunMaintenance(); err != nil {
			logger.Fatal("Maintenance failed", "error", err)
		}

		logger.Info("Maintenance completed successfully")
	},
}

// Verify command
var verifyCmd = &cobra.Command{
	Use:   "verify",
	Short: "Verify repository integrity",
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()

		logger.Info("Starting repository verification")

		if err := km.VerifyRepository(); err != nil {
			logger.Fatal("Verification failed", "error", err)
		}

		logger.Info("Verification completed successfully")
	},
}

// Garbage collection command
var gcCmd = &cobra.Command{
	Use:   "gc",
	Short: "Run garbage collection",
	Run: func(cmd *cobra.Command, args []string) {
		km := NewKopiaManager()

		logger.Info("Starting garbage collection")

		if err := km.RunGarbageCollection(); err != nil {
			logger.Fatal("Garbage collection failed", "error", err)
		}

		logger.Info("Garbage collection completed successfully")
	},
}

// Services command
var servicesCmd = &cobra.Command{
	Use:   "services",
	Short: "Show systemd service status",
	Run: func(cmd *cobra.Command, args []string) {
		execCmd := exec.Command("systemctl", "--user", "list-units", "kopia-*", "--no-legend")
		output, err := execCmd.Output()
		if err != nil {
			logger.Fatal("Failed to get service status", "error", err)
		}

		fmt.Println("Kopia Services:")
		fmt.Println(string(output))

		execCmd = exec.Command("systemctl", "--user", "list-timers", "kopia-*", "--no-legend")
		output, err = execCmd.Output()
		if err != nil {
			logger.Fatal("Failed to get timer status", "error", err)
		}

		fmt.Println("Kopia Timers:")
		fmt.Println(string(output))
	},
}

// Start backup command
var startBackupCmd = &cobra.Command{
	Use:   "start-backup [backup-name]",
	Short: "Start a specific backup service",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		backupName := args[0]
		serviceName := fmt.Sprintf("kopia-backup-%s.service", backupName)

		logger.Info("Starting backup service", "service", serviceName)

		execCmd := exec.Command("systemctl", "--user", "start", serviceName)
		if err := execCmd.Run(); err != nil {
			logger.Fatal("Failed to start service", "error", err)
		}

		logger.Info("Backup service started", "service", serviceName)
	},
}

// Logs command
var logsCmd = &cobra.Command{
	Use:   "logs",
	Short: "Show recent backup logs",
	Run: func(cmd *cobra.Command, args []string) {
		execCmd := exec.Command("journalctl", "--user", "-u", "kopia-*", "--since", "24 hours ago", "--no-pager")
		execCmd.Stdout = os.Stdout
		execCmd.Stderr = os.Stderr
		if err := execCmd.Run(); err != nil {
			logger.Fatal("Failed to get logs", "error", err)
		}
	},
}

// Completion command
var completionCmd = &cobra.Command{
	Use:                   "completion [bash|zsh|fish|powershell]",
	Short:                 "Generate completion script",
	Long:                  "",
	DisableFlagsInUseLine: true,
	ValidArgs:             []string{"bash", "zsh", "fish", "powershell"},
	Args:                  cobra.MatchAll(cobra.ExactArgs(1), cobra.OnlyValidArgs),
	Run: func(cmd *cobra.Command, args []string) {
		switch args[0] {
		case "bash":
			cmd.Root().GenBashCompletion(os.Stdout)
		case "zsh":
			cmd.Root().GenZshCompletion(os.Stdout)
		case "fish":
			cmd.Root().GenFishCompletion(os.Stdout, true)
		}
	},
}

// Initialize commands and flags
func getAvailableBackupNames() []string {
	// Look for systemd user service files matching kopia-backup-*.service
	userSystemdDir := filepath.Join(os.Getenv("HOME"), ".config", "systemd", "user")
	files, err := os.ReadDir(userSystemdDir)
	if err != nil {
		return nil
	}
	var names []string
	for _, f := range files {
		name := f.Name()
		if strings.HasPrefix(name, "kopia-backup-") && strings.HasSuffix(name, ".service") {
			// Extract backup name
			base := strings.TrimSuffix(strings.TrimPrefix(name, "kopia-backup-"), ".service")
			names = append(names, base)
		}
	}
	return names
}

func initCommands() {
	homeDir, _ := os.UserHomeDir()

	rootCmd.PersistentFlags().StringVar(&configFile, "config",
		filepath.Join(homeDir, ".config", "kopia", "repository.config"),
		"Kopia config file path")
	rootCmd.PersistentFlags().StringVar(&passwordFile, "password-file",
		filepath.Join(homeDir, ".config", "kopia", "repository.password"),
		"Kopia password file path")

	// Add custom completions
	backupCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
		return []string{"~", "~/Documents", "~/Downloads", "~/Pictures", "~/Videos"}, cobra.ShellCompDirectiveDefault
	}

	restoreCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
		if len(args) == 0 {
			// Complete snapshot IDs
			km := NewKopiaManager()
			snapshots, err := km.ListSnapshots()
			if err != nil {
				return nil, cobra.ShellCompDirectiveNoFileComp
			}

			var ids []string
			for _, snap := range snapshots {
				ids = append(ids, snap.ID)
			}
			return ids, cobra.ShellCompDirectiveNoFileComp
		} else if len(args) == 1 {
			// Complete directory paths for target
			return nil, cobra.ShellCompDirectiveFilterDirs
		}
		return nil, cobra.ShellCompDirectiveNoFileComp
	}

	deleteCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
		km := NewKopiaManager()
		snapshots, err := km.ListSnapshots()
		if err != nil {
			return nil, cobra.ShellCompDirectiveNoFileComp
		}

		var ids []string
		for _, snap := range snapshots {
			ids = append(ids, snap.ID)
		}
		return ids, cobra.ShellCompDirectiveNoFileComp
	}

	infoCmd.ValidArgsFunction = deleteCmd.ValidArgsFunction

	mountCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
		if len(args) == 0 {
			return deleteCmd.ValidArgsFunction(cmd, args, toComplete)
		}
		return nil, cobra.ShellCompDirectiveFilterDirs
	}

	startBackupCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
		if len(args) == 0 {
			return getAvailableBackupNames(), cobra.ShellCompDirectiveNoFileComp
		}
		return nil, cobra.ShellCompDirectiveNoFileComp
	}

	unmountCmd.ValidArgsFunction = func(cmd *cobra.Command, args []string, toComplete string) ([]string, cobra.ShellCompDirective) {
		return nil, cobra.ShellCompDirectiveFilterDirs
	}

	// Add flags to commands
	estimateCmd.Flags().Int("upload-speed", 1000, "Upload speed in Mbit/s for time estimation")

	// Add all commands to root
	rootCmd.AddCommand(statusCmd)
	rootCmd.AddCommand(listCmd)
	rootCmd.AddCommand(backupCmd)
	rootCmd.AddCommand(restoreCmd)
	deleteCmd.Flags().BoolVar(&deleteAll, "all", false, "Delete all snapshots (requires confirmation)")
	rootCmd.AddCommand(deleteCmd)
	rootCmd.AddCommand(infoCmd)
	rootCmd.AddCommand(diffCmd)
	rootCmd.AddCommand(mountCmd)
	rootCmd.AddCommand(unmountCmd)
	rootCmd.AddCommand(policyCmd)
	rootCmd.AddCommand(estimateCmd)
	rootCmd.AddCommand(maintenanceCmd)
	rootCmd.AddCommand(verifyCmd)
	rootCmd.AddCommand(gcCmd)
	rootCmd.AddCommand(servicesCmd)
	rootCmd.AddCommand(startBackupCmd)
	rootCmd.AddCommand(logsCmd)
	rootCmd.AddCommand(completionCmd)
}
