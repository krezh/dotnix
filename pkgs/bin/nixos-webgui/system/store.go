package system

import (
	"bufio"
	"bytes"
	"fmt"
	"os/exec"
	"strconv"
	"strings"
	"sync"
)

// StoreStats represents Nix store statistics.
type StoreStats struct {
	TotalSize uint64 // Total size in bytes
	DeadSize  uint64 // Size of unreachable paths (garbage)
	AliveSize uint64 // Size of reachable paths
	PathCount int    // Number of store paths
}

// GetStoreStats retrieves Nix store statistics.
//
// Runs du -sb /nix/store and nix-store --gc --print-dead in parallel, then
// measures dead-path disk usage with a single du -scb call.
func GetStoreStats() (*StoreStats, error) {
	var (
		wg        sync.WaitGroup
		totalSize uint64
		totalErr  error
		deadPaths []string
		deadErr   error
	)

	wg.Add(2)

	go func() {
		defer wg.Done()
		totalSize, totalErr = fetchStoreTotalSize()
	}()

	go func() {
		defer wg.Done()
		deadPaths, deadErr = fetchDeadPaths()
	}()

	wg.Wait()

	if totalErr != nil {
		return nil, fmt.Errorf("failed to get store size: %w", totalErr)
	}

	if deadErr != nil {
		return &StoreStats{TotalSize: totalSize, DeadSize: 0, AliveSize: totalSize}, nil
	}

	deadSize, err := measureDeadSize(deadPaths)
	if err != nil {
		return &StoreStats{TotalSize: totalSize, DeadSize: 0, AliveSize: totalSize}, nil
	}

	aliveSize := totalSize - deadSize
	if deadSize > totalSize {
		aliveSize = totalSize
		deadSize = 0
	}

	return &StoreStats{TotalSize: totalSize, DeadSize: deadSize, AliveSize: aliveSize}, nil
}

// fetchStoreTotalSize returns the on-disk size of /nix/store in bytes.
func fetchStoreTotalSize() (uint64, error) {
	out, err := exec.Command("du", "-sb", "/nix/store").Output()
	if err != nil {
		return 0, err
	}
	parts := strings.Fields(string(out))
	if len(parts) < 1 {
		return 0, fmt.Errorf("unexpected du output")
	}
	return strconv.ParseUint(parts[0], 10, 64)
}

// fetchDeadPaths returns the list of unreachable store paths.
func fetchDeadPaths() ([]string, error) {
	out, err := exec.Command("nix-store", "--gc", "--print-dead").Output()
	if err != nil {
		return nil, fmt.Errorf("nix-store --gc --print-dead: %w", err)
	}

	var paths []string
	scanner := bufio.NewScanner(bytes.NewReader(out))
	for scanner.Scan() {
		path := strings.TrimSpace(scanner.Text())
		if strings.HasPrefix(path, "/nix/store/") {
			paths = append(paths, path)
		}
	}
	return paths, scanner.Err()
}

// measureDeadSize returns the total on-disk size of the given paths using du.
func measureDeadSize(paths []string) (uint64, error) {
	if len(paths) == 0 {
		return 0, nil
	}

	args := append([]string{"-scb"}, paths...)
	out, err := exec.Command("du", args...).Output()
	if err != nil {
		return 0, fmt.Errorf("du on dead paths: %w", err)
	}

	// Last line is the grand total: "123456\ttotal"
	lines := strings.Split(strings.TrimSpace(string(out)), "\n")
	parts := strings.Fields(lines[len(lines)-1])
	if len(parts) < 1 {
		return 0, fmt.Errorf("unexpected du total output")
	}
	return strconv.ParseUint(parts[0], 10, 64)
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
