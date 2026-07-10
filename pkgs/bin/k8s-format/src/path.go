package main

import (
	"os"
	"path/filepath"
	"strings"

	"gopkg.in/yaml.v3"
)

// findRepoRoot walks up from dir until it finds the root of a version-control
// repository, identified by a .git (Git) or .jj (Jujutsu) directory. Returns dir
// itself if neither is found in any ancestor.
func findRepoRoot(dir string) string {
	abs, err := filepath.Abs(dir)
	if err != nil {
		return dir
	}
	current := abs
	for {
		if _, err := os.Stat(filepath.Join(current, ".git")); err == nil {
			return current
		}
		if _, err := os.Stat(filepath.Join(current, ".jj")); err == nil {
			return current
		}
		parent := filepath.Dir(current)
		if parent == current {
			return abs // reached filesystem root, give up
		}
		current = parent
	}
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

	// Use the repo root as the base for path derivation so the user can pass
	// any subdirectory as -p and still get correct paths.
	repoRoot := findRepoRoot(scanRoot)
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
