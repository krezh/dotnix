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
					Value: float64(mem.Value()) / (1024 * 1024),
					Unit:  "Mi",
				}
			}
			if mem, ok := container.Resources.Requests[corev1.ResourceMemory]; ok {
				memoryRequest = types.ResourceQuantity{
					Value: float64(mem.Value()) / (1024 * 1024),
					Unit:  "Mi",
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
			// Strip the replica set hash suffix
			name := owner.Name
			if len(name) > 10 {
				return name[:len(name)-10]
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
