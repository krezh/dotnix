package main

import (
	"bytes"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"gopkg.in/yaml.v3"
)

// ResourceType defines formatting rules for a Kubernetes resource type
type ResourceType struct {
	Kind      string              // The Kubernetes Kind
	Filenames []string            // Filenames that contain this resource type
	Ordering  map[string][]string // Field ordering rules (metadata, spec, values, etc.)
}

// Supported resource types - add new types here
var resourceTypes = []ResourceType{
	{
		Kind:      "HelmRelease",
		Filenames: []string{"helmrelease.yaml"},
		Ordering: map[string][]string{
			"spec":     {"interval", "chartRef", "chart", "driftDetection", "install", "upgrade", "uninstall", "dependsOn", "timeout", "maxHistory", "valuesFrom", "values", "postRenderers"},
			"metadata": {"name", "namespace", "labels", "annotations"},
			"values":   {"defaultPodOptions", "controllers", "service", "serviceAccount", "ingress", "route", "persistence", "configMaps", "secrets"},
		},
	},
	{
		Kind:      "OCIRepository",
		Filenames: []string{"ocirepository.yaml"},
		Ordering: map[string][]string{
			"spec":     {"interval", "layerSelector", "ref", "url", "insecure", "provider", "timeout"},
			"metadata": {"name", "namespace", "labels", "annotations"},
		},
	},
	{
		Kind:      "Kustomization",
		Filenames: []string{"ks.yaml"},
		Ordering: map[string][]string{
			"spec":     {"targetNamespace", "commonMetadata", "dependsOn", "path", "prune", "sourceRef", "wait", "interval", "retryInterval", "timeout", "force", "components", "postBuild", "patches", "images"},
			"metadata": {"name", "namespace", "labels", "annotations"},
		},
	},
	{
		Kind:      "KustomizationFile",
		Filenames: []string{"kustomization.yaml"},
		Ordering: map[string][]string{
			"root": {"apiVersion", "kind", "resources"},
		},
	},
	{
		Kind:      "ExternalSecret",
		Filenames: []string{"externalsecret.yaml"},
		Ordering: map[string][]string{
			"spec":     {"refreshInterval", "secretStoreRef", "target", "data", "dataFrom"},
			"metadata": {"name", "namespace", "labels", "annotations"},
		},
	},
}

// Build lookup maps from resourceTypes
var (
	fieldOrdering map[string]map[string][]string
	fileKindMap   map[string]string
)

// Root-level fields in a Kubernetes resource (directly under apiVersion/kind)
// Any ordering key NOT in this list is treated as a nested field
var rootLevelFields = map[string]bool{
	"metadata":   true,
	"spec":       true,
	"data":       true,
	"stringData": true,
	"status":     true,
}

// Fields to remove from resources (format: "field.path" or "kind:field.path")
// Examples: "spec.maxHistory", "HelmRelease:spec.uninstall"
var fieldsToRemove = []string{
	"HelmRelease:spec.maxHistory",
	"HelmRelease:spec.uninstall",
}

func init() {
	fieldOrdering = make(map[string]map[string][]string)
	fileKindMap = make(map[string]string)

	for _, rt := range resourceTypes {
		fieldOrdering[rt.Kind] = rt.Ordering
		for _, filename := range rt.Filenames {
			fileKindMap[filename] = rt.Kind
		}
	}
}

type Stats struct {
	Total     int
	Formatted int
	Errors    int
	ByKind    map[string]int
	ByKindFmt map[string]int
}

func main() {
	// Parse command-line arguments
	pathFlag := flag.String("path", ".", "Path to directory to format")
	flag.StringVar(pathFlag, "p", ".", "Path to directory to format (shorthand)")
	flag.Parse()

	stats := Stats{
		ByKind:    make(map[string]int),
		ByKindFmt: make(map[string]int),
	}

	// Walk through only relevant files
	err := filepath.Walk(*pathFlag, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		// Skip directories
		if info.IsDir() {
			return nil
		}

		// Check if this filename is one we care about
		baseName := info.Name()
		expectedKind, shouldProcess := fileKindMap[baseName]
		if !shouldProcess {
			return nil
		}

		// Check if we have formatting rules for this kind
		_, hasOrdering := fieldOrdering[expectedKind]
		if !hasOrdering {
			return nil
		}

		stats.Total++
		if err := formatYAMLFile(path, expectedKind, &stats); err != nil {
			fmt.Printf("Error formatting %s: %v\n", path, err)
			stats.Errors++
		}

		return nil
	})

	if err != nil {
		fmt.Fprintf(os.Stderr, "Error walking directory: %v\n", err)
		os.Exit(1)
	}

	// Print statistics
	fmt.Println("\n=== Kubernetes YAML Formatting Statistics ===")
	fmt.Printf("Total YAML files: %d\n", stats.Total)
	fmt.Printf("Formatted: %d\n", stats.Formatted)
	fmt.Printf("Errors: %d\n", stats.Errors)
	fmt.Println("\nBy Resource Kind:")
	for kind, count := range stats.ByKind {
		formatted := stats.ByKindFmt[kind]
		fmt.Printf("  %s: %d total, %d formatted\n", kind, count, formatted)
	}
}

