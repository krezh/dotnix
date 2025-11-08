package manager

import (
	"context"
	"fmt"

	"kopia-manager/internal/ui"

	"github.com/kopia/kopia/repo"
	"github.com/kopia/kopia/repo/maintenance"
	"github.com/kopia/kopia/snapshot"
	"github.com/kopia/kopia/snapshot/policy"
)

// RepositoryOps handles repository maintenance and status operations
type RepositoryOps struct {
	km *KopiaManager
}

// NewRepositoryOps creates a new RepositoryOps
func NewRepositoryOps(km *KopiaManager) *RepositoryOps {
	return &RepositoryOps{km: km}
}

// GetStatus returns the repository status
func (ro *RepositoryOps) GetStatus() (string, error) {
	ctx := context.Background()
	r, err := ro.km.openRepository(ctx)
	if err != nil {
		return "", err
	}
	defer r.Close(ctx)

	clientOpts := r.ClientOptions()

	// Get repository statistics
	sources, err := snapshot.ListSources(ctx, r)
	if err != nil {
		return "", fmt.Errorf("failed to list sources: %w", err)
	}

	// Count total snapshots
	totalSnapshots := 0
	for _, source := range sources {
		snapshots, err := snapshot.ListSnapshots(ctx, r, source)
		if err != nil {
			continue
		}
		totalSnapshots += len(snapshots)
	}

	// Create table with repository status
	headers := []string{"Property", "Value"}
	rows := [][]string{
		{"Config file", ro.km.ConfigPath},
		{"Description", clientOpts.Description},
		{"Hostname", clientOpts.Hostname},
		{"Username", clientOpts.Username},
		{"Read-only", fmt.Sprintf("%v", clientOpts.ReadOnly)},
		{"Sources", fmt.Sprintf("%d", len(sources))},
		{"Snapshots", fmt.Sprintf("%d", totalSnapshots)},
	}

	return ui.RenderTable("Repository Status", headers, rows), nil
}

// RunMaintenance runs repository maintenance
func (ro *RepositoryOps) RunMaintenance(unsafe bool) error {
	ctx := context.Background()
	r, err := ro.km.openRepository(ctx)
	if err != nil {
		return err
	}
	defer r.Close(ctx)

	// Cast to DirectRepository to access maintenance functions
	dr, ok := r.(repo.DirectRepository)
	if !ok {
		return fmt.Errorf("repository does not support direct access")
	}

	// Create and start spinner
	safetyMode := "full"
	if unsafe {
		safetyMode = "unsafe"
	}
	spin := ui.NewSpinner(fmt.Sprintf("Running repository maintenance (%s mode)", safetyMode))
	spin.Start()

	err = maintenance.RunExclusive(ctx, dr.(repo.DirectRepositoryWriter), maintenance.ModeFull, false, func(ctx context.Context, runParams maintenance.RunParameters) error {
		safety := maintenance.SafetyFull
		if unsafe {
			safety = maintenance.SafetyNone
		}

		return maintenance.Run(ctx, runParams, safety)
	})

	if err != nil {
		spin.Fail(fmt.Sprintf("Maintenance failed: %v", err))
		return err
	}

	spin.Success("Repository maintenance completed successfully")
	return nil
}

// VerifyRepository verifies repository integrity
func (ro *RepositoryOps) VerifyRepository() error {
	ctx := context.Background()
	r, err := ro.km.openRepository(ctx)
	if err != nil {
		return err
	}
	defer r.Close(ctx)

	// Create and start spinner
	spin := ui.NewSpinner("Verifying repository integrity")
	spin.Start()

	// Get all sources and verify their snapshots
	sources, err := snapshot.ListSources(ctx, r)
	if err != nil {
		spin.Fail(fmt.Sprintf("Failed to list sources: %v", err))
		return fmt.Errorf("failed to list sources: %w", err)
	}

	var totalSnapshots, verifiedSnapshots int
	var failedSnapshots []string

	for _, source := range sources {
		snapshots, err := snapshot.ListSnapshots(ctx, r, source)
		if err != nil {
			ui.Warningf("Warning: failed to list snapshots for source %s: %v", source, err)
			continue
		}

		for _, snap := range snapshots {
			totalSnapshots++
			spin.UpdateMessage(fmt.Sprintf("Verifying repository integrity (%d/%d)", totalSnapshots, len(snapshots)*len(sources)))

			_, err := r.VerifyObject(ctx, snap.RootObjectID())
			if err != nil {
				failedSnapshots = append(failedSnapshots, string(snap.ID))
			} else {
				verifiedSnapshots++
			}
		}
	}

	spin.Stop()

	// Print any failed snapshots
	if len(failedSnapshots) > 0 {
		ui.Warning("Failed to verify the following snapshots:")
		for _, id := range failedSnapshots {
			ui.Itemf("- %s", id)
		}
	}

	ui.Summaryf("Verification completed: %d/%d snapshots verified successfully", verifiedSnapshots, totalSnapshots)
	return nil
}

