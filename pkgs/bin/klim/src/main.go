package main

import (
	"fmt"
	"math"
	"os"
	"strings"
	"time"

	"github.com/spf13/cobra"

	"klim/internal/analyzer"
	"klim/internal/config"
	"klim/internal/graph"
	"klim/internal/kubernetes"
	"klim/internal/manifests"
	"klim/internal/output"
	"klim/internal/progress"
	"klim/internal/prometheus"
	"klim/internal/recommendations"
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
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

var rootCmd = &cobra.Command{
	Use:   "klim",
	Short: "Kubernetes Resource Recommender",
	Long: `Klim analyzes Kubernetes cluster resource usage and provides optimization recommendations.

It queries Prometheus for historical metrics and generates right-sized CPU and memory requests/limits
based on actual usage patterns.`,
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
	Short: "Apply recommendations to HelmRelease manifests",
	Long: `Analyzes resource usage and updates bjw-s HelmRelease manifests with recommendations.
Shows a diff and requires confirmation before applying changes.`,
	RunE: runApply,
}

func addCommonFlags(cmd *cobra.Command) {
	cmd.Flags().StringVarP(&cfg.PrometheusURL, "prometheus", "p", "", "Prometheus URL (auto-discovered if not specified)")
	cmd.Flags().StringSliceVarP(&cfg.Namespaces, "namespace", "n", []string{}, "Namespaces to analyze (all if not specified)")
	cmd.Flags().StringVarP(&cfg.LabelSelector, "selector", "l", "", "Label selector to filter pods")
	cmd.Flags().Var(&durationValue{&cfg.HistoryDuration}, "history-duration", "Historical data duration (e.g., 7d, 2w, 168h, 1w3d) (default 7d)")
	cmd.Flags().Float64Var(&cfg.MemoryBuffer, "memory-buffer", 0.5, "Memory buffer multiplier (0.5 = 50% buffer above peak)")
	cmd.Flags().Float64Var(&cfg.MinMemory, "mem-min", 10.0, "Minimum memory recommendation in Mi")
	cmd.Flags().BoolVarP(&cfg.Verbose, "verbose", "v", false, "Verbose output")
}

func init() {
	rootCmd.AddCommand(simpleCmd)
	rootCmd.AddCommand(applyCmd)

	cfg.HistoryDuration = 7 * 24 * time.Hour

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

	// Discover or use Prometheus endpoint
	prometheusURL := cfg.PrometheusURL
	if prometheusURL == "" {
		if cfg.Verbose {
			fmt.Println("Discovering Prometheus endpoint...")
		}
		discovered, err := prometheus.DiscoverPrometheus(k8sClient.Clientset(), k8sClient.Config())
		if err != nil {
			return nil, fmt.Errorf("failed to discover prometheus: %w (use -p to specify manually)", err)
		}
		prometheusURL = discovered
		if cfg.Verbose {
			fmt.Printf("Discovered Prometheus at: %s\n", prometheusURL)
		}
	}

	// Create Prometheus client
	promClient, err := prometheus.NewClient(prometheusURL, k8sClient.Config())
	if err != nil {
		return nil, fmt.Errorf("failed to create prometheus client: %w", err)
	}

	// Create recommendation engine
	engine := recommendations.NewEngine(cfg.MemoryBuffer, cfg.MinMemory)

	// Create analyzer
	an := analyzer.NewAnalyzer(k8sClient, promClient, engine, cfg)

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
		fmt.Println("  - No running pods found in the specified namespaces")
		fmt.Println("  - No metrics available in Prometheus for the specified time range")
		fmt.Println("  - Label selector filtered out all pods")
		return nil
	}

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

		// Show memory usage graphs for each container
		for _, rec := range recs {
			// Calculate peak for display
			var peak float64
			for _, point := range rec.MemoryHistory {
				valueMiB := point.Value / (1024 * 1024)
				if valueMiB > peak {
					peak = valueMiB
				}
			}

			// Show full graph
			fmt.Printf("Container: %s\n", rec.Container)
			graphOutput := graph.GenerateGraph(
				rec.MemoryHistory,
				rec.RecommendedMemory.Value,
				peak,
				cfg.HistoryDuration.String(),
			)
			fmt.Println(graphOutput)

			// Show request lowering warning
			if rec.RequestLowered {
				fmt.Printf("ℹ️  Request (%dMi) will be lowered to match recommended limit (%dMi)\n\n",
					int64(math.Ceil(rec.CurrentRequest.Value)),
					int64(math.Ceil(rec.RecommendedMemory.Value)))
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