func formatYAMLFile(path string, expectedKind string, stats *Stats) error {
	// Read file
	data, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("reading file: %w", err)
	}

	originalContent := string(data)

	// Check if file starts with ---
	hasDocSeparator := strings.HasPrefix(originalContent, "---\n") || strings.HasPrefix(originalContent, "---\r\n")

	// Extract schema comment if present
	var schemaComment string
	lines := strings.Split(originalContent, "\n")
	commentStartIdx := 0
	if hasDocSeparator {
		commentStartIdx = 1
	}

	if len(lines) > commentStartIdx && strings.HasPrefix(lines[commentStartIdx], "# yaml-language-server:") {
		schemaComment = lines[commentStartIdx]
	}

	// Strip schema comment and doc separator for parsing
	yamlContent := originalContent
	if hasDocSeparator {
		yamlContent = strings.TrimPrefix(yamlContent, "---\n")
		yamlContent = strings.TrimPrefix(yamlContent, "---\r\n")
	}
	if schemaComment != "" {
		yamlContent = strings.Replace(yamlContent, schemaComment+"\n", "", 1)
	}

	// Parse all YAML documents in the file
	decoder := yaml.NewDecoder(bytes.NewReader([]byte(yamlContent)))
	var documents []*yaml.Node
	var anyChanged bool

	for {
		var doc yaml.Node
		if err := decoder.Decode(&doc); err != nil {
			if err.Error() == "EOF" {
				break
			}
			return fmt.Errorf("parsing yaml: %w", err)
		}

		// Get the document content
		var rootNode *yaml.Node
		if doc.Kind == yaml.DocumentNode && len(doc.Content) > 0 {
			rootNode = doc.Content[0]
		} else {
			rootNode = &doc
		}

		if rootNode.Kind != yaml.MappingNode {
			documents = append(documents, &doc)
			continue
		}

		// Verify the kind matches what we expect
		kind := getFieldValue(rootNode, "kind")
		if kind != expectedKind && expectedKind != "KustomizationFile" {
			documents = append(documents, &doc)
			continue
		}

		// For kustomization.yaml files, treat them as KustomizationFile
		if expectedKind == "KustomizationFile" {
			kind = "KustomizationFile"
		}

		stats.ByKind[kind]++

		// Get ordering rules
		ordering := fieldOrdering[kind]

		// Track if we made changes to this document
		changed := false

		// Clean up multiline strings with extra whitespace in parentheses
		if cleanupMultilineStrings(rootNode) {
			changed = true
		}

		// Remove specified fields
		if removeConfiguredFields(rootNode, kind) {
			changed = true
		}

		// Handle KustomizationFile separately (native kustomization.yaml)
		if kind == "KustomizationFile" {
			// Reorder top-level fields for kustomization.yaml
			if rootOrdering, ok := ordering["root"]; ok {
				if reorderFields(rootNode, rootOrdering) {
					changed = true
				}
			}

			// Normalize resource paths to use ./
			if resourcesNode := getFieldNode(rootNode, "resources"); resourcesNode != nil {
				if normalizeResourcePaths(resourcesNode) {
					changed = true
				}
			}
		} else {
			// Handle Flux Kustomization resources
			// Reorder top-level fields (apiVersion, kind, metadata, spec)
			if reorderTopLevelFields(rootNode) {
				changed = true
			}

			// Reorder metadata fields
			if metadataNode := getFieldNode(rootNode, "metadata"); metadataNode != nil {
				if metadataOrdering, ok := ordering["metadata"]; ok {
					if reorderFields(metadataNode, metadataOrdering) {
						changed = true
					}
				}
			}

			// Reorder spec fields
			if specNode := getFieldNode(rootNode, "spec"); specNode != nil {
				if specOrdering, ok := ordering["spec"]; ok {
					if reorderFields(specNode, specOrdering) {
						changed = true
					}
				}

				// Apply nested orderings dynamically
				if changed := applyNestedOrderings(kind, "spec", specNode, ordering); changed {
					changed = true
				}
			}
		}

		if changed {
			anyChanged = true
			stats.ByKindFmt[kind]++
		}

		documents = append(documents, &doc)
	}

	if !anyChanged {
		return nil // No changes needed
	}

	// Marshal all documents back to YAML with proper formatting
	var output bytes.Buffer

	// Add document separator if it was there
	if hasDocSeparator {
		output.WriteString("---\n")
	}

	// Add schema comment if it was there
	if schemaComment != "" {
		output.WriteString(schemaComment + "\n")
	}

	// Encode all documents
	encoder := yaml.NewEncoder(&output)
	encoder.SetIndent(2) // Use 2-space indentation

	for i, doc := range documents {
		if err := encoder.Encode(doc); err != nil {
			return fmt.Errorf("marshaling yaml document %d: %w", i, err)
		}
	}
	encoder.Close()

	// Get the output and ensure it ends with a single newline
	finalOutput := output.String()
	finalOutput = strings.TrimRight(finalOutput, "\n") + "\n"

	// Add spaces inside {} for flow-style mappings
	finalOutput = addSpacesInFlowMappings(finalOutput)

	// Write file
	if err := os.WriteFile(path, []byte(finalOutput), 0644); err != nil {
		return fmt.Errorf("writing file: %w", err)
	}

	// Get kind for logging
	kind := expectedKind
	if len(documents) > 0 && documents[0].Kind == yaml.DocumentNode && len(documents[0].Content) > 0 {
		kind = getFieldValue(documents[0].Content[0], "kind")
	}

	fmt.Printf("Formatted: %s (%s)\n", path, kind)
	stats.Formatted++

	return nil
}

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

