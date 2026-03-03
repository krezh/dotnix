package system

import (
	"bufio"
	"fmt"
	"os/exec"
	"strconv"
	"strings"
)

// StoreStats represents Nix store statistics.
type StoreStats struct {
	TotalSize uint64 // Total size in bytes
	DeadSize  uint64 // Size of unreachable paths (garbage)
	AliveSize uint64 // Size of reachable paths
	PathCount int    // Number of store paths
}

// GetStoreStats retrieves Nix store statistics.
func GetStoreStats() (*StoreStats, error) {
	// Get total store size
	cmd := exec.Command("du", "-sb", "/nix/store")
	output, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("failed to get store size: %w", err)
	}

	// Parse output: "123456\t/nix/store"
	parts := strings.Fields(string(output))
	if len(parts) < 1 {
		return nil, fmt.Errorf("unexpected du output format")
	}

	totalSize, err := strconv.ParseUint(parts[0], 10, 64)
	if err != nil {
		return nil, fmt.Errorf("failed to parse store size: %w", err)
	}

	// Calculate dead paths size using nix-store --gc --print-dead
	deadSize, err := calculateDeadPathsSize()
	if err != nil {
		// If we can't calculate dead paths, just return total size
		return &StoreStats{
			TotalSize: totalSize,
			DeadSize:  0,
			AliveSize: totalSize,
		}, nil
	}

	aliveSize := totalSize - deadSize
	if deadSize > totalSize {
		aliveSize = totalSize
		deadSize = 0
	}

	return &StoreStats{
		TotalSize: totalSize,
		DeadSize:  deadSize,
		AliveSize: aliveSize,
	}, nil
}

// calculateDeadPathsSize calculates the size of dead (unreachable) store paths.
func calculateDeadPathsSize() (uint64, error) {
	// Use nix-store --gc --print-dead to get dead paths
	// Then use du with all paths at once for better performance
	cmd := exec.Command("nix-store", "--gc", "--print-dead")
	output, err := cmd.Output()
	if err != nil {
		return 0, fmt.Errorf("failed to get dead paths: %w", err)
	}

	// Collect all dead paths
	var deadPaths []string
	scanner := bufio.NewScanner(strings.NewReader(string(output)))

	for scanner.Scan() {
		path := strings.TrimSpace(scanner.Text())
		if path != "" && strings.HasPrefix(path, "/nix/store/") {
			deadPaths = append(deadPaths, path)
		}
	}

	if len(deadPaths) == 0 {
		return 0, nil
	}

	// Calculate total size using du with all paths at once
	// This is much faster than running du separately for each path
	args := append([]string{"-scb"}, deadPaths...)
	duCmd := exec.Command("du", args...)
	duOutput, err := duCmd.Output()
	if err != nil {
		// If du fails, fall back to calculating individually
		return calculateDeadPathsSizeIndividually(deadPaths)
	}

	// Parse the "total" line (last line of du -sc output)
	lines := strings.Split(strings.TrimSpace(string(duOutput)), "\n")
	if len(lines) == 0 {
		return 0, nil
	}

	// Last line format: "123456\ttotal"
	lastLine := lines[len(lines)-1]
	parts := strings.Fields(lastLine)
	if len(parts) >= 1 {
		size, err := strconv.ParseUint(parts[0], 10, 64)
		if err == nil {
			return size, nil
		}
	}

	return 0, nil
}

// calculateDeadPathsSizeIndividually is a fallback that calculates size path by path.
func calculateDeadPathsSizeIndividually(paths []string) (uint64, error) {
	var totalDeadSize uint64

	for _, path := range paths {
		duCmd := exec.Command("du", "-sb", path)
		duOutput, err := duCmd.Output()
		if err != nil {
			// Path might have been deleted or is inaccessible, skip it
			continue
		}

		parts := strings.Fields(string(duOutput))
		if len(parts) >= 1 {
			size, err := strconv.ParseUint(parts[0], 10, 64)
			if err == nil {
				totalDeadSize += size
			}
		}
	}

	return totalDeadSize, nil
}

// RunGarbageCollection runs nix garbage collection.
func RunGarbageCollection(sudoPassword string, progressChan chan<- UpdateProgress) error {
	defer close(progressChan)

	progressChan <- UpdateProgress{
		Step:    "gc",
		Message: "Running garbage collection...",
	}

	// Run: sudo nix-collect-garbage -d
	cmd := exec.Command("sudo", "-S", "nix-collect-garbage", "-d")

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
		return fmt.Errorf("failed to start gc: %w", err)
	}

	// Send password
	if sudoPassword != "" {
		_, err = stdin.Write([]byte(sudoPassword + "\n"))
		if err != nil {
			return fmt.Errorf("failed to write password: %w", err)
		}
	}
	stdin.Close()

	// Stream output
	go func() {
		scanner := bufio.NewScanner(stdout)
		for scanner.Scan() {
			line := scanner.Text()
			cleanedLine := CleanLogLine(line)
			if cleanedLine != "" {
				progressChan <- UpdateProgress{
					Step:    "gc",
					Message: cleanedLine,
				}
			}
		}
	}()

	go func() {
		scanner := bufio.NewScanner(stderr)
		for scanner.Scan() {
			line := scanner.Text()
			cleanedLine := CleanLogLine(line)
			if cleanedLine != "" {
				progressChan <- UpdateProgress{
					Step:    "gc",
					Message: cleanedLine,
				}
			}
		}
	}()

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("garbage collection failed: %w", err)
	}

	progressChan <- UpdateProgress{
		Step:    "gc",
		Message: "Garbage collection completed successfully",
		Done:    true,
	}

	return nil
}

// OptimizeStore runs nix-store --optimise to deduplicate files.
func OptimizeStore(sudoPassword string, progressChan chan<- UpdateProgress) error {
	defer close(progressChan)

	progressChan <- UpdateProgress{
		Step:    "optimize",
		Message: "Optimizing store (deduplicating files)...",
	}

	// Run: sudo nix-store --optimise
	cmd := exec.Command("sudo", "-S", "nix-store", "--optimise")

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
		return fmt.Errorf("failed to start optimize: %w", err)
	}

	// Send password
	if sudoPassword != "" {
		_, err = stdin.Write([]byte(sudoPassword + "\n"))
		if err != nil {
			return fmt.Errorf("failed to write password: %w", err)
		}
	}
	stdin.Close()

	// Stream output
	go func() {
		scanner := bufio.NewScanner(stdout)
		for scanner.Scan() {
			line := scanner.Text()
			cleanedLine := CleanLogLine(line)
			if cleanedLine != "" {
				progressChan <- UpdateProgress{
					Step:    "optimize",
					Message: cleanedLine,
				}
			}
		}
	}()

	go func() {
		scanner := bufio.NewScanner(stderr)
		for scanner.Scan() {
			line := scanner.Text()
			cleanedLine := CleanLogLine(line)
			if cleanedLine != "" {
				progressChan <- UpdateProgress{
					Step:    "optimize",
					Message: cleanedLine,
				}
			}
		}
	}()

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("store optimization failed: %w", err)
	}

	progressChan <- UpdateProgress{
		Step:    "optimize",
		Message: "Store optimization completed successfully",
		Done:    true,
	}

	return nil
}
