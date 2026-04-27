package vpa

import (
	"context"
	"fmt"

	"k8s.io/apimachinery/pkg/api/resource"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"k8s.io/client-go/dynamic"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/rest"

	"klim/pkg/types"
)

// Client wraps the VPA client.
type Client struct {
	dynamicClient dynamic.Interface
	verbose       bool
}

var vpaGVR = schema.GroupVersionResource{
	Group:    "autoscaling.k8s.io",
	Version:  "v1",
	Resource: "verticalpodautoscalers",
}

// NewClient creates a new VPA client.
func NewClient(clientset *kubernetes.Clientset, config *rest.Config) (*Client, error) {
	dynamicClient, err := dynamic.NewForConfig(config)
	if err != nil {
		return nil, fmt.Errorf("failed to create dynamic client: %w", err)
	}

	return &Client{
		dynamicClient: dynamicClient,
	}, nil
}

// SetVerbose enables verbose logging.
func (c *Client) SetVerbose(verbose bool) {
	c.verbose = verbose
}

// ListVPAWorkloads fetches all VPA recommendations for the specified namespaces.
// Returns a list of VPAWorkload objects containing targetRef and container recommendations.
func (c *Client) ListVPAWorkloads(namespaces []string, excludeNamespaces []string) ([]types.VPAWorkload, error) {
	var workloads []types.VPAWorkload

	if len(namespaces) == 0 {
		namespaces = []string{metav1.NamespaceAll}
	}

	// Create a map for fast exclude lookup
	excludeMap := make(map[string]bool)
	for _, ns := range excludeNamespaces {
		excludeMap[ns] = true
	}

	for _, namespace := range namespaces {
		vpas, err := c.dynamicClient.Resource(vpaGVR).Namespace(namespace).List(context.TODO(), metav1.ListOptions{})
		if err != nil {
			return nil, fmt.Errorf("failed to list VPAs in namespace %s: %w", namespace, err)
		}

		if c.verbose {
			fmt.Printf("Found %d VPAs in namespace %s\n", len(vpas.Items), namespace)
		}

		for _, vpa := range vpas.Items {
			vpaNamespace := vpa.GetNamespace()

			// Skip excluded namespaces
			if excludeMap[vpaNamespace] {
				if c.verbose {
					fmt.Printf("Skipping VPA %s/%s (namespace excluded)\n", vpaNamespace, vpa.GetName())
				}
				continue
			}

			workload, err := c.parseVPA(&vpa)
			if err != nil {
				if c.verbose {
					fmt.Printf("Warning: failed to parse VPA %s/%s: %v\n", vpaNamespace, vpa.GetName(), err)
				}
				continue
			}
			if workload != nil {
				workloads = append(workloads, *workload)
			}
		}
	}

	return workloads, nil
}

// parseVPA extracts recommendations from a VPA object and returns a VPAWorkload.
func (c *Client) parseVPA(vpa *unstructured.Unstructured) (*types.VPAWorkload, error) {
	namespace := vpa.GetNamespace()
	vpaName := vpa.GetName()

	// Extract spec.targetRef
	targetRef, found, err := unstructured.NestedMap(vpa.Object, "spec", "targetRef")
	if err != nil || !found {
		return nil, fmt.Errorf("spec.targetRef not found")
	}

	targetKind, ok := targetRef["kind"].(string)
	if !ok {
		return nil, fmt.Errorf("spec.targetRef.kind is not a string")
	}

	targetName, ok := targetRef["name"].(string)
	if !ok {
		return nil, fmt.Errorf("spec.targetRef.name is not a string")
	}

	// Extract status.recommendation.containerRecommendations
	containerRecs, found, err := unstructured.NestedSlice(vpa.Object, "status", "recommendation", "containerRecommendations")
	if err != nil || !found {
		if c.verbose {
			fmt.Printf("VPA %s/%s has no status.recommendation yet\n", namespace, vpaName)
		}
		return nil, nil
	}

	workload := &types.VPAWorkload{
		Namespace: namespace,
		Name:      vpaName,
		TargetRef: types.TargetRef{
			Kind: targetKind,
			Name: targetName,
		},
		Containers: make(map[string]types.VPARecommendation),
	}

	// Parse each container recommendation
	for _, cr := range containerRecs {
		rec, ok := cr.(map[string]interface{})
		if !ok {
			continue
		}

		containerName, ok := rec["containerName"].(string)
		if !ok {
			continue
		}

		recommendation := types.VPARecommendation{}

		// Extract target memory
		if target, ok := rec["target"].(map[string]interface{}); ok {
			if memoryStr, ok := target["memory"].(string); ok {
				recommendation.MemoryTarget = parseQuantity(memoryStr)
			}
		}

		// Extract lowerBound memory
		if lowerBound, ok := rec["lowerBound"].(map[string]interface{}); ok {
			if memoryStr, ok := lowerBound["memory"].(string); ok {
				recommendation.MemoryLowerBound = parseQuantity(memoryStr)
			}
		}

		// Extract upperBound memory
		if upperBound, ok := rec["upperBound"].(map[string]interface{}); ok {
			if memoryStr, ok := upperBound["memory"].(string); ok {
				recommendation.MemoryUpperBound = parseQuantity(memoryStr)
			}
		}

		// Extract uncappedTarget memory
		if uncappedTarget, ok := rec["uncappedTarget"].(map[string]interface{}); ok {
			if memoryStr, ok := uncappedTarget["memory"].(string); ok {
				recommendation.MemoryUncapped = parseQuantity(memoryStr)
			}
		}

		workload.Containers[containerName] = recommendation

		if c.verbose {
			fmt.Printf("  VPA %s targets %s/%s, container %s: %.0f bytes\n",
				vpaName, targetKind, targetName, containerName, recommendation.MemoryTarget)
		}
	}

	return workload, nil
}

// parseQuantity converts a Kubernetes quantity string (e.g., "128Mi") to bytes.
func parseQuantity(s string) float64 {
	q, err := resource.ParseQuantity(s)
	if err != nil {
		return 0
	}
	return float64(q.Value())
}
