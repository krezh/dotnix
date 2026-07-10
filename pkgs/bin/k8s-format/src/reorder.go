package main

import "gopkg.in/yaml.v3"

// reorderTopLevelFields ensures apiVersion, kind, metadata, spec order
func reorderTopLevelFields(node *yaml.Node) bool {
	if node.Kind != yaml.MappingNode {
		return false
	}

	topLevelOrder := []string{"apiVersion", "kind", "metadata", "spec", "data", "stringData"}
	return reorderFields(node, topLevelOrder)
}

// reorderFields reorders fields according to the given order
func reorderFields(node *yaml.Node, order []string) bool {
	if node.Kind != yaml.MappingNode {
		return false
	}

	// Build a map of field positions
	fieldMap := make(map[string]int)
	for i := 0; i < len(node.Content); i += 2 {
		fieldMap[node.Content[i].Value] = i
	}

	// Check if already in order
	alreadyOrdered := true
	lastIdx := -1
	for _, field := range order {
		if idx, exists := fieldMap[field]; exists {
			if idx < lastIdx {
				alreadyOrdered = false
				break
			}
			lastIdx = idx
		}
	}

	if alreadyOrdered {
		return false
	}

	// Build new content array
	newContent := make([]*yaml.Node, 0, len(node.Content))
	used := make(map[int]bool)

	// Add fields in the specified order
	for _, field := range order {
		if idx, exists := fieldMap[field]; exists {
			newContent = append(newContent, node.Content[idx], node.Content[idx+1])
			used[idx] = true
		}
	}

	// Add remaining fields (not in order list) at the end
	for i := 0; i < len(node.Content); i += 2 {
		if !used[i] {
			newContent = append(newContent, node.Content[i], node.Content[i+1])
		}
	}

	node.Content = newContent
	return true
}

// applyNestedOrderings applies nested field orderings dynamically
// It auto-detects nested fields by finding ordering keys that aren't top-level fields
func applyNestedOrderings(parentNode *yaml.Node, ordering map[string][]string) bool {
	changed := false

	// Find all ordering keys that aren't root-level fields - these are nested
	for orderingKey, orderingRules := range ordering {
		if rootLevelFields[orderingKey] {
			continue // Skip root-level fields (metadata, spec, data, etc.)
		}

		// This is a nested field - try to find and apply it
		nestedNode := getFieldNode(parentNode, orderingKey)
		if nestedNode != nil {
			if reorderFields(nestedNode, orderingRules) {
				changed = true
			}
		}
	}

	return changed
}
