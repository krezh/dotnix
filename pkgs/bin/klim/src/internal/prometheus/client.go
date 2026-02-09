package prometheus

import (
	"context"
	"fmt"
	"os"
	"time"

	"github.com/prometheus/client_golang/api"
	v1 "github.com/prometheus/client_golang/api/prometheus/v1"
	"github.com/prometheus/common/model"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/rest"

	"klim/pkg/types"
)

// Client implements the PrometheusClient interface.
type Client struct {
	api      v1.API
	timeout  time.Duration
	endpoint string
}

// NewClient creates a new Prometheus client.
func NewClient(endpoint string, tlsConfig *rest.Config) (*Client, error) {
	cfg := api.Config{
		Address: endpoint,
	}

	// Configure TLS if using Kubernetes API proxy
	if tlsConfig != nil {
		transport, err := rest.TransportFor(tlsConfig)
		if err != nil {
			return nil, fmt.Errorf("failed to create transport: %w", err)
		}
		cfg.RoundTripper = transport
	}

	client, err := api.NewClient(cfg)
	if err != nil {
		return nil, fmt.Errorf("failed to create prometheus client: %w", err)
	}

	return &Client{
		api:      v1.NewAPI(client),
		timeout:  60 * time.Second,
		endpoint: endpoint,
	}, nil
}

// QueryMemoryUsage queries memory usage metrics from Prometheus.
func (c *Client) QueryMemoryUsage(namespace, pod, container string, duration time.Duration) ([]types.MetricPoint, error) {
	// Use regex to match pod name prefix (handles pod restarts)
	// e.g., "echo-server-abc123" -> "echo-server-.*"
	// This captures historical data from previous pods in the same workload
	podPrefix := pod
	// Remove pod hash suffix if present (last segment after last hyphen followed by alphanumeric)
	if len(pod) > 10 {
		// Try to strip common hash patterns
		// ReplicaSet: name-abc123def-xyz89
		// StatefulSet: name-0, name-1
		for i := len(pod) - 1; i >= 0; i-- {
			if pod[i] == '-' {
				// Check if what follows looks like a hash (all alphanumeric, 5+ chars)
				suffix := pod[i+1:]
				if len(suffix) >= 5 && len(suffix) <= 10 {
					podPrefix = pod[:i]
					break
				}
			}
		}
	}

	query := fmt.Sprintf(
		`max(container_memory_working_set_bytes{job="kubelet", metrics_path="/metrics/cadvisor", image!="", namespace="%s", pod=~"%s.*", container="%s"})`,
		namespace, podPrefix, container,
	)

	return c.queryRange(query, duration)
}

// QueryMemoryUsageByWorkload queries by workload name using kube_pod_owner join.
// This is the most reliable method across pod restarts.
func (c *Client) QueryMemoryUsageByWorkload(namespace, workloadName, container string, duration time.Duration) ([]types.MetricPoint, error) {
	// Use kube_pod_owner to join container metrics with owner information
	// This properly handles:
	// - Deployments: owner_name will be ReplicaSet like "app-abc123", regex strips to "app"
	// - StatefulSets: owner_name is "app", stays as "app"
	// - DaemonSets: owner_name is "app", stays as "app"
	query := fmt.Sprintf(
		`max(
			container_memory_working_set_bytes{
				job="kubelet",
				metrics_path="/metrics/cadvisor",
				namespace="%s",
				container="%s",
				image!="",
				container!=""
			}
			* on(namespace, pod) group_left(owner_name)
			(
				label_replace(
					max by (namespace, pod, owner_name, owner_kind) (
						kube_pod_owner{
							namespace="%s",
							owner_kind=~"StatefulSet|DaemonSet|ReplicaSet",
							owner_name=~"%s(-[a-z0-9]+)?"
						}
					),
					"owner_name", "$1", "owner_name", "^(.*)-[a-z0-9]+$"
				)
			)
		)`,
		namespace, container, namespace, workloadName,
	)

	if os.Getenv("KLIM_DEBUG_QUERIES") == "true" {
		fmt.Printf("DEBUG: Workload query: %s\n", query)
	}

	return c.queryRange(query, duration)
}

