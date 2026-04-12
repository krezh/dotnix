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

	currentGenLink, err := os.Readlink("/nix/var/nix/profiles/system")
	if err != nil {
		return nil, fmt.Errorf("failed to read current system link: %w", err)
	}

	currentGenPath := filepath.Join(profilePath, currentGenLink)
	currentLink, err := os.Readlink(currentGenPath)
	if err != nil {
		return nil, fmt.Errorf("failed to resolve current generation link: %w", err)
	}

	var generations []Generation

	for _, entry := range entries {
		name := entry.Name()

		if !strings.HasPrefix(name, "system-") || !strings.HasSuffix(name, "-link") {
			continue
		}

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

		info, err := entry.Info()
		if err != nil {
			continue
		}

		generations = append(generations, Generation{
			Number:       genNum,
			Path:         target,
			Date:         info.ModTime(),
			Current:      target == currentLink,
			NixOSVersion: extractNixOSVersion(target),
		})
	}

	sort.Slice(generations, func(i, j int) bool {
		return generations[i].Number > generations[j].Number
	})

	return generations, nil
}

// extractNixOSVersion extracts version from store path.
func extractNixOSVersion(path string) string {
	parts := strings.Split(filepath.Base(path), "-")
	for i, part := range parts {
		if strings.Contains(part, ".") && i > 2 {
			return part
		}
	}
	return "unknown"
}

// FormatRelativeTime formats a time as a human-readable relative duration.
func FormatRelativeTime(t time.Time) string {
	duration := time.Since(t)

	switch {
	case duration.Hours() < 1:
		return fmt.Sprintf("%.0f minutes ago", duration.Minutes())
	case duration.Hours() < 24:
		return fmt.Sprintf("%.0f hours ago", duration.Hours())
	case duration.Hours() < 24*7:
		return fmt.Sprintf("%d days ago", int(duration.Hours()/24))
	case duration.Hours() < 24*30:
		return fmt.Sprintf("%d weeks ago", int(duration.Hours()/24/7))
	default:
		return fmt.Sprintf("%d months ago", int(duration.Hours()/24/30))
	}
}

// RollbackToGeneration rolls back to a specific generation.
func RollbackToGeneration(genNum int, sudoPassword string) error {
	cmd := exec.Command("sudo", "-S", "nix-env",
		"--profile", "/nix/var/nix/profiles/system",
		"--switch-generation", strconv.Itoa(genNum))
	if err := runWithSudo(cmd, sudoPassword); err != nil {
		return fmt.Errorf("rollback failed: %w", err)
	}

	cmd = exec.Command("sudo", "-S", "/nix/var/nix/profiles/system/bin/switch-to-configuration", "switch")
	if err := runWithSudo(cmd, sudoPassword); err != nil {
		return fmt.Errorf("switch-to-configuration failed: %w", err)
	}

	return nil
}

// DeleteGenerations deletes old generations.
func DeleteGenerations(keepLast int, sudoPassword string) (int, error) {
	var deleteArg string
	if keepLast > 0 {
		deleteArg = fmt.Sprintf("+%d", keepLast)
	} else {
		deleteArg = "old"
	}

	cmd := exec.Command("sudo", "-S", "nix-env",
		"--profile", "/nix/var/nix/profiles/system",
		"--delete-generations", deleteArg)

	stdin, err := cmd.StdinPipe()
	if err != nil {
		return 0, fmt.Errorf("failed to create stdin pipe: %w", err)
	}

	outputPipe, err := cmd.StdoutPipe()
	if err != nil {
		return 0, fmt.Errorf("failed to create stdout pipe: %w", err)
	}

	if err := cmd.Start(); err != nil {
		return 0, fmt.Errorf("failed to start delete: %w", err)
	}

	if sudoPassword != "" {
		if _, err = stdin.Write([]byte(sudoPassword + "\n")); err != nil {
			return 0, fmt.Errorf("failed to write password: %w", err)
		}
	}
	stdin.Close()

	outputBytes, err := io.ReadAll(outputPipe)
	if err != nil {
		return 0, fmt.Errorf("failed to read output: %w", err)
	}

	if err := cmd.Wait(); err != nil {
		return 0, fmt.Errorf("delete generations failed: %w", err)
	}

	deleted := strings.Count(string(outputBytes), "removing generation")
	return deleted, nil
}
