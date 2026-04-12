package system

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"sync"
	"time"
)

// GitStatus holds information about the git repository state.
type GitStatus struct {
	Behind  int      // number of commits on remote not yet pulled
	Commits []string // subject lines of unpulled commits, newest first
}

var (
	gitStatusCache     *GitStatus
	gitStatusCacheTime time.Time
	gitStatusMu        sync.Mutex
	gitStatusTTL       = time.Minute
)

// GetGitStatus returns the cached git status, refreshing if the cache is stale.
func GetGitStatus() (*GitStatus, error) {
	gitStatusMu.Lock()
	defer gitStatusMu.Unlock()

	if gitStatusCache != nil && time.Since(gitStatusCacheTime) < gitStatusTTL {
		return gitStatusCache, nil
	}

	status, err := fetchGitStatus()
	if err != nil {
		return nil, err
	}

	gitStatusCache = status
	gitStatusCacheTime = time.Now()
	return status, nil
}

// RefreshGitStatus forces a fresh fetch from remote, bypassing the cache.
func RefreshGitStatus() (*GitStatus, error) {
	gitStatusMu.Lock()
	defer gitStatusMu.Unlock()

	status, err := fetchGitStatus()
	if err != nil {
		return nil, err
	}

	gitStatusCache = status
	gitStatusCacheTime = time.Now()
	return status, nil
}

// fetchGitStatus runs git fetch and counts unpulled commits. Must be called with gitStatusMu held.
func fetchGitStatus() (*GitStatus, error) {
	configDir := os.Getenv("NH_FLAKE")
	if configDir == "" {
		return nil, fmt.Errorf("NH_FLAKE environment variable is not set")
	}

	// Fetch quietly so the count reflects the real remote state.
	fetchCmd := exec.Command("git", "fetch", "--quiet")
	fetchCmd.Dir = configDir
	// Ignore fetch errors (e.g. no network) — fall through to count.
	fetchCmd.Run()

	// Count commits on remote branch not yet in HEAD.
	countCmd := exec.Command("git", "rev-list", "--count", "HEAD..@{u}")
	countCmd.Dir = configDir
	out, err := countCmd.Output()
	if err != nil {
		// No upstream configured or other error — treat as 0.
		return &GitStatus{Behind: 0}, nil
	}

	behind := 0
	fmt.Sscanf(strings.TrimSpace(string(out)), "%d", &behind)

	// Fetch subject lines for unpulled commits, newest first.
	var commits []string
	if behind > 0 {
		logCmd := exec.Command("git", "log", "--format=%s", "HEAD..@{u}")
		logCmd.Dir = configDir
		if logOut, err := logCmd.Output(); err == nil {
			for _, line := range strings.Split(strings.TrimSpace(string(logOut)), "\n") {
				if line != "" {
					commits = append(commits, line)
				}
			}
		}
	}

	return &GitStatus{Behind: behind, Commits: commits}, nil
}
