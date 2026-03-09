package manager

// This file provides backward-compatible wrapper methods for existing code during migration to specialized managers.

func (km *KopiaManager) ListSnapshots(hostname, username string) ([]SnapshotSummary, error) {
	return km.Snapshots().ListSnapshots(hostname, username)
}

func (km *KopiaManager) GetSnapshotInfo(snapshotID string) (string, error) {
	return km.Snapshots().GetSnapshotInfo(snapshotID)
}

func (km *KopiaManager) CreateBackup(paths []string, description string) error {
	return km.Snapshots().CreateBackup(paths, description)
}

func (km *KopiaManager) RestoreSnapshot(snapshotID, targetDir string) error {
	return km.Snapshots().RestoreSnapshot(snapshotID, targetDir)
}

func (km *KopiaManager) DeleteSnapshot(snapshotID string, allFlag bool, hostname, username string) error {
	return km.Snapshots().DeleteSnapshot(snapshotID, allFlag, hostname, username)
}

func (km *KopiaManager) DeleteBackupGroup(backupName string) error {
	return km.Snapshots().DeleteBackupGroup(backupName)
}

func (km *KopiaManager) DiffSnapshots(snapshot1, snapshot2 string) (string, error) {
	return km.Snapshots().DiffSnapshots(snapshot1, snapshot2)
}

func (km *KopiaManager) RestoreBackupGroup(backupName, targetDir string) error {
	return km.Snapshots().RestoreBackupGroup(backupName, targetDir)
}

func (km *KopiaManager) GetStatus() (string, error) {
	return km.Repository().GetStatus()
}

func (km *KopiaManager) RunMaintenance(unsafe bool) error {
	return km.Repository().RunMaintenance(unsafe)
}

func (km *KopiaManager) VerifyRepository() error {
	return km.Repository().VerifyRepository()
}

func (km *KopiaManager) ShowPolicy(path string) (string, error) {
	return km.Repository().ShowPolicy(path)
}

func (km *KopiaManager) EstimateBackupSize(paths []string, uploadSpeed int) (string, error) {
	return km.Estimator().EstimateBackupSize(paths, uploadSpeed)
}

func (km *KopiaManager) MountSnapshot(snapshotID, mountPoint string) error {
	return km.Mounts().MountSnapshot(snapshotID, mountPoint)
}

func (km *KopiaManager) UnmountSnapshot(mountPoint string) error {
	return km.Mounts().UnmountSnapshot(mountPoint)
}

func (km *KopiaManager) GetServicesStatus() (string, error) {
	return Services().GetServicesStatus()
}

func (km *KopiaManager) GetServiceLogs(lines int, follow bool) (string, error) {
	return Services().GetServiceLogs(lines, follow)
}

func (km *KopiaManager) TriggerBackupService(backupName string) error {
	return Services().TriggerBackupService(backupName)
}

func (km *KopiaManager) ListBackupServices() ([]string, error) {
	return Services().ListBackupServices()
}
