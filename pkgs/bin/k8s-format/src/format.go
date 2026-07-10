package main

import (
	"bytes"
	"fmt"
	"os"
	"slices"
	"strings"

	"gopkg.in/yaml.v3"
)

// injectSchemaOnly injects or updates the yaml-language-server schema comment in any
// YAML file that has apiVersion + kind, without reformatting or reordering fields.
func injectSchemaOnly(path string, stats *Stats, ociIndex map[string]string, scanRoot string) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("reading file: %w", err)
	}

	content := string(data)
	hasDocSep := strings.HasPrefix(content, "---\n") || strings.HasPrefix(content, "---\r\n")

	lines := strings.Split(content, "\n")
	commentIdx := 0
	if hasDocSep {
		commentIdx = 1
	}

	var schemaComment string
	if len(lines) > commentIdx && strings.HasPrefix(lines[commentIdx], "# yaml-language-server:") {
		schemaComment = lines[commentIdx]
	}

	// Strip doc separator and schema comment to get the parseable YAML body
	body := content
	if hasDocSep {
		body = strings.TrimPrefix(body, "---\n")
		body = strings.TrimPrefix(body, "---\r\n")
	}
	if schemaComment != "" {
		body = strings.Replace(body, schemaComment+"\n", "", 1)
	}

	var firstDoc yaml.Node
	if err := yaml.NewDecoder(bytes.NewReader([]byte(body))).Decode(&firstDoc); err != nil {
		return nil // not valid YAML or empty
	}
	var rootNode *yaml.Node
	if firstDoc.Kind == yaml.DocumentNode && len(firstDoc.Content) > 0 {
		rootNode = firstDoc.Content[0]
	} else {
		rootNode = &firstDoc
	}
	if rootNode == nil || rootNode.Kind != yaml.MappingNode {
		return nil
	}

	kind := getFieldValue(rootNode, "kind")
	apiVersion := getFieldValue(rootNode, "apiVersion")
	if kind == "" || apiVersion == "" {
		return nil
	}

	stats.ByKind[kind]++
	targetURL := getSchemaURL(kind, apiVersion)

	if kind == "HelmRelease" {
		if specNode := getFieldNode(rootNode, "spec"); isAppTemplate(specNode, ociIndex) {
			targetURL = appTemplateSchemaURLs[0]
		}
		currentURL := strings.TrimPrefix(schemaComment, "# yaml-language-server: $schema=")
		if slices.Contains(appTemplateSchemaURLs, currentURL) && !slices.Contains(appTemplateSchemaURLs, targetURL) {
			return nil
		}
	}

	if targetURL == "" {
		return nil
	}

	newComment := "# yaml-language-server: $schema=" + targetURL
	if newComment == schemaComment {
		return nil
	}

	action := "schema-added"
	if schemaComment != "" {
		action = "schema-updated"
	}

	var out bytes.Buffer
	if hasDocSep {
		out.WriteString("---\n")
	}
	out.WriteString(newComment)
	out.WriteString("\n")
	out.WriteString(body)

	if err := os.WriteFile(path, out.Bytes(), 0644); err != nil {
		return fmt.Errorf("writing file: %w", err)
	}

	fmt.Printf("  %s %s  %s\n", successIcon, bold.Render(relPath(path, scanRoot)), muted.Render(action))
	stats.Formatted++
	stats.ByKindFmt[kind]++
	return nil
}

