package main

import (
	"flag"
	"fmt"
	"os"
	"sort"
	"strings"
	"time"

	"github.com/spf13/cobra"
	"k8s.io/klog/v2"

	"klim/internal/analyzer"
	"klim/internal/config"
	"klim/internal/kubernetes"
	"klim/internal/manifests"
	"klim/internal/output"
	"klim/internal/progress"
	"klim/internal/recommendations"
	"klim/internal/vpa"
	"klim/pkg/types"
)

var (
	version = "dev"
	cfg     = config.DefaultConfig()
)

// durationValue is a custom flag type that supports weeks and days.
type durationValue struct {
	target *time.Duration
}

func (d *durationValue) Set(s string) error {
	parsed, err := config.ParseDuration(s)
	if err != nil {
		return err
	}
	*d.target = parsed
	return nil
}

func (d *durationValue) String() string {
	if d.target == nil {
		return ""
	}
	return d.target.String()
}

func (d *durationValue) Type() string {
	return "duration"
}

func main() {
	// Suppress Kubernetes client-go info logs (only show warnings and errors)
	klog.InitFlags(nil)
	flag.Set("v", "0")
	flag.Set("logtostderr", "false")
	flag.Parse()

	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

var rootCmd = &cobra.Command{
	Use:   "klim",
	Short: "Kubernetes Resource Recommender",
	Long: `Klim analyzes Kubernetes cluster resource usage and provides optimization recommendations.

It queries VPA (Vertical Pod Autoscaler) for calculated recommendations and can apply them to
bjw-s HelmRelease manifests.

Prerequisites:
- VPA must be installed in the cluster
- VPAs must be configured for workloads you want to analyze`,
	Version: version,
}

var simpleCmd = &cobra.Command{
	Use:   "simple",
	Short: "Run a simple analysis",
	Long:  `Analyzes resource usage and generates recommendations for Kubernetes workloads.`,
	RunE:  runSimple,
}

var applyCmd = &cobra.Command{
	Use:   "apply",
	Short: "Apply VPA recommendations to HelmRelease manifests",
	Long: `Fetches VPA recommendations and updates bjw-s HelmRelease manifests.
Shows a diff and requires confirmation before applying changes.`,
	RunE: runApply,
}

func addCommonFlags(cmd *cobra.Command) {
	cmd.Flags().StringSliceVarP(&cfg.Namespaces, "namespace", "n", []string{}, "Namespaces to analyze (all if not specified)")
	cmd.Flags().StringVarP(&cfg.LabelSelector, "selector", "l", "", "Label selector to filter pods")
	cmd.Flags().Float64Var(&cfg.MinMemory, "mem-min", 10.0, "Minimum memory recommendation in M")
	cmd.Flags().BoolVarP(&cfg.Verbose, "verbose", "v", false, "Verbose output")
}

func init() {
	rootCmd.AddCommand(simpleCmd)
	rootCmd.AddCommand(applyCmd)

	// Simple command flags
	addCommonFlags(simpleCmd)
	simpleCmd.Flags().StringSliceVarP(&cfg.Contexts, "context", "c", []string{}, "Kubernetes contexts to analyze (current if not specified)")
	simpleCmd.Flags().StringVarP(&cfg.OutputFormat, "format", "f", "table", "Output format (table, json, yaml, csv, html)")
	simpleCmd.Flags().StringVar(&cfg.OutputFile, "fileoutput", "", "Output file (stdout if not specified)")
	simpleCmd.Flags().IntVar(&cfg.Concurrency, "concurrency", 10, "Number of concurrent pod analyses")

	// Apply command flags
	addCommonFlags(applyCmd)
	applyCmd.Flags().IntVar(&cfg.Concurrency, "concurrency", 8, "Number of concurrent pod analyses")
	applyCmd.Flags().StringVar(&cfg.GitRepoPath, "git-repo", "", "Path to git repository containing manifests (required)")
	applyCmd.MarkFlagRequired("git-repo")
}

func setupAndAnalyze(ctx string) ([]types.Recommendation, error) {
	// Create Kubernetes client
	k8sClient, err := kubernetes.NewClient(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to create kubernetes client: %w", err)
	}

	// Create VPA client
	vpaClient, err := vpa.NewClient(k8sClient.Clientset(), k8sClient.Config())
	if err != nil {
		return nil, fmt.Errorf("failed to create VPA client: %w", err)
	}
	vpaClient.SetVerbose(cfg.Verbose)

	// Create recommendation engine
	engine := recommendations.NewEngine(cfg.MinMemory)

	// Create analyzer
	an := analyzer.NewAnalyzer(k8sClient, vpaClient, engine, cfg)

	// Get pod count for progress tracker
	pods, err := k8sClient.GetPods(cfg.Namespaces, cfg.LabelSelector)
	if err != nil {
		return nil, fmt.Errorf("failed to get pods: %w", err)
	}

	// Create and start progress tracker
	tracker := progress.NewTracker(len(pods), cfg.Concurrency, cfg.Verbose)
	an.SetProgressTracker(tracker)

	if !cfg.Verbose {
		tracker.Start()
	}

	// Run analysis
	recs, err := an.Analyze()

	// Stop tracker
	if !cfg.Verbose {
		tracker.Stop()
		fmt.Println(tracker.Summary())
	}

	if err != nil {
		return nil, fmt.Errorf("analysis failed: %w", err)
	}

	return recs, nil
}

func runSimple(cmd *cobra.Command, args []string) error {
	if cfg.Verbose {
		fmt.Printf("Klim version %s\n", version)
		fmt.Println("Starting analysis...")
	}

	if err := config.Validate(cfg); err != nil {
		return fmt.Errorf("invalid configuration: %w", err)
	}

	contexts := cfg.Contexts
	if len(contexts) == 0 {
		contexts = []string{""}
	}

	allRecommendations := make([]types.Recommendation, 0)

	for _, ctx := range contexts {
		if cfg.Verbose && ctx != "" {
			fmt.Printf("\nAnalyzing context: %s\n", ctx)
		}

		recs, err := setupAndAnalyze(ctx)
		if err != nil {
			return err
		}

		allRecommendations = append(allRecommendations, recs...)
	}

	if len(allRecommendations) == 0 {
		fmt.Println("No recommendations generated. This might mean:")
		fmt.Println("  - No VPAs found in the specified namespaces")
		fmt.Println("  - No workloads found for existing VPAs")
		fmt.Println("  - Label selector filtered out all workloads")
		return nil
	}

	// Sort recommendations by namespace > workload > container
	sort.Slice(allRecommendations, func(i, j int) bool {
		if allRecommendations[i].Namespace != allRecommendations[j].Namespace {
			return allRecommendations[i].Namespace < allRecommendations[j].Namespace
		}
		workloadI := allRecommendations[i].WorkloadKind + "/" + allRecommendations[i].WorkloadName
		workloadJ := allRecommendations[j].WorkloadKind + "/" + allRecommendations[j].WorkloadName
		if workloadI != workloadJ {
			return workloadI < workloadJ
		}
		return allRecommendations[i].Container < allRecommendations[j].Container
	})

	formatter := output.NewFormatter(cfg.OutputFormat, cfg.OutputFile)
	if err := formatter.Output(allRecommendations); err != nil {
		return fmt.Errorf("failed to output recommendations: %w", err)
	}

	if cfg.OutputFile != "" {
		fmt.Printf("Recommendations written to: %s\n", cfg.OutputFile)
	}

	return nil
}

func runApply(cmd *cobra.Command, args []string) error {
	if cfg.Verbose {
		fmt.Printf("Klim version %s\n", version)
		fmt.Println("Starting analysis for manifest updates...")
	}

	if err := config.Validate(cfg); err != nil {
		return fmt.Errorf("invalid configuration: %w", err)
	}

	if cfg.GitRepoPath == "" {
		return fmt.Errorf("--git-repo is required")
	}

	recs, err := setupAndAnalyze("")
	if err != nil {
		return err
	}

	if !cfg.Verbose {
		fmt.Println()
	}

	if len(recs) == 0 {
		fmt.Println("No recommendations generated.")
		return nil
	}

	// Group recommendations by manifest
	locator := manifests.NewManifestLocator(cfg.GitRepoPath)
	updater := manifests.NewHelmReleaseUpdater()

	manifestRecs := make(map[string][]types.Recommendation)
	skipped := 0

	for _, rec := range recs {
		// Create a temporary pod object to pass labels
		pod := kubernetes.CreatePodFromLabels(rec.Namespace, rec.WorkloadName, rec.PodLabels)
		manifestPath, err := locator.FindHelmRelease(pod)
		if err != nil {
			if cfg.Verbose {
				fmt.Printf("Skipping %s/%s: %v\n", rec.Namespace, rec.WorkloadName, err)
			}
			skipped++
			continue
		}

		rec.ManifestPath = manifestPath
		manifestRecs[manifestPath] = append(manifestRecs[manifestPath], rec)
	}

	if len(manifestRecs) == 0 {
		fmt.Printf("No bjw-s HelmReleases found to update (%d recommendations skipped)\n", skipped)
		return nil
	}

	fmt.Printf("Found %d bjw-s HelmRelease(s) to update (%d skipped)\n\n", len(manifestRecs), skipped)

	// Process each manifest
	for manifestPath, recs := range manifestRecs {
		// Sort recommendations by container name
		sort.Slice(recs, func(i, j int) bool {
			return recs[i].Container < recs[j].Container
		})

		fmt.Printf("=== %s ===\n\n", manifestPath)

		// Read original content
		originalContent, err := os.ReadFile(manifestPath)
		if err != nil {
			fmt.Printf("Error reading manifest: %v\n", err)
			continue
		}

		// Update the manifest
		updatedContent, err := updater.Update(manifestPath, recs)
		if err != nil {
			fmt.Printf("Error updating manifest: %v\n", err)
			continue
		}

		// Show VPA recommendations for each container
		for _, rec := range recs {
			fmt.Printf("Container: %s\n", rec.Container)
			fmt.Printf("  VPA Target: %s\n", recommendations.FormatResourceQuantity(rec.RecommendedMemory))
			fmt.Printf("  Current Limit: %s\n", recommendations.FormatResourceQuantity(rec.CurrentMemory))
			fmt.Printf("  Change: %.1f%%\n\n", rec.MemoryChange)

			// Show request lowering warning
			if rec.RequestLowered {
				fmt.Printf("ℹ️  Request (%s) will be lowered to match recommended limit (%s)\n\n",
					recommendations.FormatResourceQuantity(rec.CurrentRequest),
					recommendations.FormatResourceQuantity(rec.RecommendedMemory))
			}
		}

		// Generate and display diff
		diff := manifests.GenerateDiff(manifestPath, string(originalContent), updatedContent)
		fmt.Println(diff)

		// Ask for confirmation
		fmt.Print("Apply these changes? [y/N]: ")
		var response string
		fmt.Scanln(&response)

		if strings.ToLower(response) == "y" || strings.ToLower(response) == "yes" {
			if err := updater.ApplyChanges(manifestPath, updatedContent); err != nil {
				fmt.Printf("Error applying changes: %v\n", err)
			} else {
				fmt.Println("✓ Changes applied")
			}
		} else {
			fmt.Println("Skipped")
		}
		fmt.Println()
	}

	return nil
}
