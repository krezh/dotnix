package main

import (
	"math/rand"
	"os"
	"path/filepath"
	"strings"
)

var imageExtensions = []string{
	".jpg", ".jpeg", ".png", ".bmp", ".gif", ".webp",
	".pnm", ".tga", ".tiff", ".farbfeld", ".svg",
}

// findImages recursively finds all supported image files in a directory
func findImages(directory string) []string {
	var images []string
	extMap := make(map[string]bool)
	for _, ext := range imageExtensions {
		extMap[ext] = true
	}

	filepath.Walk(directory, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}
		if extMap[strings.ToLower(filepath.Ext(path))] {
			images = append(images, path)
		}
		return nil
	})

	return images
}

// selectRandomImage selects one image uniformly at random from the provided slice.
func selectRandomImage(images []string) string {
	if len(images) == 0 {
		return ""
	}
	return images[rand.Intn(len(images))]
}

// shuffleImages performs an in-place Fisher-Yates shuffle of the image paths.
func shuffleImages(images []string) {
	rand.Shuffle(len(images), func(i, j int) {
		images[i], images[j] = images[j], images[i]
	})
}
