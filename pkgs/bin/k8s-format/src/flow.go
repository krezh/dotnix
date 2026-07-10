package main

import "strings"

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
