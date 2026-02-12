package analyzer

import (
	"fmt"
	"sync"

	corev1 "k8s.io/api/core/v1"

	"klim/internal/kubernetes"
	"klim/internal/progress"
	"klim/internal/recommendations"
	"klim/pkg/types"
)

// Analyzer coordinates the analysis process.
type Analyzer struct {
	k8sClient        *kubernetes.Client
	prometheusClient types.PrometheusClient
	engine           *recommendations.Engine
	config           *types.Config
	progressTracker  *progress.Tracker
	bulkData         map[string]map[string][]types.MetricPoint
	bulkDataMu       sync.RWMutex
}

// NewAnalyzer creates a new analyzer.
func NewAnalyzer(
	k8sClient *kubernetes.Client,
	prometheusClient types.PrometheusClient,
	engine *recommendations.Engine,
	config *types.Config,
) *Analyzer {
	return &Analyzer{
		k8sClient:        k8sClient,
		prometheusClient: prometheusClient,
		engine:           engine,
		config:           config,
	}
}

// SetProgressTracker sets the progress tracker for visual feedback.
func (a *Analyzer) SetProgressTracker(tracker *progress.Tracker) {
	a.progressTracker = tracker
}

// Analyze performs the analysis and generates recommendations.
func (a *Analyzer) Analyze() ([]types.Recommendation, error) {
	if a.config.Verbose {
		fmt.Println("Fetching pods from Kubernetes cluster...")
	}

	pods, err := a.k8sClient.GetPods(a.config.Namespaces, a.config.LabelSelector)
	if err != nil {
		return nil, fmt.Errorf("failed to get pods: %w", err)
	}

	if a.config.Verbose {
		fmt.Printf("Found %d pods\n", len(pods))
	}

	// Filter running pods
	runningPods, skippedPods := filterRunningPods(pods)

	if a.config.Verbose {
		fmt.Printf("Analyzing %d running pods", len(runningPods))
		if len(skippedPods) > 0 {
			fmt.Printf(" (%d unhealthy pods skipped)\n", len(skippedPods))
			for _, skipped := range skippedPods {
				fmt.Printf("  - Skipped %s/%s: %s\n", skipped.Namespace, skipped.Name, skipped.Reason)
			}
		} else {
			fmt.Println()
		}
	}

	// Fetch all memory data upfront with a single bulk query
	if a.config.Verbose {
		fmt.Println("Fetching memory usage data from Prometheus...")
	}
	bulkData, err := a.prometheusClient.BulkQueryMemoryUsage(a.config.Namespaces, a.config.HistoryDuration)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch bulk memory data: %w", err)
	}
	a.bulkDataMu.Lock()
	a.bulkData = bulkData
	a.bulkDataMu.Unlock()

	if a.config.Verbose {
		fmt.Printf("Fetched data for %d workloads\n", len(bulkData))
	}

	// Group pods by workload to avoid duplicate analysis of replicas
	workloadPods := make(map[string]corev1.Pod)
	for _, pod := range runningPods {
		appInstance, hasLabel := pod.Labels["app.kubernetes.io/instance"]
		if !hasLabel || appInstance == "" {
			// No label, process individually
			key := fmt.Sprintf("%s/%s", pod.Namespace, pod.Name)
			workloadPods[key] = pod
		} else {
			// Group by workload
			key := fmt.Sprintf("%s/%s", pod.Namespace, appInstance)
			// Use first pod as representative
			if _, exists := workloadPods[key]; !exists {
				workloadPods[key] = pod
			}
		}
	}

	if a.config.Verbose {
		fmt.Printf("Grouped into %d unique workloads (from %d pods)\n", len(workloadPods), len(runningPods))
	}

	// Update progress tracker with actual workload count
	if a.progressTracker != nil {
		a.progressTracker.UpdateTotal(len(workloadPods))
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

	for _, pod := range workloadPods {
		wg.Add(1)
		pod := pod
		go func() {
			defer wg.Done()
			semaphore <- struct{}{}
			defer func() { <-semaphore }()

			if a.progressTracker != nil {
				a.progressTracker.StartProcessing(pod.Namespace, pod.Name)
				defer a.progressTracker.FinishProcessing(pod.Name)
			}

			podRecs := a.analyzePod(pod)
			if len(podRecs) > 0 {
				mu.Lock()
				recommendations = append(recommendations, podRecs...)
				processedCount++
				if a.config.Verbose {
					fmt.Printf("\rProcessed %d/%d workloads with recommendations...", processedCount, len(workloadPods))
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

// analyzePod analyzes a single pod and returns recommendations for its containers.
func (a *Analyzer) analyzePod(pod corev1.Pod) []types.Recommendation {
	var recommendations []types.Recommendation

	workloadKind := kubernetes.GetWorkloadKind(pod)
	workloadName := kubernetes.GetWorkloadName(pod)

	for _, container := range pod.Spec.Containers {
		if a.config.Verbose {
			fmt.Printf("Analyzing %s/%s container %s\n", pod.Namespace, pod.Name, container.Name)
		}

		metrics, err := a.collectMetrics(pod, container.Name)
		if err != nil {
			if a.config.Verbose {
				fmt.Printf("Warning: failed to collect metrics for %s/%s/%s: %v\n",
					pod.Namespace, pod.Name, container.Name, err)
			}
			continue
		}

		if len(metrics.MemoryUsage) == 0 {
			if a.config.Verbose {
				fmt.Printf("Warning: no metrics found for %s/%s/%s (checked %s history)\n",
					pod.Namespace, pod.Name, container.Name, a.config.HistoryDuration)
			}
			continue
		}

		if a.config.Verbose {
			fmt.Printf("Found %d metric points for %s/%s/%s\n",
				len(metrics.MemoryUsage), pod.Namespace, pod.Name, container.Name)
		}

		rec := a.engine.Generate(metrics, workloadKind, workloadName)
		rec.PodLabels = pod.Labels
		recommendations = append(recommendations, rec)
	}

	return recommendations
}

// collectMetrics collects resource metrics for a container.
func (a *Analyzer) collectMetrics(pod corev1.Pod, containerName string) (types.ResourceMetrics, error) {
	// Get the app instance label from the pod
	appInstance, hasLabel := pod.Labels["app.kubernetes.io/instance"]

	var memoryUsage []types.MetricPoint

	// Try to get data from bulk query results
	if hasLabel && appInstance != "" {
		key := fmt.Sprintf("%s/%s", pod.Namespace, appInstance)

		a.bulkDataMu.RLock()
		workloadData, found := a.bulkData[key]
		a.bulkDataMu.RUnlock()

		if found {
			containerData, hasContainer := workloadData[containerName]
			if hasContainer {
				memoryUsage = containerData
				if a.config.Verbose {
					fmt.Printf("  Found bulk data for %s/%s/%s (%d points)\n",
						pod.Namespace, appInstance, containerName, len(memoryUsage))
				}
			}
		}
	}

	// Fallback to pod-specific query if no bulk data available
	if len(memoryUsage) == 0 {
		if a.config.Verbose {
			fmt.Printf("  No bulk data for %s/%s/%s, falling back to pod query\n",
				pod.Namespace, pod.Name, containerName)
		}

		var err error
		memoryUsage, err = a.prometheusClient.QueryMemoryUsage(
			pod.Namespace,
			pod.Name,
			containerName,
			a.config.HistoryDuration,
		)
		if err != nil {
			return types.ResourceMetrics{}, fmt.Errorf("failed to query memory usage: %w", err)
		}
	}

	_, memoryLimit, _, memoryRequest := kubernetes.GetContainerResources(pod, containerName)

	return types.ResourceMetrics{
		Namespace:      pod.Namespace,
		Pod:            pod.Name,
		Container:      containerName,
		MemoryUsage:    memoryUsage,
		CurrentMemory:  memoryLimit,
		CurrentRequest: memoryRequest,
	}, nil
}

type skippedPod struct {
	Namespace string
	Name      string
	Reason    string
}

// filterRunningPods returns only running pods that are healthy.
func filterRunningPods(pods []corev1.Pod) ([]corev1.Pod, []skippedPod) {
	var running []corev1.Pod
	var skipped []skippedPod

	for _, pod := range pods {
		if pod.Status.Phase != corev1.PodRunning {
			continue
		}

		// Skip pods in crash loop state
		isHealthy := true
		skipReason := ""

		for _, containerStatus := range pod.Status.ContainerStatuses {
			if containerStatus.State.Waiting != nil {
				reason := containerStatus.State.Waiting.Reason
				if reason == "CrashLoopBackOff" {
					isHealthy = false
					skipReason = "CrashLoopBackOff"
					break
				}
			}
			// Check restart count - high restart count indicates problems
			if containerStatus.RestartCount > 5 {
				isHealthy = false
				skipReason = fmt.Sprintf("High restart count (%d)", containerStatus.RestartCount)
				break
			}
		}

		if isHealthy {
			running = append(running, pod)
		} else {
			skipped = append(skipped, skippedPod{
				Namespace: pod.Namespace,
				Name:      pod.Name,
				Reason:    skipReason,
			})
		}
	}
	return running, skipped
}
