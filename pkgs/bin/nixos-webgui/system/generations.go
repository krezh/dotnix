package system

import (
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"
)

// Generation represents a NixOS system generation.
type Generation struct {
	Number       int
	Path         string
	Date         time.Time
	Current      bool
	NixOSVersion string
}

// GetGenerations retrieves all NixOS system generations.
func GetGenerations() ([]Generation, error) {
	profilePath := "/nix/var/nix/profiles"

	entries, err := os.ReadDir(profilePath)
	if err != nil {
		return nil, fmt.Errorf("failed to read profiles directory: %w", err)
	}

	var generations []Generation

	// Read current system link - this points to system-XXX-link
	currentGenLink, err := os.Readlink("/nix/var/nix/profiles/system")
	if err != nil {
		return nil, fmt.Errorf("failed to read current system link: %w", err)
	}

	// Resolve to actual store path
	currentGenPath := filepath.Join(profilePath, currentGenLink)
	currentLink, err := os.Readlink(currentGenPath)
	if err != nil {
		return nil, fmt.Errorf("failed to resolve current generation link: %w", err)
	}

	for _, entry := range entries {
		name := entry.Name()

		// Match system-XXX-link pattern
		if !strings.HasPrefix(name, "system-") || !strings.HasSuffix(name, "-link") {
			continue
		}

		// Extract generation number
		numStr := strings.TrimPrefix(name, "system-")
		numStr = strings.TrimSuffix(numStr, "-link")
		genNum, err := strconv.Atoi(numStr)
		if err != nil {
			continue
		}

		linkPath := filepath.Join(profilePath, name)
		target, err := os.Readlink(linkPath)
		if err != nil {
			continue
		}

		// Get modification time
		info, err := entry.Info()
		if err != nil {
			continue
		}

		// Extract NixOS version from path
		version := extractNixOSVersion(target)

		generations = append(generations, Generation{
			Number:       genNum,
			Path:         target,
			Date:         info.ModTime(),
			Current:      target == currentLink,
			NixOSVersion: version,
		})
	}

	// Sort by generation number (descending)
	sort.Slice(generations, func(i, j int) bool {
		return generations[i].Number > generations[j].Number
	})

	return generations, nil
}

// extractNixOSVersion extracts version from store path.
func extractNixOSVersion(path string) string {
	// Path format: /nix/store/...-nixos-system-hostname-VERSION
	parts := strings.Split(filepath.Base(path), "-")

	// Look for version-like pattern
	for i, part := range parts {
		if strings.Contains(part, ".") && i > 2 {
			// Found something like "24.11" or "25.05.20260227"
			return part
		}
	}

	return "unknown"
}

// RollbackToGeneration rolls back to a specific generation.
func RollbackToGeneration(genNum int, sudoPassword string) error {
	// Run: sudo nix-env --profile /nix/var/nix/profiles/system --switch-generation <num>
	cmd := exec.Command("sudo", "-S", "nix-env",
		"--profile", "/nix/var/nix/profiles/system",
		"--switch-generation", strconv.Itoa(genNum))

	// Set up password pipe
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdin pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start rollback: %w", err)
	}

	// Send password
	if sudoPassword != "" {
		_, err = stdin.Write([]byte(sudoPassword + "\n"))
		if err != nil {
			return fmt.Errorf("failed to write password: %w", err)
		}
	}
	stdin.Close()

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("rollback failed: %w", err)
	}

	// Now run: sudo /nix/var/nix/profiles/system/bin/switch-to-configuration switch
	cmd = exec.Command("sudo", "-S", "/nix/var/nix/profiles/system/bin/switch-to-configuration", "switch")

	stdin, err = cmd.StdinPipe()
	if err != nil {
		return fmt.Errorf("failed to create stdin pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start switch-to-configuration: %w", err)
	}

	if sudoPassword != "" {
		_, err = stdin.Write([]byte(sudoPassword + "\n"))
		if err != nil {
			return fmt.Errorf("failed to write password: %w", err)
		}
	}
	stdin.Close()

	if err := cmd.Wait(); err != nil {
		return fmt.Errorf("switch-to-configuration failed: %w", err)
	}

	return nil
}

// DeleteGenerations deletes old generations.
func DeleteGenerations(keepLast int, sudoPassword string) (int, error) {
	// Run: sudo nix-env --profile /nix/var/nix/profiles/system --delete-generations old
	// Or: sudo nix-collect-garbage --delete-older-than Xd

	var args []string
	if keepLast > 0 {
		// Keep last N generations
		args = []string{"-S", "nix-env", "--profile", "/nix/var/nix/profiles/system",
			"--delete-generations", fmt.Sprintf("+%d", keepLast)}
	} else {
		// Delete all old generations
		args = []string{"-S", "nix-env", "--profile", "/nix/var/nix/profiles/system",
			"--delete-generations", "old"}
	}

	cmd := exec.Command("sudo", args...)

	stdin, err := cmd.StdinPipe()
	if err != nil {
		return 0, fmt.Errorf("failed to create stdin pipe: %w", err)
	}

	output, err := cmd.StdoutPipe()
	if err != nil {
		return 0, fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return 0, fmt.Errorf("failed to start delete: %w", err)
	}

	if sudoPassword != "" {
		_, err = stdin.Write([]byte(sudoPassword + "\n"))
		if err != nil {
			return 0, fmt.Errorf("failed to write password: %w", err)
		}
	}
	stdin.Close()

	// Read output to count deletions
	outputBytes, err := io.ReadAll(output)
	if err != nil {
		return 0, fmt.Errorf("failed to read output: %w", err)
	}

	if err := cmd.Wait(); err != nil {
		return 0, fmt.Errorf("delete generations failed: %w", err)
	}

	// Count how many were deleted (output shows "removing generation X")
	deleted := strings.Count(string(outputBytes), "removing generation")

	return deleted, nil
}
