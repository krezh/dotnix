package kubernetes

import (
	"context"
	"fmt"
	"os"
	"path/filepath"

	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/rest"
	"k8s.io/client-go/tools/clientcmd"

	"klim/pkg/types"
)

// Client wraps the Kubernetes client.
type Client struct {
	clientset *kubernetes.Clientset
	config    *rest.Config
	context   string
}

// Clientset returns the underlying Kubernetes clientset.
func (c *Client) Clientset() *kubernetes.Clientset {
	return c.clientset
}

// Config returns the underlying Kubernetes config.
func (c *Client) Config() *rest.Config {
	return c.config
}

// NewClient creates a new Kubernetes client.
func NewClient(contextName string) (*Client, error) {
	var config *rest.Config
	var err error

	if contextName == "" {
		// Try in-cluster config first
		config, err = rest.InClusterConfig()
		if err != nil {
			// Fall back to kubeconfig
			config, contextName, err = getKubeconfigWithContext("")
			if err != nil {
				return nil, fmt.Errorf("failed to get kubeconfig: %w", err)
			}
		}
	} else {
		config, contextName, err = getKubeconfigWithContext(contextName)
		if err != nil {
			return nil, fmt.Errorf("failed to get kubeconfig for context %s: %w", contextName, err)
		}
	}

	clientset, err := kubernetes.NewForConfig(config)
	if err != nil {
		return nil, fmt.Errorf("failed to create kubernetes client: %w", err)
	}

	return &Client{
		clientset: clientset,
		config:    config,
		context:   contextName,
	}, nil
}

// getKubeconfigWithContext loads kubeconfig with optional context override.
func getKubeconfigWithContext(contextName string) (*rest.Config, string, error) {
	kubeconfig := os.Getenv("KUBECONFIG")
	if kubeconfig == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			return nil, "", err
		}
		kubeconfig = filepath.Join(home, ".kube", "config")
	}

	loadingRules := &clientcmd.ClientConfigLoadingRules{ExplicitPath: kubeconfig}
	configOverrides := &clientcmd.ConfigOverrides{}

	if contextName != "" {
		configOverrides.CurrentContext = contextName
	}

	config, err := clientcmd.NewNonInteractiveDeferredLoadingClientConfig(
		loadingRules,
		configOverrides,
	).ClientConfig()

	if err != nil {
		return nil, "", err
	}

	// Get the actual context name
	rawConfig, err := clientcmd.NewNonInteractiveDeferredLoadingClientConfig(
		loadingRules,
		configOverrides,
	).RawConfig()

	if err != nil {
		return nil, "", err
	}

	actualContext := rawConfig.CurrentContext
	if contextName != "" {
		actualContext = contextName
	}

	return config, actualContext, nil
}

// GetPods returns all pods in the specified namespaces with label selector.
func (c *Client) GetPods(namespaces []string, labelSelector string) ([]corev1.Pod, error) {
	var allPods []corev1.Pod

	if len(namespaces) == 0 {
		namespaces = []string{corev1.NamespaceAll}
	}

	for _, namespace := range namespaces {
		pods, err := c.clientset.CoreV1().Pods(namespace).List(context.TODO(), metav1.ListOptions{
			LabelSelector: labelSelector,
		})
		if err != nil {
			return nil, fmt.Errorf("failed to list pods in namespace %s: %w", namespace, err)
		}
		allPods = append(allPods, pods.Items...)
	}

	return allPods, nil
}

// GetContainerResources extracts current resource requests and limits from a pod.
func GetContainerResources(pod corev1.Pod, containerName string) (cpuLimit, memoryLimit, cpuRequest, memoryRequest types.ResourceQuantity) {
	for _, container := range pod.Spec.Containers {
		if container.Name == containerName {
			if mem, ok := container.Resources.Limits[corev1.ResourceMemory]; ok {
				memoryLimit = types.ResourceQuantity{
					Value: float64(mem.Value()) / 1000000, // Convert bytes to MB
					Unit:  "M",
				}
			}
			if mem, ok := container.Resources.Requests[corev1.ResourceMemory]; ok {
				memoryRequest = types.ResourceQuantity{
					Value: float64(mem.Value()) / 1000000, // Convert bytes to MB
					Unit:  "M",
				}
			}
			break
		}
	}
	return
}

