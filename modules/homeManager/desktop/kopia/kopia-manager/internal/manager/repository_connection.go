package manager

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/kopia/kopia/repo"
)

// NewKopiaManager creates a new KopiaManager instance
func NewKopiaManager() *KopiaManager {
	homeDir, _ := os.UserHomeDir()
	return &KopiaManager{
		ConfigPath:   filepath.Join(homeDir, ".config", "kopia", "repository.config"),
		PasswordPath: filepath.Join(homeDir, ".config", "kopia", "repository.password"),
	}
}

// openRepository opens a connection to the repository with caching
func (km *KopiaManager) openRepository(ctx context.Context) (repo.Repository, error) {
	km.mu.Lock()
	defer km.mu.Unlock()

	// Return cached connection if available
	if km.cachedRepo != nil {
		return km.cachedRepo, nil
	}

	password, err := os.ReadFile(km.PasswordPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read password file: %w", err)
	}

	r, err := repo.Open(ctx, km.ConfigPath, strings.TrimSpace(string(password)), &repo.Options{})
	if err != nil {
		return nil, fmt.Errorf("failed to open repository: %w", err)
	}

	// Cache the connection and context
	km.cachedRepo = r
	km.cachedCtx = ctx

	return r, nil
}

// closeRepository closes the cached repository connection
func (km *KopiaManager) closeRepository() {
	km.mu.Lock()
	defer km.mu.Unlock()

	if km.cachedRepo != nil && km.cachedCtx != nil {
		km.cachedRepo.Close(km.cachedCtx)
		km.cachedRepo = nil
		km.cachedCtx = nil
	}
}

// Snapshots returns a SnapshotManager for snapshot operations
func (km *KopiaManager) Snapshots() *SnapshotManager {
	return NewSnapshotManager(km)
}

// Repository returns a RepositoryOps for repository operations
func (km *KopiaManager) Repository() *RepositoryOps {
	return NewRepositoryOps(km)
}

// Estimator returns a BackupEstimator for size estimation
func (km *KopiaManager) Estimator() *BackupEstimator {
	return NewBackupEstimator(km)
}

// Mounts returns a MountManager for mount operations
func (km *KopiaManager) Mounts() *MountManager {
	return NewMountManager(km)
}

// Services returns a ServiceManager for systemd service operations
func Services() *ServiceManager {
	return NewServiceManager()
}
