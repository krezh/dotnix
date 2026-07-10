package main

import (
	"flag"
	"fmt"
	"os"
	"path/filepath"
)

// Stats tracks formatting results across the scanned tree.
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
			if skipDir(info.Name()) {
				return filepath.SkipDir
			}
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
