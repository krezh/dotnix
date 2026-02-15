package manager

// This file provides backward-compatible methods that delegate to the new specialized managers
// This allows existing code to continue working while we migrate to the new structure

// ListSnapshots delegates to SnapshotManager
func (km *KopiaManager) ListSnapshots(hostname, username string) ([]SnapshotSummary, error) {
	return km.Snapshots().ListSnapshots(hostname, username)
}

// GetSnapshotInfo delegates to SnapshotManager
func (km *KopiaManager) GetSnapshotInfo(snapshotID string) (string, error) {
	return km.Snapshots().GetSnapshotInfo(snapshotID)
}

// CreateBackup delegates to SnapshotManager
func (km *KopiaManager) CreateBackup(paths []string, description string) error {
	return km.Snapshots().CreateBackup(paths, description)
}

// RestoreSnapshot delegates to SnapshotManager
func (km *KopiaManager) RestoreSnapshot(snapshotID, targetDir string) error {
	return km.Snapshots().RestoreSnapshot(snapshotID, targetDir)
}

// DeleteSnapshot delegates to SnapshotManager
func (km *KopiaManager) DeleteSnapshot(snapshotID string, allFlag bool, hostname, username string) error {
	return km.Snapshots().DeleteSnapshot(snapshotID, allFlag, hostname, username)
}

// DeleteBackupGroup delegates to SnapshotManager
func (km *KopiaManager) DeleteBackupGroup(backupName string) error {
	return km.Snapshots().DeleteBackupGroup(backupName)
}

// DiffSnapshots delegates to SnapshotManager
func (km *KopiaManager) DiffSnapshots(snapshot1, snapshot2 string) (string, error) {
	return km.Snapshots().DiffSnapshots(snapshot1, snapshot2)
}

// RestoreBackupGroup delegates to SnapshotManager
func (km *KopiaManager) RestoreBackupGroup(backupName, targetDir string) error {
	return km.Snapshots().RestoreBackupGroup(backupName, targetDir)
}

// GetStatus delegates to RepositoryOps
func (km *KopiaManager) GetStatus() (string, error) {
	return km.Repository().GetStatus()
}

// RunMaintenance delegates to RepositoryOps
func (km *KopiaManager) RunMaintenance(unsafe bool) error {
	return km.Repository().RunMaintenance(unsafe)
}

// VerifyRepository delegates to RepositoryOps
func (km *KopiaManager) VerifyRepository() error {
	return km.Repository().VerifyRepository()
}

// ShowPolicy delegates to RepositoryOps
func (km *KopiaManager) ShowPolicy(path string) (string, error) {
	return km.Repository().ShowPolicy(path)
}

// EstimateBackupSize delegates to BackupEstimator
func (km *KopiaManager) EstimateBackupSize(paths []string, uploadSpeed int) (string, error) {
	return km.Estimator().EstimateBackupSize(paths, uploadSpeed)
}

// MountSnapshot delegates to MountManager
func (km *KopiaManager) MountSnapshot(snapshotID, mountPoint string) error {
	return km.Mounts().MountSnapshot(snapshotID, mountPoint)
}

// UnmountSnapshot delegates to MountManager
func (km *KopiaManager) UnmountSnapshot(mountPoint string) error {
	return km.Mounts().UnmountSnapshot(mountPoint)
}

// GetServicesStatus delegates to ServiceManager
func (km *KopiaManager) GetServicesStatus() (string, error) {
	return Services().GetServicesStatus()
}

// GetServiceLogs delegates to ServiceManager
func (km *KopiaManager) GetServiceLogs(lines int, follow bool) (string, error) {
	return Services().GetServiceLogs(lines, follow)
}

// TriggerBackupService delegates to ServiceManager
func (km *KopiaManager) TriggerBackupService(backupName string) error {
	return Services().TriggerBackupService(backupName)
}

// ListBackupServices delegates to ServiceManager
func (km *KopiaManager) ListBackupServices() ([]string, error) {
	return Services().ListBackupServices()
}