// queryRange executes a range query and returns metric points.
func (c *Client) queryRange(query string, duration time.Duration) ([]types.MetricPoint, error) {
	ctx, cancel := context.WithTimeout(context.Background(), c.timeout)
	defer cancel()

	end := time.Now()
	start := end.Add(-duration)

	// Calculate appropriate step based on duration to avoid too many data points
	// Target ~500 data points for faster queries
	step := duration / 500
	if step < time.Minute {
		step = time.Minute // Minimum 1 minute
	}

	if os.Getenv("KLIM_DEBUG_QUERIES") == "true" {
		fmt.Printf("DEBUG: Query range: %s to %s (%s), step: %s\n", start.Format(time.RFC3339), end.Format(time.RFC3339), duration, step)
	}

	result, warnings, err := c.api.QueryRange(ctx, query, v1.Range{
		Start: start,
		End:   end,
		Step:  step,
	})

	if err != nil {
		return nil, fmt.Errorf("prometheus query failed: %w", err)
	}

	if len(warnings) > 0 {
		for _, w := range warnings {
			fmt.Printf("Warning: %s\n", w)
		}
	}

	matrix, ok := result.(model.Matrix)
	if !ok {
		return nil, fmt.Errorf("unexpected result type: %T", result)
	}

	if len(matrix) == 0 {
		if os.Getenv("KLIM_DEBUG_QUERIES") == "true" {
			fmt.Printf("DEBUG: No data returned from query\n")
		}
		return []types.MetricPoint{}, nil
	}

	points := make([]types.MetricPoint, 0, len(matrix[0].Values))
	for _, sample := range matrix[0].Values {
		points = append(points, types.MetricPoint{
			Timestamp: sample.Timestamp.Time(),
			Value:     float64(sample.Value),
		})
	}

	if os.Getenv("KLIM_DEBUG_QUERIES") == "true" {
		fmt.Printf("DEBUG: Returned %d data points\n", len(points))
	}

	return points, nil
}

