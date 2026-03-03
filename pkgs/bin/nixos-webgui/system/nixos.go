package system

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"os/exec"
	"strings"
	"sync"
)

// UpdateProgress represents the progress of a NixOS update.
type UpdateProgress struct {
	Step    string
	Message string
	Done    bool
	Error   error
}

// BuildDiff builds the configuration and shows the diff without applying.
func BuildDiff(progressChan chan<- UpdateProgress) error {
	defer close(progressChan)

	// Get config directory from NH_FLAKE environment variable
	configDir := os.Getenv("NH_FLAKE")
	if configDir == "" {
		progressChan <- UpdateProgress{
			Step:  "error",
			Error: fmt.Errorf("NH_FLAKE environment variable is not set"),
		}
		return fmt.Errorf("NH_FLAKE environment variable is not set")
	}

	// Step 1: Git pull
	progressChan <- UpdateProgress{
		Step:    "git-pull",
		Message: "Pulling latest changes from git repository...",
	}

	if err := gitPull(configDir, progressChan); err != nil {
		progressChan <- UpdateProgress{
			Step:  "git-pull",
			Error: err,
		}
		return err
	}

	progressChan <- UpdateProgress{
		Step:    "git-pull",
		Message: "Git pull completed successfully",
		Done:    true,
	}

	// Step 2: Build configuration
	progressChan <- UpdateProgress{
		Step:    "nixos-build",
		Message: "Building NixOS configuration...",
	}

	if err := nixosRebuildBuild(configDir, progressChan); err != nil {
		progressChan <- UpdateProgress{
			Step:  "nixos-build",
			Error: err,
		}
		return err
	}

	progressChan <- UpdateProgress{
		Step:    "nixos-build",
		Message: "Build completed successfully",
		Done:    true,
	}

	// Step 3: Show diff
	progressChan <- UpdateProgress{
		Step:    "diff",
		Message: "Comparing configurations...",
	}

	if err := showDiff(configDir, progressChan); err != nil {
		progressChan <- UpdateProgress{
			Step:  "diff",
			Error: err,
		}
		return err
	}

	progressChan <- UpdateProgress{
		Step:    "diff",
		Message: "Diff complete - review changes above",
		Done:    true,
	}

	return nil
}

// ApplyUpdate applies the already-built configuration.
func ApplyUpdate(progressChan chan<- UpdateProgress, sudoPassword string) error {
	defer close(progressChan)

	configDir := os.Getenv("NH_FLAKE")
	if configDir == "" {
		progressChan <- UpdateProgress{
			Step:  "error",
			Error: fmt.Errorf("NH_FLAKE environment variable is not set"),
		}
		return fmt.Errorf("NH_FLAKE environment variable is not set")
	}

	progressChan <- UpdateProgress{
		Step:    "nixos-switch",
		Message: "Applying configuration...",
	}

	if err := nixosRebuildSwitch(configDir, progressChan, sudoPassword); err != nil {
		progressChan <- UpdateProgress{
			Step:  "nixos-switch",
			Error: err,
		}
		return err
	}

	progressChan <- UpdateProgress{
		Step:    "nixos-switch",
		Message: "System update completed successfully",
		Done:    true,
	}

	return nil
}

// gitPull executes git pull in the specified directory and streams output.
func gitPull(dir string, progressChan chan<- UpdateProgress) error {
	cmd := exec.Command("git", "pull")
	cmd.Dir = dir

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	stderr, err := cmd.StderrPipe()
	if err != nil {
		return fmt.Errorf("failed to create stderr pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start git pull: %w", err)
	}

	// Stream output
	var wg sync.WaitGroup
	wg.Add(2)

	go streamOutput(stdout, progressChan, "git-pull", &wg)
	go streamOutput(stderr, progressChan, "git-pull", &wg)

	wg.Wait()

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("git pull failed: %w", err)
	}

	return nil
}

// nixosRebuildBuild executes nixos-rebuild build and streams output.
func nixosRebuildBuild(dir string, progressChan chan<- UpdateProgress) error {
	// Get hostname for flake reference
	hostname, err := os.Hostname()
	if err != nil {
		return fmt.Errorf("failed to get hostname: %w", err)
	}

	// Remove domain suffix if present
	if idx := strings.Index(hostname, "."); idx != -1 {
		hostname = hostname[:idx]
	}

	flakeRef := fmt.Sprintf("%s/#%s", dir, hostname)

	cmd := exec.Command("nixos-rebuild", "build", "--flake", flakeRef)
	cmd.Dir = dir

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	stderr, err := cmd.StderrPipe()
	if err != nil {
		return fmt.Errorf("failed to create stderr pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start nixos-rebuild build: %w", err)
	}

	// Stream output
	var wg sync.WaitGroup
	wg.Add(2)

	go streamOutput(stdout, progressChan, "nixos-build", &wg)
	go streamOutput(stderr, progressChan, "nixos-build", &wg)

	wg.Wait()

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("nixos-rebuild build failed: %w", err)
	}

	return nil
}

