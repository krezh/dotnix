package types

import "time"

// ResourceMetrics holds time-series metrics for a container.
type ResourceMetrics struct {
	Namespace      string
	Pod            string
	Container      string
	MemoryUsage    []MetricPoint
	CurrentMemory  ResourceQuantity
	CurrentRequest ResourceQuantity
}

// MetricPoint represents a single metric value at a point in time.
type MetricPoint struct {
	Timestamp time.Time
	Value     float64
}

// ResourceQuantity represents a Kubernetes resource quantity.
type ResourceQuantity struct {
	Value float64
	Unit  string
}

// Recommendation contains resource recommendations for a container.
type Recommendation struct {
	Namespace          string
	WorkloadName       string
	WorkloadKind       string
	Container          string
	CurrentMemory      ResourceQuantity
	CurrentRequest     ResourceQuantity
	RecommendedMemory  ResourceQuantity
	RecommendedRequest ResourceQuantity
	MemoryChange       float64           // percentage change
	Severity           string            // e.g., "critical", "warning", "info"
	PodLabels          map[string]string // Pod labels for manifest lookup
	ManifestPath       string            // Path to HelmRelease manifest
	RequestLowered     bool              // True if request was lowered to match limit
	MemoryHistory      []MetricPoint     // Historical memory usage data
}

// Config holds the configuration for klim.
type Config struct {
	PrometheusURL     string
	Namespaces        []string
	Contexts          []string
	LabelSelector     string
	OutputFormat      string
	OutputFile        string
	GitRepoPath       string
	HistoryDuration   time.Duration
	MemoryBuffer      float64
	MinMemory         float64
	Verbose           bool
	JobGroupingLabels []string
	Concurrency       int
}

// PrometheusClient defines the interface for querying Prometheus.
type PrometheusClient interface {
	QueryMemoryUsage(namespace, pod, container string, duration time.Duration) ([]MetricPoint, error)
	QueryMemoryUsageByWorkload(namespace, workloadName, container string, duration time.Duration) ([]MetricPoint, error)
	BulkQueryMemoryUsage(namespaces []string, duration time.Duration) (map[string]map[string][]MetricPoint, error)
}
