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

	deadSize, err := calculateDeadPathsSize()
	if err != nil {
		// If dead paths cannot be calculated, report total as alive.
		return &StoreStats{TotalSize: totalSize, DeadSize: 0, AliveSize: totalSize}, nil
	}

	aliveSize := totalSize - deadSize
	if deadSize > totalSize {
		aliveSize = totalSize
		deadSize = 0
	}

	return &StoreStats{TotalSize: totalSize, DeadSize: deadSize, AliveSize: aliveSize}, nil
}

// calculateDeadPathsSize calculates the size of dead (unreachable) store paths.
func calculateDeadPathsSize() (uint64, error) {
	cmd := exec.Command("nix-store", "--gc", "--print-dead")
	output, err := cmd.Output()
	if err != nil {
		return 0, fmt.Errorf("failed to get dead paths: %w", err)
	}

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

	// Calculate total size using du with all paths at once for performance.
	args := append([]string{"-scb"}, deadPaths...)
	duCmd := exec.Command("du", args...)
	duOutput, err := duCmd.Output()
	if err != nil {
		return calculateDeadPathsSizeIndividually(deadPaths)
	}

	// Last line format: "123456\ttotal"
	lines := strings.Split(strings.TrimSpace(string(duOutput)), "\n")
	if len(lines) == 0 {
		return 0, nil
	}
	parts := strings.Fields(lines[len(lines)-1])
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

	progressChan <- UpdateProgress{Step: "gc", Message: "Running garbage collection..."}

	cmd := exec.Command("sudo", "-S", "nix-collect-garbage", "-d")
	if err := streamCommandOutputWithSudo(cmd, sudoPassword, progressChan, "gc"); err != nil {
		return fmt.Errorf("garbage collection failed: %w", err)
	}

	progressChan <- UpdateProgress{Step: "gc", Message: "Garbage collection completed successfully", Done: true}
	return nil
}

// OptimizeStore runs nix-store --optimise to deduplicate files.
func OptimizeStore(sudoPassword string, progressChan chan<- UpdateProgress) error {
	defer close(progressChan)

	progressChan <- UpdateProgress{Step: "optimize", Message: "Optimizing store (deduplicating files)..."}

	cmd := exec.Command("sudo", "-S", "nix-store", "--optimise")
	if err := streamCommandOutputWithSudo(cmd, sudoPassword, progressChan, "optimize"); err != nil {
		return fmt.Errorf("store optimization failed: %w", err)
	}

	progressChan <- UpdateProgress{Step: "optimize", Message: "Store optimization completed successfully", Done: true}
	return nil
}