// GetWorkloadKind determines the workload kind (Deployment, StatefulSet, etc.) for a pod.
func GetWorkloadKind(pod corev1.Pod) string {
	for _, owner := range pod.OwnerReferences {
		switch owner.Kind {
		case "ReplicaSet":
			// For Deployments, the owner is a ReplicaSet
			return "Deployment"
		case "StatefulSet", "DaemonSet", "Job", "CronJob":
			return owner.Kind
		}
	}
	return "Pod"
}

// GetWorkloadName extracts the workload name from a pod.
func GetWorkloadName(pod corev1.Pod) string {
	for _, owner := range pod.OwnerReferences {
		if owner.Kind == "ReplicaSet" {
			// Strip the replica set hash suffix (format: name-<10-char-hash>)
			// Remove 11 characters: 10 for hash + 1 for dash
			name := owner.Name
			if len(name) > 11 {
				return name[:len(name)-11]
			}
			return name
		}
		return owner.Name
	}
	return pod.Name
}

// CreatePodFromLabels creates a minimal pod object with labels for manifest lookup.
func CreatePodFromLabels(namespace, name string, labels map[string]string) corev1.Pod {
	return corev1.Pod{
		ObjectMeta: metav1.ObjectMeta{
			Namespace: namespace,
			Name:      name,
			Labels:    labels,
		},
	}
}

// WorkloadCache caches workload specs to avoid repeated API calls.
type WorkloadCache struct {
	deployments  map[string]map[string][]corev1.Container // namespace -> name -> containers
	statefulsets map[string]map[string][]corev1.Container
	daemonsets   map[string]map[string][]corev1.Container
	cronjobs     map[string]map[string][]corev1.Container
	jobs         map[string]map[string][]corev1.Container
	labels       map[string]map[string]map[string]string // namespace -> name -> labels
}

// BulkFetchWorkloads fetches all workloads in the specified namespaces at once.
func (c *Client) BulkFetchWorkloads(namespaces []string) (*WorkloadCache, error) {
	ctx := context.TODO()
	cache := &WorkloadCache{
		deployments:  make(map[string]map[string][]corev1.Container),
		statefulsets: make(map[string]map[string][]corev1.Container),
		daemonsets:   make(map[string]map[string][]corev1.Container),
		cronjobs:     make(map[string]map[string][]corev1.Container),
		jobs:         make(map[string]map[string][]corev1.Container),
		labels:       make(map[string]map[string]map[string]string),
	}

	if len(namespaces) == 0 {
		namespaces = []string{corev1.NamespaceAll}
	}

	for _, ns := range namespaces {
		// Fetch Deployments
		deployments, err := c.clientset.AppsV1().Deployments(ns).List(ctx, metav1.ListOptions{})
		if err != nil {
			return nil, fmt.Errorf("failed to list deployments: %w", err)
		}
		for _, d := range deployments.Items {
			// Use the actual namespace from the object, not the query namespace
			actualNs := d.Namespace
			if cache.deployments[actualNs] == nil {
				cache.deployments[actualNs] = make(map[string][]corev1.Container)
			}
			if cache.labels[actualNs] == nil {
				cache.labels[actualNs] = make(map[string]map[string]string)
			}
			cache.deployments[actualNs][d.Name] = d.Spec.Template.Spec.Containers
			cache.labels[actualNs][d.Name] = d.Spec.Template.Labels
		}

		// Fetch StatefulSets
		statefulsets, err := c.clientset.AppsV1().StatefulSets(ns).List(ctx, metav1.ListOptions{})
		if err != nil {
			return nil, fmt.Errorf("failed to list statefulsets: %w", err)
		}
		for _, s := range statefulsets.Items {
			actualNs := s.Namespace
			if cache.statefulsets[actualNs] == nil {
				cache.statefulsets[actualNs] = make(map[string][]corev1.Container)
			}
			if cache.labels[actualNs] == nil {
				cache.labels[actualNs] = make(map[string]map[string]string)
			}
			cache.statefulsets[actualNs][s.Name] = s.Spec.Template.Spec.Containers
			cache.labels[actualNs][s.Name] = s.Spec.Template.Labels
		}

		// Fetch DaemonSets
		daemonsets, err := c.clientset.AppsV1().DaemonSets(ns).List(ctx, metav1.ListOptions{})
		if err != nil {
			return nil, fmt.Errorf("failed to list daemonsets: %w", err)
		}
		for _, d := range daemonsets.Items {
			actualNs := d.Namespace
			if cache.daemonsets[actualNs] == nil {
				cache.daemonsets[actualNs] = make(map[string][]corev1.Container)
			}
			if cache.labels[actualNs] == nil {
				cache.labels[actualNs] = make(map[string]map[string]string)
			}
			cache.daemonsets[actualNs][d.Name] = d.Spec.Template.Spec.Containers
			cache.labels[actualNs][d.Name] = d.Spec.Template.Labels
		}

		// Fetch CronJobs
		cronjobs, err := c.clientset.BatchV1().CronJobs(ns).List(ctx, metav1.ListOptions{})
		if err != nil {
			return nil, fmt.Errorf("failed to list cronjobs: %w", err)
		}
		for _, cj := range cronjobs.Items {
			actualNs := cj.Namespace
			if cache.cronjobs[actualNs] == nil {
				cache.cronjobs[actualNs] = make(map[string][]corev1.Container)
			}
			if cache.labels[actualNs] == nil {
				cache.labels[actualNs] = make(map[string]map[string]string)
			}
			cache.cronjobs[actualNs][cj.Name] = cj.Spec.JobTemplate.Spec.Template.Spec.Containers
			cache.labels[actualNs][cj.Name] = cj.Spec.JobTemplate.Spec.Template.Labels
		}

		// Fetch Jobs
		jobs, err := c.clientset.BatchV1().Jobs(ns).List(ctx, metav1.ListOptions{})
		if err != nil {
			return nil, fmt.Errorf("failed to list jobs: %w", err)
		}
		for _, j := range jobs.Items {
			actualNs := j.Namespace
			if cache.jobs[actualNs] == nil {
				cache.jobs[actualNs] = make(map[string][]corev1.Container)
			}
			if cache.labels[actualNs] == nil {
				cache.labels[actualNs] = make(map[string]map[string]string)
			}
			cache.jobs[actualNs][j.Name] = j.Spec.Template.Spec.Containers
			cache.labels[actualNs][j.Name] = j.Spec.Template.Labels
		}
	}

	return cache, nil
}

