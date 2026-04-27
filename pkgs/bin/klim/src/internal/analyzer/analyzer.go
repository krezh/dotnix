package analyzer

import (
	"fmt"
	"sync"

	"klim/internal/kubernetes"
	"klim/internal/progress"
	"klim/internal/recommendations"
	"klim/pkg/types"
)

// Analyzer coordinates the analysis process.
type Analyzer struct {
	k8sClient       *kubernetes.Client
	vpaClient       types.VPAClient
	engine          *recommendations.Engine
	config          *types.Config
	progressTracker *progress.Tracker
	workloadCache   *kubernetes.WorkloadCache
}

// NewAnalyzer creates a new analyzer.
func NewAnalyzer(
	k8sClient *kubernetes.Client,
	vpaClient types.VPAClient,
	engine *recommendations.Engine,
	config *types.Config,
) *Analyzer {
	return &Analyzer{
		k8sClient: k8sClient,
		vpaClient: vpaClient,
		engine:    engine,
		config:    config,
	}
}

// SetProgressTracker sets the progress tracker for visual feedback.
func (a *Analyzer) SetProgressTracker(tracker *progress.Tracker) {
	a.progressTracker = tracker
}

// Analyze performs the analysis and generates recommendations.
func (a *Analyzer) Analyze() ([]types.Recommendation, error) {
	if a.config.Verbose {
		fmt.Println("Fetching VPA recommendations from Kubernetes API...")
	}

	vpaWorkloads, err := a.vpaClient.ListVPAWorkloads(a.config.Namespaces, a.config.ExcludeNamespaces)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch VPA recommendations: %w", err)
	}

	if a.config.Verbose {
		fmt.Printf("Found %d VPAs with recommendations\n", len(vpaWorkloads))
	}

	// Bulk fetch all workloads to avoid individual API calls
	if a.config.Verbose {
		fmt.Println("Bulk fetching workload specs...")
	}
	workloadCache, err := a.k8sClient.BulkFetchWorkloads(a.config.Namespaces)
	if err != nil {
		return nil, fmt.Errorf("failed to bulk fetch workloads: %w", err)
	}
	a.workloadCache = workloadCache

	// Update progress tracker with VPA count
	if a.progressTracker != nil {
		a.progressTracker.UpdateTotal(len(vpaWorkloads))
	}

	var recommendations []types.Recommendation
	var mu sync.Mutex
	var wg sync.WaitGroup
	var processedCount int

	concurrency := a.config.Concurrency
	if concurrency <= 0 {
		concurrency = 10
	}
	semaphore := make(chan struct{}, concurrency)

	if a.config.Verbose {
		fmt.Printf("Processing with concurrency: %d\n", concurrency)
	}

	for _, vpaWorkload := range vpaWorkloads {
		wg.Add(1)
		vpaWorkload := vpaWorkload
		go func() {
			defer wg.Done()
			semaphore <- struct{}{}
			defer func() { <-semaphore }()

			if a.progressTracker != nil {
				a.progressTracker.StartProcessing(vpaWorkload.Namespace, vpaWorkload.TargetRef.Name)
				defer a.progressTracker.FinishProcessing(vpaWorkload.TargetRef.Name)
			}

			workloadRecs := a.analyzeVPAWorkload(vpaWorkload)
			if len(workloadRecs) > 0 {
				mu.Lock()
				recommendations = append(recommendations, workloadRecs...)
				processedCount++
				if a.config.Verbose {
					fmt.Printf("\rProcessed %d/%d workloads with recommendations...", processedCount, len(vpaWorkloads))
				}
				mu.Unlock()
			}
		}()
	}

	wg.Wait()

	if a.config.Verbose && processedCount > 0 {
		fmt.Println() // New line after progress
	}

	if a.config.Verbose {
		fmt.Printf("Generated %d recommendations\n", len(recommendations))
	}

	return recommendations, nil
}

// analyzeVPAWorkload analyzes a VPA workload and returns recommendations for its containers.
func (a *Analyzer) analyzeVPAWorkload(vpaWorkload types.VPAWorkload) []types.Recommendation {
	var recommendations []types.Recommendation

	// Get workload containers from cache
	containers, labels, err := a.workloadCache.GetWorkloadContainers(vpaWorkload.Namespace, vpaWorkload.TargetRef)
	if err != nil {
		if a.config.Verbose {
			fmt.Printf("Warning: failed to get workload %s/%s %s: %v\n",
				vpaWorkload.Namespace, vpaWorkload.TargetRef.Kind, vpaWorkload.TargetRef.Name, err)
		}
		return recommendations
	}

	// Process each container
	for _, container := range containers {
		vpaRec, hasRec := vpaWorkload.Containers[container.Name]
		if !hasRec {
			if a.config.Verbose {
				fmt.Printf("Warning: no VPA recommendation for container %s in %s/%s\n",
					container.Name, vpaWorkload.TargetRef.Kind, vpaWorkload.TargetRef.Name)
			}
			continue
		}

		if vpaRec.MemoryTarget == 0 {
			if a.config.Verbose {
				fmt.Printf("Warning: VPA recommendation has zero memory target for %s/%s/%s\n",
					vpaWorkload.Namespace, vpaWorkload.TargetRef.Name, container.Name)
			}
			continue
		}

		// Get current resources from container spec
		_, memoryLimit, _, memoryRequest := kubernetes.GetContainerResourcesFromSpec(container)

		// Skip containers without memory limits
		if memoryLimit.Value == 0 {
			if a.config.Verbose {
				fmt.Printf("Skipping %s/%s/%s: no memory limit set\n",
					vpaWorkload.Namespace, vpaWorkload.TargetRef.Name, container.Name)
			}
			continue
		}

		metrics := types.ResourceMetrics{
			Namespace:           vpaWorkload.Namespace,
			Pod:                 vpaWorkload.TargetRef.Name,
			Container:           container.Name,
			VPAMemoryTarget:     vpaRec.MemoryTarget,
			VPAMemoryLowerBound: vpaRec.MemoryLowerBound,
			VPAMemoryUpperBound: vpaRec.MemoryUpperBound,
			CurrentMemory:       memoryLimit,
			CurrentRequest:      memoryRequest,
		}

		rec := a.engine.Generate(metrics, vpaWorkload.TargetRef.Kind, vpaWorkload.TargetRef.Name)
		rec.PodLabels = labels
		recommendations = append(recommendations, rec)
	}

	return recommendations
}
