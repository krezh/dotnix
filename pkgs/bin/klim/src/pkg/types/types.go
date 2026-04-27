package types

// ResourceMetrics holds VPA recommendation data for a container.
type ResourceMetrics struct {
	Namespace           string
	Pod                 string
	Container           string
	VPAMemoryTarget     float64
	VPAMemoryLowerBound float64
	VPAMemoryUpperBound float64
	CurrentMemory       ResourceQuantity
	CurrentRequest      ResourceQuantity
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
}

// Config holds the configuration for klim.
type Config struct {
	Namespaces        []string
	ExcludeNamespaces []string
	Contexts          []string
	LabelSelector     string
	OutputFormat      string
	OutputFile        string
	GitRepoPath       string
	MinMemory         float64
	Verbose           bool
	JobGroupingLabels []string
	Concurrency       int
}

// VPARecommendation contains memory recommendation values from VPA.
type VPARecommendation struct {
	MemoryTarget     float64
	MemoryLowerBound float64
	MemoryUpperBound float64
	MemoryUncapped   float64
}

// VPAWorkload contains VPA metadata and recommendations for a workload.
type VPAWorkload struct {
	Namespace  string
	Name       string
	TargetRef  TargetRef
	Containers map[string]VPARecommendation
}

// TargetRef identifies the workload targeted by a VPA.
type TargetRef struct {
	Kind string // Deployment, StatefulSet, DaemonSet, etc.
	Name string
}

// VPAClient defines the interface for querying VPA recommendations.
type VPAClient interface {
	ListVPAWorkloads(namespaces []string, excludeNamespaces []string) ([]VPAWorkload, error)
	SetVerbose(verbose bool)
}
