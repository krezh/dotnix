package main

import (
	"context"
	"flag"
	"fmt"
	"os"
	"os/exec"
	"os/signal"
	"syscall"
)

var (
	imageDirectory = flag.String("d", "", "directory containing images")
	interval       = flag.Int("i", 0, "seconds to wait between images")
	oneshot        = flag.Bool("o", false, "run once and exit")
	help           = flag.Bool("h", false, "show this help")
	transitionFPS  = flag.String("fps", "", "transition FPS (overrides SWWW_TRANSITION_FPS)")
	transitionStep = flag.String("step", "", "transition step (overrides SWWW_TRANSITION_STEP)")
	transitionType = flag.String("transition", "", "transition type (overrides SWWW_TRANSITION)")
	transitionPos  = flag.String("pos", "", "transition position (overrides SWWW_TRANSITION_POS)")
)

func init() {
	flag.StringVar(imageDirectory, "image-directory", "", "directory containing images")
	flag.IntVar(interval, "interval", 0, "seconds to wait between images")
	flag.BoolVar(oneshot, "oneshot", false, "run once and exit")
	flag.BoolVar(help, "help", false, "show this help")
	flag.StringVar(transitionFPS, "transition-fps", "", "transition FPS (overrides SWWW_TRANSITION_FPS)")
	flag.StringVar(transitionStep, "transition-step", "", "transition step (overrides SWWW_TRANSITION_STEP)")
	flag.StringVar(transitionType, "transition-type", "", "transition type (overrides SWWW_TRANSITION)")
	flag.StringVar(transitionPos, "transition-pos", "", "transition position (overrides SWWW_TRANSITION_POS)")
}

func main() {
	flag.Parse()

	if *help {
		showHelp()
		return
	}

	if err := validateArgs(); err != nil {
		fmt.Printf("Error: %v\n", err)
		os.Exit(1)
	}

	if err := checkDependencies(); err != nil {
		fmt.Printf("Error: %v\n", err)
		os.Exit(1)
	}

	printConfig()

	if *oneshot {
		runOneshot()
		return
	}

	fmt.Println("Starting slideshow...")
	runSlideshow()
}

func showHelp() {
	fmt.Printf("Usage: %s -d <image-directory> [-i <interval>] [-o] [transition options]\n", os.Args[0])
	fmt.Println("  -d, --image-directory    directory containing images")
	fmt.Println("  -i, --interval           seconds to wait between images (required for slideshow mode)")
	fmt.Println("  -o, --oneshot            run once and exit")
	fmt.Println("  -h, --help               show this help")
	fmt.Println("Transition options:")
	fmt.Println("  --fps, --transition-fps  transition FPS (overrides SWWW_TRANSITION_FPS)")
	fmt.Println("  --step, --transition-step transition step (overrides SWWW_TRANSITION_STEP)")
	fmt.Println("  --transition, --transition-type transition type (overrides SWWW_TRANSITION)")
	fmt.Println("  --pos, --transition-pos  transition position (overrides SWWW_TRANSITION_POS)")
	fmt.Println("                           use 'mouse' to automatically use current mouse position")
}

func validateArgs() error {
	if *imageDirectory == "" {
		return fmt.Errorf("image directory is required")
	}

	if !*oneshot && *interval <= 0 {
		return fmt.Errorf("interval is required for slideshow mode")
	}

	if _, err := os.Stat(*imageDirectory); os.IsNotExist(err) {
		return fmt.Errorf("image directory '%s' does not exist", *imageDirectory)
	}

	return nil
}

func checkDependencies() error {
	if _, err := exec.LookPath("swww"); err != nil {
		return fmt.Errorf("swww command not found. Please install swww")
	}

	if err := exec.Command("swww", "query").Run(); err != nil {
		return fmt.Errorf("swww daemon is not running. Please start it with 'swww init'")
	}

	return nil
}

func printConfig() {
	fmt.Printf("Image directory: %s\n", *imageDirectory)
	if *oneshot {
		fmt.Println("Mode: Oneshot")
	} else {
		fmt.Printf("Interval: %d seconds\n", *interval)
		fmt.Println("Mode: Slideshow")
	}
	fmt.Printf("Transition FPS: %s\n", getTransitionValue(*transitionFPS, "SWWW_TRANSITION_FPS", "30"))
	fmt.Printf("Transition Step: %s\n", getTransitionValue(*transitionStep, "SWWW_TRANSITION_STEP", "90"))
	fmt.Printf("Transition Type: %s\n", getTransitionValue(*transitionType, "SWWW_TRANSITION", "any"))
	fmt.Printf("Transition Position: %s\n", getTransitionValue(*transitionPos, "SWWW_TRANSITION_POS", "center"))
}

func runOneshot() {
	images := findImages(*imageDirectory)
	if len(images) == 0 {
		fmt.Printf("Error: No images found in %s\n", *imageDirectory)
		os.Exit(1)
	}

	randomImage := selectRandomImage(images)
	displayImage(randomImage, getTransitionOptions())
}

func runSlideshow() {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Handle graceful shutdown
	go func() {
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)
		<-sigChan
		fmt.Println("\nShutting down...")
		cancel()
	}()

	startSlideshow(ctx, *imageDirectory, *interval, getTransitionOptions())
}

func getTransitionOptions() TransitionOptions {
	return TransitionOptions{
		FPS:  getTransitionValue(*transitionFPS, "SWWW_TRANSITION_FPS", ""),
		Step: getTransitionValue(*transitionStep, "SWWW_TRANSITION_STEP", ""),
		Type: getTransitionValue(*transitionType, "SWWW_TRANSITION", ""),
		Pos:  getTransitionValue(*transitionPos, "SWWW_TRANSITION_POS", ""),
	}
}
