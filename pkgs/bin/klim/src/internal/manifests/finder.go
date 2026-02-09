package manifests

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	corev1 "k8s.io/api/core/v1"
)

const (
	AppInstanceLabel = "app.kubernetes.io/instance"
)

// ManifestLocator finds HelmRelease manifest files for pods.
type ManifestLocator struct {
	gitRepoPath string
}

// NewManifestLocator creates a new manifest locator.
func NewManifestLocator(gitRepoPath string) *ManifestLocator {
	return &ManifestLocator{
		gitRepoPath: gitRepoPath,
	}
}

// FindHelmRelease finds the HelmRelease YAML file for a pod.
func (m *ManifestLocator) FindHelmRelease(pod corev1.Pod) (string, error) {
	// Check if this pod has the app instance label
	helmReleaseName, hasInstance := pod.Labels[AppInstanceLabel]

	if !hasInstance {
		return "", fmt.Errorf("pod %s/%s does not have %s label", pod.Namespace, pod.Name, AppInstanceLabel)
	}

	// Use pod's namespace for the HelmRelease namespace
	helmReleaseNamespace := pod.Namespace

	// Search for helmrelease.yaml in the git repo
	matches, err := m.findFileRecursive(
		filepath.Join(m.gitRepoPath, "clusters"),
		helmReleaseNamespace,
		helmReleaseName,
	)
	if err != nil || len(matches) == 0 {
		return "", fmt.Errorf("helmrelease.yaml not found for %s/%s", helmReleaseNamespace, helmReleaseName)
	}

	// Check if it uses bjw-s chart by looking for the OCIRepository
	if !m.isBjwsChart(filepath.Dir(matches[0])) {
		return "", fmt.Errorf("HelmRelease %s/%s does not use bjw-s chart", helmReleaseNamespace, helmReleaseName)
	}

	// Return the first match
	return matches[0], nil
}

// isBjwsChart checks if the HelmRelease uses the bjw-s chart.
func (m *ManifestLocator) isBjwsChart(appDir string) bool {
	// Look for ocirepository.yaml in the same directory
	ociRepoPath := filepath.Join(appDir, "ocirepository.yaml")

	data, err := os.ReadFile(ociRepoPath)
	if err != nil {
		return false
	}

	// Check if it references bjw-s
	return strings.Contains(string(data), "bjw-s")
}

// findFileRecursive searches for helmrelease.yaml recursively.
func (m *ManifestLocator) findFileRecursive(rootDir, namespace, releaseName string) ([]string, error) {
	var matches []string

	err := filepath.Walk(rootDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil // Continue on errors
		}

		if !info.IsDir() && info.Name() == "helmrelease.yaml" {
			// Check if the path contains the namespace and release name
			if strings.Contains(path, "/"+namespace+"/") && strings.Contains(path, "/"+releaseName+"/") {
				matches = append(matches, path)
			}
		}

		return nil
	})

	return matches, err
}
