package main

import (
	"context"
	"fmt"
	"time"
)

// startSlideshow runs the main slideshow loop
func startSlideshow(ctx context.Context, directory string, intervalSeconds int, options TransitionOptions) {
	interval := time.Duration(intervalSeconds) * time.Second

	for {
		images := findImages(directory)
		if len(images) == 0 {
			fmt.Printf("Warning: No images found, retrying in %d seconds...\n", intervalSeconds)
			if !sleep(ctx, interval) {
				return
			}
			continue
		}

		shuffleImages(images)

		for _, img := range images {
			select {
			case <-ctx.Done():
				return
			default:
				displayImage(img, options)
				if !sleep(ctx, interval) {
					return
				}
			}
		}
	}
}

// sleep waits for the specified duration or until context is cancelled
func sleep(ctx context.Context, duration time.Duration) bool {
	select {
	case <-ctx.Done():
		return false
	case <-time.After(duration):
		return true
	}
}
