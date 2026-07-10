package main

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"

	"gopkg.in/yaml.v3"
)

// skipDir reports whether a directory entry should be excluded from scans,
// covering version-control metadata and vendored/dependency trees that may
// contain duplicate or mutated manifests (e.g. .terraform copies).
func skipDir(name string) bool {
	switch name {
	case ".git", ".jj", ".terraform", "node_modules", "vendor":
		return true
	}
	return false
}

// buildOCIRepoIndex walks rootPath and maps every OCIRepository
// "namespace/name" → spec.url. A bare name → url entry is also kept as a
// fallback for chartRefs that omit the namespace, so lookups stay correct even
// when namespaces aren't spelled out.
func buildOCIRepoIndex(rootPath string) map[string]string {
	index := make(map[string]string)
	filepath.Walk(rootPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil
		}
		if info.IsDir() {
			if skipDir(info.Name()) {
				return filepath.SkipDir
			}
			return nil
		}
		if info.Name() != "ocirepository.yaml" {
			return nil
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return nil
		}
		decoder := yaml.NewDecoder(bytes.NewReader(data))
		for {
			var doc yaml.Node
			if err := decoder.Decode(&doc); err != nil {
				break
			}
			root := &doc
			if doc.Kind == yaml.DocumentNode && len(doc.Content) > 0 {
				root = doc.Content[0]
			}
			if root.Kind != yaml.MappingNode || getFieldValue(root, "kind") != "OCIRepository" {
				continue
			}
			metaNode := getFieldNode(root, "metadata")
			specNode := getFieldNode(root, "spec")
			if metaNode == nil || specNode == nil {
				continue
			}
			name := getFieldValue(metaNode, "name")
			ns := getFieldValue(metaNode, "namespace")
			url := getFieldValue(specNode, "url")
			if name == "" || url == "" {
				continue
			}
			if ns != "" {
				index[ns+"/"+name] = url
			}
			index[name] = url
		}
		return nil
	})
	return index
}

// isAppTemplate returns true when a HelmRelease spec references an app-template chart,
// either via chartRef → OCIRepository URL ending in /app-template, or via
// chart.spec.chart == "app-template" for HelmRepository-based releases.
func isAppTemplate(specNode *yaml.Node, ociIndex map[string]string) bool {
	if specNode == nil {
		return false
	}
	if chartRefNode := getFieldNode(specNode, "chartRef"); chartRefNode != nil {
		name := getFieldValue(chartRefNode, "name")
		ns := getFieldValue(chartRefNode, "namespace")
		key := name
		if ns != "" {
			key = ns + "/" + name
		}
		url, ok := ociIndex[key]
		if !ok {
			url, ok = ociIndex[name]
		}
		if ok && strings.HasSuffix(url, "/app-template") {
			return true
		}
		return false
	}
	if chartNode := getFieldNode(specNode, "chart"); chartNode != nil {
		if chartSpecNode := getFieldNode(chartNode, "spec"); chartSpecNode != nil {
			return getFieldValue(chartSpecNode, "chart") == "app-template"
		}
	}
	return false
}
