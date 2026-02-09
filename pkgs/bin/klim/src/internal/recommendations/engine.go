package recommendations

import (
	"fmt"
	"math"

	"klim/pkg/types"
)

// Engine generates resource recommendations.
type Engine struct {
	memoryBuffer float64
	minMemory    float64
}

// NewEngine creates a new recommendation engine.
func NewEngine(memoryBuffer, minMemory float64) *Engine {
	return &Engine{
		memoryBuffer: memoryBuffer,
		minMemory:    minMemory,
	}
}

// Generate creates a recommendation based on resource metrics.
func (e *Engine) Generate(metrics types.ResourceMetrics, workloadKind, workloadName string) types.Recommendation {
	memoryRecommendation := e.calculateMemoryRecommendation(metrics.MemoryUsage)

	// Calculate recommended request - must not exceed limit
	recommendedRequest := metrics.CurrentRequest
	requestLowered := false

	if metrics.CurrentRequest.Value > 0 && metrics.CurrentRequest.Value > memoryRecommendation.Value {
		// Lower the request to match the recommended limit
		recommendedRequest = types.ResourceQuantity{
			Value: memoryRecommendation.Value,
			Unit:  "Mi",
		}
		requestLowered = true
	}

	memoryChange := calculatePercentageChange(metrics.CurrentMemory.Value, memoryRecommendation.Value)
	severity := determineSeverity(memoryChange)

	return types.Recommendation{
		Namespace:          metrics.Namespace,
		WorkloadName:       workloadName,
		WorkloadKind:       workloadKind,
		Container:          metrics.Container,
		CurrentMemory:      metrics.CurrentMemory,
		CurrentRequest:     metrics.CurrentRequest,
		RecommendedMemory:  memoryRecommendation,
		RecommendedRequest: recommendedRequest,
		MemoryChange:       memoryChange,
		Severity:           severity,
		RequestLowered:     requestLowered,
		MemoryHistory:      metrics.MemoryUsage,
	}
}

// calculateMemoryRecommendation computes memory recommendation using peak + buffer.
func (e *Engine) calculateMemoryRecommendation(memoryUsage []types.MetricPoint) types.ResourceQuantity {
	if len(memoryUsage) == 0 {
		return types.ResourceQuantity{Value: e.minMemory, Unit: "Mi"}
	}

	peak := 0.0
	for _, point := range memoryUsage {
		// Convert bytes to MiB
		valueMiB := point.Value / (1024 * 1024)
		if valueMiB > peak {
			peak = valueMiB
		}
	}

	recommended := peak * (1.0 + e.memoryBuffer)
	recommended = math.Max(recommended, e.minMemory)

	// Round up to nearest integer
	return types.ResourceQuantity{
		Value: math.Ceil(recommended),
		Unit:  "Mi",
	}
}

// calculatePercentageChange computes the percentage change between current and recommended.
func calculatePercentageChange(current, recommended float64) float64 {
	if current == 0 {
		if recommended == 0 {
			return 0
		}
		return 100 // Show 100% when going from nothing to something
	}
	return ((recommended - current) / current) * 100
}

// determineSeverity assigns a severity level based on memory change.
func determineSeverity(memoryChange float64) string {
	absChange := math.Abs(memoryChange)

	switch {
	case absChange > 50:
		return "critical"
	case absChange > 25:
		return "warning"
	default:
		return "info"
	}
}

// FormatResourceQuantity formats a resource quantity as a string.
func FormatResourceQuantity(rq types.ResourceQuantity) string {
	if rq.Unit == "" {
		return "N/A"
	}
	// Value should already be an integer from math.Ceil
	return fmt.Sprintf("%d%s", int64(rq.Value), rq.Unit)
}
