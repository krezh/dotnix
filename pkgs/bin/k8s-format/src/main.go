package main

import (
	"bytes"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"slices"
	"strings"

	"gopkg.in/yaml.v3"
)

// getSchemaURL auto-derives the yaml-language-server schema URL from apiVersion and kind.
// Returns "" if the kind is excluded or the apiVersion is not in the expected format.
func getSchemaURL(actualKind, apiVersion string) string {
	key := apiVersion + " " + actualKind
	if schemaExclusions[apiVersion] || schemaExclusions[key] {
		return ""
	}
	if url, ok := schemaOverrides[key]; ok {
		return url
	}
	parts := strings.SplitN(apiVersion, "/", 2)
	if len(parts) != 2 || parts[0] == "" || parts[1] == "" {
		return ""
	}
	group, version := parts[0], parts[1]
	return fmt.Sprintf("%s/%s/%s_%s.json",
		schemaBaseURL, group, strings.ToLower(actualKind), version)
}

// findGitRoot walks up from dir until it finds a directory containing .git.
// Returns dir itself if no .git ancestor is found.
func findGitRoot(dir string) string {
	abs, err := filepath.Abs(dir)
	if err != nil {
		return dir
	}
	current := abs
	for {
		if _, err := os.Stat(filepath.Join(current, ".git")); err == nil {
			return current
		}
		parent := filepath.Dir(current)
		if parent == current {
			return abs // reached filesystem root, give up
		}
		current = parent
	}
}

// buildOCIRepoIndex walks rootPath and maps every OCIRepository metadata.name → spec.url.
func buildOCIRepoIndex(rootPath string) map[string]string {
	index := make(map[string]string)
	filepath.Walk(rootPath, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() || info.Name() != "ocirepository.yaml" {
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
			url := getFieldValue(specNode, "url")
			if name != "" && url != "" {
				index[name] = url
			}
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
		if url, ok := ociIndex[name]; ok && strings.HasSuffix(url, "/app-template") {
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

	ociIndex := buildOCIRepoIndex(*pathFlag)

	// Walk all YAML files: full format for known kinds, schema-only for everything else
	err := filepath.Walk(*pathFlag, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if info.IsDir() {
			return nil
		}
		baseName := info.Name()
		ext := filepath.Ext(baseName)
		if ext != ".yaml" && ext != ".yml" {
			return nil
		}

		stats.Total++
		expectedKind, inKindMap := fileKindMap[baseName]
		_, hasOrdering := fieldOrdering[expectedKind]

		scanRoot := *pathFlag
		if inKindMap && hasOrdering {
			if err := formatYAMLFile(path, expectedKind, &stats, ociIndex, scanRoot); err != nil {
				fmt.Fprintf(os.Stderr, "  %s %s  %s\n", errorIcon, relPath(path, scanRoot), muted.Render(err.Error()))
				stats.Errors++
			}
		} else {
			if err := injectSchemaOnly(path, &stats, ociIndex, scanRoot); err != nil {
				fmt.Fprintf(os.Stderr, "  %s %s  %s\n", errorIcon, relPath(path, scanRoot), muted.Render(err.Error()))
				stats.Errors++
			}
		}

		return nil
	})

	if err != nil {
		fmt.Fprintf(os.Stderr, "  %s %s\n", errorIcon, red.Render(err.Error()))
		os.Exit(1)
	}

	scanned := muted.Render(fmt.Sprintf("%d scanned", stats.Total))
	fmtd := accent.Render(fmt.Sprintf("%d fixed", stats.Formatted))
	if stats.Errors > 0 {
		errs := red.Render(fmt.Sprintf("%d error(s)", stats.Errors))
		fmt.Printf("\n  %s · %s · %s\n\n", scanned, fmtd, errs)
	} else {
		fmt.Printf("\n  %s · %s\n\n", scanned, fmtd)
	}
}

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
				result.WriteString(" ")
				result.WriteString(strings.Join(parenContent, " "))
				result.WriteString(" ")
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

// normalizeKsSpecPath ensures spec.path in a Flux Kustomization is correct.
// Without scanRoot it only adds a "./" prefix. With scanRoot it derives the
// canonical path as "./" + (ks.yaml parent relative to scanRoot) + "/" + (last
// segment of the current path), enforcing that paths always anchor to the file's
// own location in the repository.
func normalizeKsSpecPath(specNode *yaml.Node, ksFilePath, scanRoot string) bool {
	pathNode := getFieldNode(specNode, "path")
	if pathNode == nil || pathNode.Kind != yaml.ScalarNode || pathNode.Value == "" {
		return false
	}
	val := pathNode.Value

	if strings.HasPrefix(val, "http") || strings.HasPrefix(val, "/") || strings.HasPrefix(val, "../") {
		return false
	}

	if scanRoot == "" {
		if strings.HasPrefix(val, "./") {
			return false
		}
		pathNode.Value = "./" + val
		return true
	}

	// Use the git root as the base for path derivation so the user can pass
	// any subdirectory as -p and still get correct paths.
	repoRoot := findGitRoot(scanRoot)
	ksDir := filepath.Dir(ksFilePath)
	ksDirRel, err := filepath.Rel(repoRoot, ksDir)
	if err != nil || ksDirRel == "." {
		// Can't derive a meaningful base path — just ensure "./" prefix
		if strings.HasPrefix(val, "./") {
			return false
		}
		pathNode.Value = "./" + val
		return true
	}
	ksDirRel = filepath.ToSlash(ksDirRel)

	// Derive: "./" + ksDirRel + "/" + last segment of the current path
	stripped := strings.TrimPrefix(strings.TrimPrefix(val, "./"), "../")
	parts := strings.Split(stripped, "/")
	subDir := parts[len(parts)-1]
	if subDir == "" || subDir == "." {
		// Path has no usable segment — just ensure "./" prefix
		if strings.HasPrefix(val, "./") {
			return false
		}
		pathNode.Value = "./" + val
		return true
	}

	derived := "./" + ksDirRel + "/" + subDir
	if derived == val {
		return false
	}
	pathNode.Value = derived
	return true
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
