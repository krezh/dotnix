package main

import (
	"strings"

	"gopkg.in/yaml.v3"
)

// removeConfiguredFields removes fields specified in fieldsToRemove configuration
func removeConfiguredFields(node *yaml.Node, kind string) bool {
	changed := false

	for _, fieldPath := range fieldsToRemove {
		// Check if this field applies to this kind
		parts := strings.Split(fieldPath, ":")
		var targetKind, path string

		if len(parts) == 2 {
			// Kind-specific: "HelmRelease:spec.maxHistory"
			targetKind = parts[0]
			path = parts[1]
			if targetKind != kind {
				continue // Skip if not for this kind
			}
		} else {
			// Global: "spec.maxHistory"
			path = fieldPath
		}

		// Parse the path (e.g., "spec.maxHistory" -> ["spec", "maxHistory"])
		pathParts := strings.Split(path, ".")
		if len(pathParts) == 0 {
			continue
		}

		// Remove the field
		if removeFieldByPath(node, pathParts) {
			changed = true
		}
	}

	return changed
}

// removeFieldByPath removes a field from a node using a path like ["spec", "maxHistory"]
func removeFieldByPath(node *yaml.Node, path []string) bool {
	if node == nil || len(path) == 0 {
		return false
	}

	if node.Kind != yaml.MappingNode {
		return false
	}

	// If we're at the last part of the path, remove it from this node
	if len(path) == 1 {
		fieldName := path[0]
		for i := 0; i < len(node.Content); i += 2 {
			if node.Content[i].Value == fieldName {
				// Remove both key and value
				node.Content = append(node.Content[:i], node.Content[i+2:]...)
				return true
			}
		}
		return false
	}

	// Otherwise, navigate to the next level
	firstPart := path[0]
	for i := 0; i < len(node.Content); i += 2 {
		if node.Content[i].Value == firstPart {
			// Found the field, recurse into it
			return removeFieldByPath(node.Content[i+1], path[1:])
		}
	}

	return false
}