// BulkQueryMemoryUsage fetches all memory data for all workloads in specified namespaces.
// Returns a map of "namespace/workload/container" -> []types.MetricPoint for fast lookup.
func (c *Client) BulkQueryMemoryUsage(namespaces []string, duration time.Duration) (map[string]map[string][]types.MetricPoint, error) {
	// Build namespace filter
	namespaceFilter := ""
	if len(namespaces) > 0 {
		namespaceFilter = fmt.Sprintf(`namespace=~"%s"`, joinNamespaces(namespaces))
	} else {
		namespaceFilter = `namespace=~".+"`
	}

	// Query all workload memory usage at once
	// Use max by to deduplicate kube_pod_owner in case of stale metrics
	query := fmt.Sprintf(
		`max by (namespace, owner_name, container) (
			container_memory_working_set_bytes{
				job="kubelet",
				metrics_path="/metrics/cadvisor",
				%s,
				image!="",
				container!=""
			}
			* on(namespace, pod) group_left(owner_name)
			(
				label_replace(
					max by (namespace, pod, owner_name, owner_kind) (
						kube_pod_owner{
							%s,
							owner_kind=~"StatefulSet|DaemonSet|ReplicaSet"
						}
					),
					"owner_name", "$1", "owner_name", "^(.*)-[a-z0-9]+$"
				)
			)
		)`,
		namespaceFilter, namespaceFilter,
	)

	if os.Getenv("KLIM_DEBUG_QUERIES") == "true" {
		fmt.Printf("DEBUG: Bulk query: %s\n", query)
	}

	ctx, cancel := context.WithTimeout(context.Background(), c.timeout)
	defer cancel()

	end := time.Now()
	start := end.Add(-duration)

	// Use larger step for bulk queries
	step := duration / 300
	if step < time.Minute {
		step = time.Minute
	}

	result, warnings, err := c.api.QueryRange(ctx, query, v1.Range{
		Start: start,
		End:   end,
		Step:  step,
	})

	if err != nil {
		return nil, fmt.Errorf("prometheus bulk query failed: %w", err)
	}

	if len(warnings) > 0 {
		for _, w := range warnings {
			fmt.Printf("Warning: %s\n", w)
		}
	}

	matrix, ok := result.(model.Matrix)
	if !ok {
		return nil, fmt.Errorf("unexpected result type: %T", result)
	}

	// Parse results into nested map: namespace/workload -> container -> []types.MetricPoint
	results := make(map[string]map[string][]types.MetricPoint)

	for _, stream := range matrix {
		namespace := string(stream.Metric["namespace"])
		ownerName := string(stream.Metric["owner_name"])
		container := string(stream.Metric["container"])

		if namespace == "" || ownerName == "" || container == "" {
			continue
		}

		key := fmt.Sprintf("%s/%s", namespace, ownerName)

		if results[key] == nil {
			results[key] = make(map[string][]types.MetricPoint)
		}

		points := make([]types.MetricPoint, 0, len(stream.Values))
		for _, sample := range stream.Values {
			points = append(points, types.MetricPoint{
				Timestamp: sample.Timestamp.Time(),
				Value:     float64(sample.Value),
			})
		}

		results[key][container] = points
	}

	if os.Getenv("KLIM_DEBUG_QUERIES") == "true" {
		fmt.Printf("DEBUG: Bulk query returned data for %d workloads\n", len(results))
	}

	return results, nil
}

// joinNamespaces creates a regex pattern for multiple namespaces.
func joinNamespaces(namespaces []string) string {
	if len(namespaces) == 0 {
		return ".+"
	}
	if len(namespaces) == 1 {
		return namespaces[0]
	}
	// Multiple namespaces: "ns1|ns2|ns3"
	result := ""
	for i, ns := range namespaces {
		if i > 0 {
			result += "|"
		}
		result += ns
	}
	return result
}

// DiscoverPrometheus attempts to discover the Prometheus endpoint from the cluster using Kubernetes API.
func DiscoverPrometheus(k8sClient kubernetes.Interface, config *rest.Config) (string, error) {
	labelSelectors := []string{
		"app=kube-prometheus-stack-prometheus",
		"app=prometheus,component=server",
		"app=prometheus-server",
		"app=prometheus-operator-prometheus",
		"app=rancher-monitoring-prometheus",
		"app=prometheus-prometheus",
		"app.kubernetes.io/name=prometheus,app.kubernetes.io/component=server",
		"app=stack-prometheus",
	}

	// Detect if running inside cluster
	_, insideCluster := os.LookupEnv("KUBERNETES_SERVICE_HOST")

	for _, selector := range labelSelectors {
		services, err := k8sClient.CoreV1().Services("").List(context.TODO(), metav1.ListOptions{
			LabelSelector: selector,
		})
		if err != nil {
			continue
		}

		if len(services.Items) == 0 {
			continue
		}

		svc := services.Items[0]
		name := svc.Name
		namespace := svc.Namespace
		port := svc.Spec.Ports[0].Port

		if insideCluster {
			// Inside cluster: use internal DNS
			url := fmt.Sprintf("http://%s.%s.svc.cluster.local:%d", name, namespace, port)
			return url, nil
		} else {
			// Outside cluster: use Kubernetes API proxy
			url := fmt.Sprintf("%s/api/v1/namespaces/%s/services/%s:%d/proxy", config.Host, namespace, name, port)
			return url, nil
		}
	}

	return "", fmt.Errorf("failed to discover prometheus endpoint using label selectors")
}
