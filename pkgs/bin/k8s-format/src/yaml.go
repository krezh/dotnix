package main

import "gopkg.in/yaml.v3"

// getFieldNode returns the value node for a given field
func getFieldNode(node *yaml.Node, field string) *yaml.Node {
	if node.Kind != yaml.MappingNode {
		return nil
	}

	for i := 0; i < len(node.Content); i += 2 {
		if node.Content[i].Value == field {
			return node.Content[i+1]
		}
	}
	return nil
}

// getFieldValue returns the string value for a given field
func getFieldValue(node *yaml.Node, field string) string {
	valueNode := getFieldNode(node, field)
	if valueNode == nil {
		return ""
	}
	return valueNode.Value
}