// showDiff runs dix to show configuration changes.
func showDiff(dir string, progressChan chan<- UpdateProgress) error {
	cmd := exec.Command("dix", "/run/current-system", "result")
	cmd.Dir = dir

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	stderr, err := cmd.StderrPipe()
	if err != nil {
		return fmt.Errorf("failed to create stderr pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start dix diff: %w", err)
	}

	// Stream output
	var wg sync.WaitGroup
	wg.Add(2)

	go streamOutput(stdout, progressChan, "diff", &wg)
	go streamOutput(stderr, progressChan, "diff", &wg)

	wg.Wait()

	if err := cmd.Wait(); err != nil {
		// dix diff returns non-zero when there are differences, which is expected
		// Only return error if it's not exit code 1
		if exitErr, ok := err.(*exec.ExitError); ok {
			if exitErr.ExitCode() != 1 {
				return fmt.Errorf("dix diff failed: %w", err)
			}
		} else {
			return fmt.Errorf("dix diff failed: %w", err)
		}
	}

	return nil
}

// nixosRebuildSwitch executes nixos-rebuild switch and streams output.
func nixosRebuildSwitch(dir string, progressChan chan<- UpdateProgress, sudoPassword string) error {
	// Get hostname for flake reference
	hostname, err := os.Hostname()
	if err != nil {
		return fmt.Errorf("failed to get hostname: %w", err)
	}

	// Remove domain suffix if present (e.g., "thor.local" -> "thor")
	if idx := strings.Index(hostname, "."); idx != -1 {
		hostname = hostname[:idx]
	}

	flakeRef := fmt.Sprintf("%s/#%s", dir, hostname)

	// Use sudo -S to read password from stdin
	cmd := exec.Command("sudo", "-S", "nixos-rebuild", "switch", "--flake", flakeRef, "--sudo")
	cmd.Dir = dir

	// Set environment to avoid interactive prompts
	cmd.Env = append(os.Environ(), "NIXOS_REBUILD_NO_CONFIRM=1")

	// Get stdin to send password
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdin pipe: %w", err)
	}

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	stderr, err := cmd.StderrPipe()
	if err != nil {
		return fmt.Errorf("failed to create stderr pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start nixos-rebuild switch: %w", err)
	}

	// Send password to sudo
	if sudoPassword != "" {
		_, err = stdin.Write([]byte(sudoPassword + "\n"))
		if err != nil {
			return fmt.Errorf("failed to write password: %w", err)
		}
	}
	stdin.Close()

	// Stream output
	var wg sync.WaitGroup
	wg.Add(2)

	go streamOutput(stdout, progressChan, "nixos-rebuild", &wg)
	go streamOutput(stderr, progressChan, "nixos-rebuild", &wg)

	wg.Wait()

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("nixos-rebuild switch failed: %w", err)
	}

	return nil
}

// streamOutput reads from a reader and sends each line to the progress channel.
func streamOutput(reader io.Reader, progressChan chan<- UpdateProgress, step string, wg *sync.WaitGroup) {
	defer wg.Done()

	scanner := bufio.NewScanner(reader)
	for scanner.Scan() {
		line := scanner.Text()
		progressChan <- UpdateProgress{
			Step:    step,
			Message: line,
		}
	}
}

// FlakeUpdate updates flake inputs.
func FlakeUpdate(progressChan chan<- UpdateProgress) error {
	defer close(progressChan)

	configDir := os.Getenv("NH_FLAKE")
	if configDir == "" {
		progressChan <- UpdateProgress{
			Step:  "error",
			Error: fmt.Errorf("NH_FLAKE environment variable is not set"),
		}
		return fmt.Errorf("NH_FLAKE environment variable is not set")
	}

	progressChan <- UpdateProgress{
		Step:    "flake-update",
		Message: "Updating flake inputs...",
	}

	cmd := exec.Command("nix", "flake", "update")
	cmd.Dir = configDir

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	stderr, err := cmd.StderrPipe()
	if err != nil {
		return fmt.Errorf("failed to create stderr pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		progressChan <- UpdateProgress{
			Step:  "flake-update",
			Error: err,
		}
		return fmt.Errorf("failed to start flake update: %w", err)
	}

	var wg sync.WaitGroup
	wg.Add(2)

	go streamOutput(stdout, progressChan, "flake-update", &wg)
	go streamOutput(stderr, progressChan, "flake-update", &wg)

	wg.Wait()

	if err := cmd.Wait(); err != nil {
		progressChan <- UpdateProgress{
			Step:  "flake-update",
			Error: err,
		}
		return fmt.Errorf("flake update failed: %w", err)
	}

	progressChan <- UpdateProgress{
		Step:    "flake-update",
		Message: "Flake update completed successfully",
		Done:    true,
	}

	return nil
}

// RollbackFlakeLock restores flake.lock using git.
func RollbackFlakeLock() error {
	configDir := os.Getenv("NH_FLAKE")
	if configDir == "" {
		return fmt.Errorf("NH_FLAKE environment variable is not set")
	}

	// Use git to restore flake.lock
	cmd := exec.Command("git", "restore", "flake.lock")
	cmd.Dir = configDir

	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("git restore failed: %w: %s", err, string(output))
	}

	return nil
}