// GetWorkloadContainersFromCache returns container specs from cache.
func (cache *WorkloadCache) GetWorkloadContainers(namespace string, targetRef types.TargetRef) ([]corev1.Container, map[string]string, error) {
	var containers []corev1.Container
	var labels map[string]string

	switch targetRef.Kind {
	case "Deployment":
		if nsCache, ok := cache.deployments[namespace]; ok {
			containers = nsCache[targetRef.Name]
			labels = cache.labels[namespace][targetRef.Name]
		}
	case "StatefulSet":
		if nsCache, ok := cache.statefulsets[namespace]; ok {
			containers = nsCache[targetRef.Name]
			labels = cache.labels[namespace][targetRef.Name]
		}
	case "DaemonSet":
		if nsCache, ok := cache.daemonsets[namespace]; ok {
			containers = nsCache[targetRef.Name]
			labels = cache.labels[namespace][targetRef.Name]
		}
	case "CronJob":
		if nsCache, ok := cache.cronjobs[namespace]; ok {
			containers = nsCache[targetRef.Name]
			labels = cache.labels[namespace][targetRef.Name]
		}
	case "Job":
		if nsCache, ok := cache.jobs[namespace]; ok {
			containers = nsCache[targetRef.Name]
			labels = cache.labels[namespace][targetRef.Name]
		}
	default:
		return nil, nil, fmt.Errorf("unsupported workload kind: %s", targetRef.Kind)
	}

	if containers == nil {
		return nil, nil, fmt.Errorf("%s %s not found in namespace %s", targetRef.Kind, targetRef.Name, namespace)
	}

	return containers, labels, nil
}

// GetContainerResourcesFromSpec extracts current resource requests and limits from a container spec.
func GetContainerResourcesFromSpec(container corev1.Container) (cpuLimit, memoryLimit, cpuRequest, memoryRequest types.ResourceQuantity) {
	if mem, ok := container.Resources.Limits[corev1.ResourceMemory]; ok {
		memoryLimit = types.ResourceQuantity{
			Value: float64(mem.Value()) / 1000000, // Convert bytes to MB
			Unit:  "M",
		}
	}
	if mem, ok := container.Resources.Requests[corev1.ResourceMemory]; ok {
		memoryRequest = types.ResourceQuantity{
			Value: float64(mem.Value()) / 1000000, // Convert bytes to MB
			Unit:  "M",
		}
	}
	return
}
