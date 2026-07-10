package main

import (
	"fmt"
	"strings"
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
