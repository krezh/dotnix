package system

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
)

// UpdateProgress represents the progress of a NixOS update.
type UpdateProgress struct {
	Step    string
	Message string
	Done    bool
	Error   error
}

// getConfigDir returns the config directory from NH_FLAKE, or sends an error
// progress and returns an error if the variable is unset.
func getConfigDir(progressChan chan<- UpdateProgress) (string, error) {
	configDir := os.Getenv("NH_FLAKE")
	if configDir == "" {
		err := fmt.Errorf("NH_FLAKE environment variable is not set")
		progressChan <- UpdateProgress{Step: "error", Error: err}
		return "", err
	}
	return configDir, nil
}

// shortHostname returns the local hostname without any domain suffix.
func shortHostname() (string, error) {
	hostname, err := os.Hostname()
	if err != nil {
		return "", fmt.Errorf("failed to get hostname: %w", err)
	}
	if idx := strings.Index(hostname, "."); idx != -1 {
		hostname = hostname[:idx]
	}
	return hostname, nil
}

// BuildDiff builds the configuration and shows the diff without applying.
func BuildDiff(progressChan chan<- UpdateProgress) error {
	defer close(progressChan)

	configDir, err := getConfigDir(progressChan)
	if err != nil {
		return err
	}

	// Step 1: Git pull
	progressChan <- UpdateProgress{Step: "git-pull", Message: "Pulling latest changes from git repository..."}
	if err := gitPull(configDir, progressChan); err != nil {
		progressChan <- UpdateProgress{Step: "git-pull", Error: err}
		return err
	}
	progressChan <- UpdateProgress{Step: "git-pull", Message: "Git pull completed successfully", Done: true}

	// Step 2: Build configuration
	progressChan <- UpdateProgress{Step: "nixos-build", Message: "Building NixOS configuration..."}
	if err := nixosRebuildBuild(configDir, progressChan); err != nil {
		progressChan <- UpdateProgress{Step: "nixos-build", Error: err}
		return err
	}
	progressChan <- UpdateProgress{Step: "nixos-build", Message: "Build completed successfully", Done: true}

	// Step 3: Show diff
	progressChan <- UpdateProgress{Step: "diff", Message: "Comparing configurations..."}
	if err := showDiff(configDir, progressChan); err != nil {
		progressChan <- UpdateProgress{Step: "diff", Error: err}
		return err
	}
	progressChan <- UpdateProgress{Step: "diff", Message: "Diff complete - review changes above", Done: true}

	return nil
}

// ApplyUpdate applies the already-built configuration.
func ApplyUpdate(progressChan chan<- UpdateProgress, sudoPassword string) error {
	defer close(progressChan)

	configDir, err := getConfigDir(progressChan)
	if err != nil {
		return err
	}

	progressChan <- UpdateProgress{Step: "nixos-switch", Message: "Applying configuration..."}
	if err := nixosRebuildSwitch(configDir, progressChan, sudoPassword); err != nil {
		progressChan <- UpdateProgress{Step: "nixos-switch", Error: err}
		return err
	}
	progressChan <- UpdateProgress{Step: "nixos-switch", Message: "System update completed successfully", Done: true}

	return nil
}

// gitPull executes git pull in the specified directory and streams output.
func gitPull(dir string, progressChan chan<- UpdateProgress) error {
	cmd := exec.Command("git", "pull")
	cmd.Dir = dir
	if err := streamCommandOutput(cmd, progressChan, "git-pull"); err != nil {
		return fmt.Errorf("git pull failed: %w", err)
	}
	return nil
}

// nixosRebuildBuild executes nixos-rebuild build and streams output.
func nixosRebuildBuild(dir string, progressChan chan<- UpdateProgress) error {
	hostname, err := shortHostname()
	if err != nil {
		return err
	}

	flakeRef := fmt.Sprintf("%s/#%s", dir, hostname)
	cmd := exec.Command("nixos-rebuild", "build", "--flake", flakeRef)
	cmd.Dir = dir

	if err := streamCommandOutput(cmd, progressChan, "nixos-build"); err != nil {
		return fmt.Errorf("nixos-rebuild build failed: %w", err)
	}
	return nil
}

// showDiff runs dix to show configuration changes.
func showDiff(dir string, progressChan chan<- UpdateProgress) error {
	cmd := exec.Command("dix", "/run/current-system", "result")
	cmd.Dir = dir

	err := streamCommandOutput(cmd, progressChan, "diff")
	if err != nil {
		// dix returns exit code 1 when there are differences — that is expected.
		if exitErr, ok := err.(*exec.ExitError); ok {
			if exitErr.ExitCode() == 1 {
				return nil
			}
		}
		return fmt.Errorf("dix diff failed: %w", err)
	}
	return nil
}

// nixosRebuildSwitch executes nixos-rebuild switch with sudo and streams output.
func nixosRebuildSwitch(dir string, progressChan chan<- UpdateProgress, sudoPassword string) error {
	hostname, err := shortHostname()
	if err != nil {
		return err
	}

	flakeRef := fmt.Sprintf("%s/#%s", dir, hostname)
	cmd := exec.Command("sudo", "-S", "nixos-rebuild", "switch", "--flake", flakeRef, "--sudo")
	cmd.Dir = dir
	cmd.Env = append(os.Environ(), "NIXOS_REBUILD_NO_CONFIRM=1")

	if err := streamCommandOutputWithSudo(cmd, sudoPassword, progressChan, "nixos-rebuild"); err != nil {
		return fmt.Errorf("nixos-rebuild switch failed: %w", err)
	}
	return nil
}

// FlakeUpdate updates flake inputs.
func FlakeUpdate(progressChan chan<- UpdateProgress) error {
	defer close(progressChan)

	configDir, err := getConfigDir(progressChan)
	if err != nil {
		return err
	}

	progressChan <- UpdateProgress{Step: "flake-update", Message: "Updating flake inputs..."}

	cmd := exec.Command("nix", "flake", "update")
	cmd.Dir = configDir

	if err := streamCommandOutput(cmd, progressChan, "flake-update"); err != nil {
		progressChan <- UpdateProgress{Step: "flake-update", Error: err}
		return fmt.Errorf("flake update failed: %w", err)
	}

	progressChan <- UpdateProgress{Step: "flake-update", Message: "Flake update completed successfully", Done: true}
	return nil
}

// RollbackFlakeLock restores flake.lock using git.
func RollbackFlakeLock() error {
	configDir := os.Getenv("NH_FLAKE")
	if configDir == "" {
		return fmt.Errorf("NH_FLAKE environment variable is not set")
	}

	cmd := exec.Command("git", "restore", "flake.lock")
	cmd.Dir = configDir

	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("git restore failed: %w: %s", err, string(output))
	}

	return nil
}