// applyNestedOrderings applies nested field orderings dynamically
// It auto-detects nested fields by finding ordering keys that aren't top-level fields
func applyNestedOrderings(kind, parentPath string, parentNode *yaml.Node, ordering map[string][]string) bool {
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

// cleanupMultilineStrings removes blank lines after opening parentheses in multiline strings
func cleanupMultilineStrings(node *yaml.Node) bool {
	if node == nil {
		return false
	}

	changed := false

	// Process scalar nodes (string values)
	if node.Kind == yaml.ScalarNode && node.Value != "" {
		// Check if the string contains parentheses with double newlines (blank line)
		if strings.Contains(node.Value, "(\n\n") || strings.Contains(node.Value, "(\r\n\r\n") {
			original := node.Value
			// Remove blank lines after opening parenthesis
			cleaned := strings.ReplaceAll(original, "(\n\n", "(\n")
			cleaned = strings.ReplaceAll(cleaned, "(\r\n\r\n", "(\r\n")
			// Also remove blank lines before closing parenthesis
			cleaned = strings.ReplaceAll(cleaned, "\n\n)", "\n)")
			cleaned = strings.ReplaceAll(cleaned, "\r\n\r\n)", "\r\n)")

			if cleaned != original {
				node.Value = cleaned
				// Use literal style (|) to preserve exact formatting
				node.Style = yaml.LiteralStyle
				changed = true
			}
		}
	}

	// Recursively process child nodes
	for _, child := range node.Content {
		if cleanupMultilineStrings(child) {
			changed = true
		}
	}

	return changed
}

// cleanupParenthesesInString cleans up whitespace/newlines inside parentheses in a string
func cleanupParenthesesInString(s string) string {
	// Pattern: content before ( + whitespace/newlines + content + whitespace/newlines + )
	var result strings.Builder
	i := 0

	for i < len(s) {
		if s[i] == '(' {
			// Found opening parenthesis
			result.WriteByte('(')
			i++

			// Collect content until closing parenthesis
			var parenContent []string
			currentWord := ""

			for i < len(s) && s[i] != ')' {
				ch := s[i]
				if ch == '\n' || ch == '\r' || ch == ' ' || ch == '\t' {
					if currentWord != "" {
						parenContent = append(parenContent, currentWord)
						currentWord = ""
					}
					i++
				} else {
					currentWord += string(ch)
					i++
				}
			}

			if currentWord != "" {
				parenContent = append(parenContent, currentWord)
			}

			// Write cleaned content
			if len(parenContent) > 0 {
				result.WriteString(" " + strings.Join(parenContent, " ") + " ")
			}

			// Write closing parenthesis
			if i < len(s) && s[i] == ')' {
				result.WriteByte(')')
				i++
			}
		} else {
			result.WriteByte(s[i])
			i++
		}
	}

	return result.String()
}

// normalizeResourcePaths ensures all resource paths start with ./
func normalizeResourcePaths(node *yaml.Node) bool {
	if node.Kind != yaml.SequenceNode {
		return false
	}

	changed := false
	for _, item := range node.Content {
		if item.Kind == yaml.ScalarNode && item.Value != "" {
			// Check if it's a local file reference (not a URL or absolute path)
			if !strings.HasPrefix(item.Value, "http://") &&
				!strings.HasPrefix(item.Value, "https://") &&
				!strings.HasPrefix(item.Value, "/") {

				// Add ./ prefix only if not already starting with ./ or ../
				if !strings.HasPrefix(item.Value, "./") && !strings.HasPrefix(item.Value, "../") {
					item.Value = "./" + item.Value
					changed = true
				}
			}
		}
	}

	return changed
}

// cleanupParentheses removes extra whitespace and newlines inside parentheses
func cleanupParentheses(content string) string {
	lines := strings.Split(content, "\n")
	var result []string

	i := 0
	for i < len(lines) {
		line := lines[i]

		// Check if line ends with "in (" or similar pattern with opening parenthesis
		trimmed := strings.TrimSpace(line)
		if strings.HasSuffix(trimmed, "(") {
			// Look ahead to collect content until closing parenthesis
			parenContent := []string{line}
			i++
			foundClosing := false

			for i < len(lines) {
				nextLine := lines[i]
				parenContent = append(parenContent, nextLine)

				if strings.TrimSpace(nextLine) == ")" {
					foundClosing = true
					i++
					break
				}
				i++
			}

			// If we found a closing paren, reconstruct as single line
			if foundClosing && len(parenContent) > 2 {
				// Extract the value between parentheses (skip first and last lines)
				var values []string
				for j := 1; j < len(parenContent)-1; j++ {
					val := strings.TrimSpace(parenContent[j])
					if val != "" {
						values = append(values, val)
					}
				}

				// Get indentation from first line
				indent := ""
				for _, ch := range parenContent[0] {
					if ch == ' ' || ch == '\t' {
						indent += string(ch)
					} else {
						break
					}
				}

				// Reconstruct as single line: "key: value ( content )"
				firstLine := parenContent[0]
				if len(values) > 0 {
					result = append(result, firstLine[:len(firstLine)-1]+"( "+strings.Join(values, " ")+" )")
				} else {
					result = append(result, firstLine+")")
				}
			} else {
				// Keep as-is if pattern doesn't match
				result = append(result, parenContent...)
			}
		} else {
			result = append(result, line)
			i++
		}
	}

	return strings.Join(result, "\n")
}

// addSpacesInFlowMappings adds spaces inside flow-style mappings { key: value }
// but preserves {{ }} for Helm templates and ${} for variable substitutions
func addSpacesInFlowMappings(content string) string {
	lines := strings.Split(content, "\n")

	for i, line := range lines {
		// Skip lines that are likely string values (contain quotes or are comments)
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, "#") {
			continue
		}

		// Only process lines with flow-style mappings (contain : inside {})
		if strings.Contains(line, "{") && strings.Contains(line, "}") && strings.Contains(line, ":") {
			// Check if it's a flow mapping by looking for key:value pattern inside {}
			inMapping := false
			var result strings.Builder

			for j := 0; j < len(line); j++ {
				char := line[j]

				if char == '{' && j+1 < len(line) && line[j+1] != '{' {
					// Check if this is a variable substitution ${...}
					isVarSubst := j > 0 && line[j-1] == '$'

					// Start of flow mapping (not {{ template and not ${var})
					result.WriteByte(char)
					if !isVarSubst && j+1 < len(line) && line[j+1] != ' ' && line[j+1] != '}' {
						result.WriteByte(' ')
					}
					if !isVarSubst {
						inMapping = true
					}
				} else if char == '}' && j > 0 && line[j-1] != '}' {
					// Check if we're closing a variable substitution
					isVarSubst := false
					for k := j - 1; k >= 0; k-- {
						if line[k] == '{' && k > 0 && line[k-1] == '$' {
							isVarSubst = true
							break
						}
						if line[k] == ' ' || line[k] == ':' {
							break
						}
					}

					// End of flow mapping (not }} template and not ${var})
					if inMapping && !isVarSubst && j > 0 && line[j-1] != ' ' && line[j-1] != '{' {
						result.WriteByte(' ')
					}
					result.WriteByte(char)
					if inMapping && !isVarSubst {
						inMapping = false
					}
				} else {
					result.WriteByte(char)
				}
			}

			lines[i] = result.String()
		}
	}

	return strings.Join(lines, "\n")
}
