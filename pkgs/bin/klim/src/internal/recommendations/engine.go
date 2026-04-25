package recommendations

import (
	"fmt"
	"math"

	"klim/pkg/types"
)

// Engine generates resource recommendations.
type Engine struct {
	minMemory float64
}

// NewEngine creates a new recommendation engine.
func NewEngine(minMemory float64) *Engine {
	return &Engine{
		minMemory: minMemory,
	}
}

// Generate creates a recommendation based on VPA recommendations.
func (e *Engine) Generate(metrics types.ResourceMetrics, workloadKind, workloadName string) types.Recommendation {
	// Use VPA upperBound recommendation, apply minMemory floor if configured
	vpaMemoryMB := metrics.VPAMemoryUpperBound / 1000000 // Convert bytes to MB
	recommendedMemoryMB := math.Max(vpaMemoryMB, e.minMemory)

	// Round up both current and recommended for display and percentage calculation
	currentMemoryRounded := math.Ceil(metrics.CurrentMemory.Value)
	recommendedMemoryRounded := math.Ceil(recommendedMemoryMB)

	memoryRecommendation := types.ResourceQuantity{
		Value: recommendedMemoryRounded,
		Unit:  "M",
	}

	// Calculate recommended request - must not exceed limit
	recommendedRequest := metrics.CurrentRequest
	requestLowered := false

	if metrics.CurrentRequest.Value > 0 && math.Ceil(metrics.CurrentRequest.Value) > recommendedMemoryRounded {
		// Lower the request to match the recommended limit
		recommendedRequest = types.ResourceQuantity{
			Value: recommendedMemoryRounded,
			Unit:  "M",
		}
		requestLowered = true
	}

	memoryChange := calculatePercentageChange(currentMemoryRounded, recommendedMemoryRounded)
	severity := determineSeverity(memoryChange)

	return types.Recommendation{
		Namespace:    metrics.Namespace,
		WorkloadName: workloadName,
		WorkloadKind: workloadKind,
		Container:    metrics.Container,
		CurrentMemory: types.ResourceQuantity{
			Value: currentMemoryRounded,
			Unit:  "M",
		},
		CurrentRequest: types.ResourceQuantity{
			Value: math.Ceil(metrics.CurrentRequest.Value),
			Unit:  "M",
		},
		RecommendedMemory:  memoryRecommendation,
		RecommendedRequest: recommendedRequest,
		MemoryChange:       memoryChange,
		Severity:           severity,
		RequestLowered:     requestLowered,
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
	return fmt.Sprintf("%d%s", int64(rq.Value), rq.Unit)
}
