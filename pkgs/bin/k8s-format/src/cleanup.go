package main

import (
	"strings"

	"gopkg.in/yaml.v3"
)

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
