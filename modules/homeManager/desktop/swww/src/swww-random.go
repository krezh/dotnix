package main

import (
	"context"
	"flag"
	"fmt"
	"math/rand"
	"os"
	"os/exec"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

var (
	imageDirectory = flag.String("d", "", "directory containing images")
	interval       = flag.Int("i", 0, "seconds to wait between images")
	help           = flag.Bool("h", false, "show this help")
)

var imageExtensions = []string{
	".jpg", ".jpeg", ".png", ".bmp", ".gif", ".webp",
	".pnm", ".tga", ".tiff", ".farbfeld", ".svg",
}

func init() {
	flag.StringVar(imageDirectory, "image-directory", "", "directory containing images")
	flag.IntVar(interval, "interval", 0, "seconds to wait between images")
	flag.BoolVar(help, "help", false, "show this help")
}

func main() {
	flag.Parse()

	if *help {
		fmt.Printf("Usage: %s -d <image-directory> -i <interval>\n", os.Args[0])
		fmt.Println("  -d, --image-directory  directory containing images")
		fmt.Println("  -i, --interval         seconds to wait between images")
		fmt.Println("  -h, --help             show this help")
		return
	}

	if *imageDirectory == "" || *interval <= 0 {
		fmt.Println("Error: Both image directory and interval are required")
		os.Exit(1)
	}

	if _, err := os.Stat(*imageDirectory); os.IsNotExist(err) {
		fmt.Printf("Error: Image directory '%s' does not exist\n", *imageDirectory)
		os.Exit(1)
	}

	if _, err := exec.LookPath("swww"); err != nil {
		fmt.Println("Error: swww command not found. Please install swww.")
		os.Exit(1)
	}

	if err := exec.Command("swww", "query").Run(); err != nil {
		fmt.Println("Error: swww daemon is not running. Please start it with 'swww init'")
		os.Exit(1)
	}

	fmt.Printf("Image directory: %s\n", *imageDirectory)
	fmt.Printf("Interval: %d seconds\n", *interval)
	fmt.Printf("Transition FPS: %s\n", getEnvWithDefault("SWWW_TRANSITION_FPS", "30"))
	fmt.Printf("Transition Step: %s\n", getEnvWithDefault("SWWW_TRANSITION_STEP", "90"))
	fmt.Printf("Transition Type: %s\n", getEnvWithDefault("SWWW_TRANSITION", "any"))
	fmt.Printf("Transition Position: %s\n", getEnvWithDefault("SWWW_TRANSITION_POS", "center"))
	fmt.Println("Starting slideshow...")

	// Set up graceful shutdown
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go func() {
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)
		<-sigChan
		fmt.Println("\nShutting down...")
		cancel()
	}()

	runSlideshow(ctx)
}

func runSlideshow(ctx context.Context) {
	for {
		images, err := findImages(*imageDirectory)
		if err != nil || len(images) == 0 {
			fmt.Printf("Warning: No images found, retrying in %d seconds...\n", *interval)
			if !sleep(ctx, time.Duration(*interval)*time.Second) {
				return
			}
			continue
		}

		// Shuffle images
		rand.Shuffle(len(images), func(i, j int) {
			images[i], images[j] = images[j], images[i]
		})

		for _, img := range images {
			select {
			case <-ctx.Done():
				return
			default:
				displayImage(img)
				if !sleep(ctx, time.Duration(*interval)*time.Second) {
					return
				}
			}
		}
	}
}

func findImages(directory string) ([]string, error) {
	var images []string
	extMap := make(map[string]bool)
	for _, ext := range imageExtensions {
		extMap[ext] = true
	}

	err := filepath.Walk(directory, func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}
		if extMap[strings.ToLower(filepath.Ext(path))] {
			images = append(images, path)
		}
		return nil
	})

	return images, err
}

func displayImage(imagePath string) {
	fmt.Printf("Displaying: %s\n", filepath.Base(imagePath))
	if err := exec.Command("swww", "img", imagePath).Run(); err != nil {
		fmt.Printf("Warning: Failed to display %s\n", filepath.Base(imagePath))
	}
}

func getEnvWithDefault(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

func sleep(ctx context.Context, duration time.Duration) bool {
	select {
	case <-ctx.Done():
		return false
	case <-time.After(duration):
		return true
	}
}