// ShowPolicy shows the backup policy for a path
func (ro *RepositoryOps) ShowPolicy(path string) (string, error) {
	ctx := context.Background()
	r, err := ro.km.openRepository(ctx)
	if err != nil {
		return "", err
	}
	defer r.Close(ctx)

	// Get client options to build complete source info
	clientOpts := r.ClientOptions()
	sourceInfo, err := snapshot.ParseSourceInfo(path, clientOpts.Hostname, clientOpts.Username)
	if err != nil {
		return "", fmt.Errorf("failed to parse source info: %w", err)
	}

	// Get detailed policy information including sources
	effective, definition, _, err := policy.GetEffectivePolicy(ctx, r, sourceInfo)
	if err != nil {
		return "", fmt.Errorf("failed to get policy: %w", err)
	}

	// Check if there's a policy specifically defined for this path
	if err != nil && err != policy.ErrPolicyNotFound {
		return "", fmt.Errorf("failed to check for defined policy: %w", err)
	}

	// Helper function to check if a setting is defined for this target
	isDefinedForTarget := func(defSource snapshot.SourceInfo) bool {
		return defSource.Path == sourceInfo.Path && defSource.Host == sourceInfo.Host && defSource.UserName == sourceInfo.UserName
	}

	// Create table
	title := fmt.Sprintf("Policy for %s@%s:%s", sourceInfo.UserName, sourceInfo.Host, sourceInfo.Path)
	headers := []string{"Setting", "Value", "Source"}
	var rows [][]string

	// Compression
	compSource := "Inherited"
	if definition != nil && definition.CompressionPolicy.CompressorName.Path != "" {
		if isDefinedForTarget(definition.CompressionPolicy.CompressorName) {
			compSource = "Defined"
		}
	}
	rows = append(rows, []string{"Compression", string(effective.CompressionPolicy.CompressorName), compSource})

	// Retention policy
	annual := 0
	if effective.RetentionPolicy.KeepAnnual != nil {
		annual = effective.RetentionPolicy.KeepAnnual.OrDefault(0)
	}
	monthly := 0
	if effective.RetentionPolicy.KeepMonthly != nil {
		monthly = effective.RetentionPolicy.KeepMonthly.OrDefault(0)
	}
	weekly := 0
	if effective.RetentionPolicy.KeepWeekly != nil {
		weekly = effective.RetentionPolicy.KeepWeekly.OrDefault(0)
	}
	daily := 0
	if effective.RetentionPolicy.KeepDaily != nil {
		daily = effective.RetentionPolicy.KeepDaily.OrDefault(0)
	}
	hourly := 0
	if effective.RetentionPolicy.KeepHourly != nil {
		hourly = effective.RetentionPolicy.KeepHourly.OrDefault(0)
	}
	latest := 0
	if effective.RetentionPolicy.KeepLatest != nil {
		latest = effective.RetentionPolicy.KeepLatest.OrDefault(0)
	}

	retentionSource := "Inherited"
	if definition != nil && definition.RetentionPolicy.KeepDaily.Path != "" {
		if isDefinedForTarget(definition.RetentionPolicy.KeepDaily) {
			retentionSource = "Defined"
		}
	}
	retentionValue := fmt.Sprintf("%dy/%dm/%dw/%dd/%dh/%dl", annual, monthly, weekly, daily, hourly, latest)
	rows = append(rows, []string{"Retention", retentionValue, retentionSource})

	// Files policy
	if len(effective.FilesPolicy.IgnoreRules) > 0 {
		ignoreSource := "Inherited"
		if definition != nil && definition.FilesPolicy.IgnoreRules.Path != "" {
			if isDefinedForTarget(definition.FilesPolicy.IgnoreRules) {
				ignoreSource = "Defined"
			}
		}
		// Show first rule, then add additional rows for remaining rules
		for i, rule := range effective.FilesPolicy.IgnoreRules {
			setting := "Ignore Rules"
			if i > 0 {
				setting = "" // Empty for continuation rows
			}
			source := ignoreSource
			if i > 0 {
				source = "" // Empty for continuation rows
			}
			rows = append(rows, []string{setting, rule, source})
		}
	}

	return ui.RenderTable(title, headers, rows), nil
}