func formatYAMLFile(path string, expectedKind string, stats *Stats, ociIndex map[string]string, scanRoot string) error {
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
	var reasons []string

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

		addReason := func(r string) {
			if !slices.Contains(reasons, r) {
				reasons = append(reasons, r)
			}
		}

		// Clean up multiline strings with extra whitespace in parentheses
		if cleanupMultilineStrings(rootNode) {
			anyChanged = true
			addReason("strings-cleaned")
		}

		// Remove specified fields
		if removeConfiguredFields(rootNode, kind) {
			anyChanged = true
			addReason("fields-removed")
		}

		// Handle KustomizationFile separately (native kustomization.yaml)
		if kind == "KustomizationFile" {
			// Reorder top-level fields for kustomization.yaml
			if rootOrdering, ok := ordering["root"]; ok {
				if reorderFields(rootNode, rootOrdering) {
					anyChanged = true
					addReason("fields-reordered")
				}
			}

			// Normalize resource paths to use ./
			if resourcesNode := getFieldNode(rootNode, "resources"); resourcesNode != nil {
				if normalizeResourcePaths(resourcesNode) {
					anyChanged = true
					addReason("paths-normalized")
				}
			}
		} else {
			// Handle Flux Kustomization resources
			// Reorder top-level fields (apiVersion, kind, metadata, spec)
			if reorderTopLevelFields(rootNode) {
				anyChanged = true
				addReason("fields-reordered")
			}

			// Reorder metadata fields
			if metadataNode := getFieldNode(rootNode, "metadata"); metadataNode != nil {
				if metadataOrdering, ok := ordering["metadata"]; ok {
					if reorderFields(metadataNode, metadataOrdering) {
						anyChanged = true
						addReason("fields-reordered")
					}
				}
			}

			// Reorder spec fields
			if specNode := getFieldNode(rootNode, "spec"); specNode != nil {
				if specOrdering, ok := ordering["spec"]; ok {
					if reorderFields(specNode, specOrdering) {
						anyChanged = true
						addReason("fields-reordered")
					}
				}

				// Normalize spec.path for Flux Kustomization resources
				if kind == "Kustomization" {
					if normalizeKsSpecPath(specNode, path, scanRoot) {
						anyChanged = true
						addReason("path-normalized")
					}
				}

				// Apply nested orderings dynamically
				if applyNestedOrderings(specNode, ordering) {
					anyChanged = true
					addReason("fields-reordered")
				}
			}
		}

		documents = append(documents, &doc)
	}

	// Ensure the schema comment is correct.
	// Files whose existing schema matches the ResourceType's SchemaURL override are left alone
	// (e.g. app-template HelmReleases with a bjw-s-labs schema). All others are updated.
	schemaChanged := false
	if len(documents) > 0 {
		var firstRoot *yaml.Node
		if documents[0].Kind == yaml.DocumentNode && len(documents[0].Content) > 0 {
			firstRoot = documents[0].Content[0]
		}
		if firstRoot != nil && firstRoot.Kind == yaml.MappingNode {
			actualKind := getFieldValue(firstRoot, "kind")
			apiVersion := getFieldValue(firstRoot, "apiVersion")
			if actualKind == "" {
				actualKind = expectedKind
			}

			targetURL := getSchemaURL(actualKind, apiVersion)

			// For HelmRelease, detect app-template via OCIRepository index
			if expectedKind == "HelmRelease" {
				if specNode := getFieldNode(firstRoot, "spec"); isAppTemplate(specNode, ociIndex) {
					targetURL = appTemplateSchemaURLs[0]
				}
			}

			currentURL := strings.TrimPrefix(schemaComment, "# yaml-language-server: $schema=")

			if expectedKind == "HelmRelease" && slices.Contains(appTemplateSchemaURLs, currentURL) && !slices.Contains(appTemplateSchemaURLs, targetURL) {
				// Safety net: preserve known app-template schema when detection had no OCI index.
			} else if targetURL != "" {
				newComment := "# yaml-language-server: $schema=" + targetURL
				if newComment != schemaComment {
					if schemaComment == "" {
						reasons = append(reasons, "schema-added")
					} else {
						reasons = append(reasons, "schema-updated")
					}
					schemaComment = newComment
					schemaChanged = true
				}
			}
		}
	}

	if !anyChanged && !schemaChanged {
		return nil // No changes needed
	}

	// Marshal all documents back to YAML with proper formatting
	var output bytes.Buffer

	// Add document separator if it was there
	if hasDocSeparator {
		output.WriteString("---\n")
	}

	// Add schema comment (existing or newly derived)
	if schemaComment != "" {
		output.WriteString(schemaComment)
		output.WriteString("\n")
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

	fmt.Printf("  %s %s  %s\n", successIcon, bold.Render(relPath(path, scanRoot)), muted.Render(strings.Join(reasons, ", ")))
	stats.Formatted++
	stats.ByKindFmt[kind]++

	return nil
}
