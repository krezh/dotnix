package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"recshot/internal/capture"
	"recshot/internal/config"
	"recshot/internal/notify"
	"recshot/internal/process"
	"recshot/internal/upload"
)

func main() {
	cfg := config.New()

	// Track which flags were explicitly set
	urlSet := false
	tokenSet := false

	// Define flags
	flag.StringVar(&cfg.ZiplineURL, "url", cfg.ZiplineURL, "Zipline URL")
	flag.StringVar(&cfg.ZiplineURL, "u", cfg.ZiplineURL, "Zipline URL (short)")
	flag.StringVar(&cfg.TokenFile, "token", cfg.TokenFile, "Token file path")
	flag.StringVar(&cfg.TokenFile, "t", cfg.TokenFile, "Token file path (short)")
	flag.StringVar(&cfg.Mode, "mode", cfg.Mode, "Mode: image-area, image-window, image-screen, video-area, video-window, video-screen")
	flag.StringVar(&cfg.Mode, "m", cfg.Mode, "Mode (short)")
	flag.StringVar(&cfg.SavePath, "path", cfg.SavePath, "Save path")
	flag.StringVar(&cfg.SavePath, "p", cfg.SavePath, "Save path (short)")
	flag.BoolVar(&cfg.UseOriginalName, "original-name", cfg.UseOriginalName, "Use original filename")
	flag.BoolVar(&cfg.UseOriginalName, "o", cfg.UseOriginalName, "Use original filename (short)")
	flag.BoolVar(&cfg.Status, "status", cfg.Status, "Show recording status")

	help := flag.Bool("help", false, "Show help")
	flag.BoolVar(help, "h", false, "Show help (short)")

	flag.Usage = func() {
		fmt.Fprintf(os.Stderr, "recshot - Screenshot and recording tool with optional Zipline upload\n\n")
		fmt.Fprintf(os.Stderr, "USAGE:\n")
		fmt.Fprintf(os.Stderr, "  %s [OPTIONS]\n\n", os.Args[0])

		fmt.Fprintf(os.Stderr, "MODES:\n")
		fmt.Fprintf(os.Stderr, "  Image modes:\n")
		fmt.Fprintf(os.Stderr, "    image-area     Select area to screenshot\n")
		fmt.Fprintf(os.Stderr, "    image-window   Screenshot active window\n")
		fmt.Fprintf(os.Stderr, "    image-screen   Screenshot entire screen\n\n")
		fmt.Fprintf(os.Stderr, "  Video modes:\n")
		fmt.Fprintf(os.Stderr, "    video-area     Record selected area\n")
		fmt.Fprintf(os.Stderr, "    video-window   Record active window\n")
		fmt.Fprintf(os.Stderr, "    video-screen   Record entire screen\n\n")

		fmt.Fprintf(os.Stderr, "OPTIONS:\n")
		fmt.Fprintf(os.Stderr, "  -m, --mode MODE        Capture mode (required, see modes above)\n")
		fmt.Fprintf(os.Stderr, "  -p, --path PATH        Save directory (default: /tmp)\n")
		fmt.Fprintf(os.Stderr, "  -o, --original-name    Use original filename (default: true)\n")
		fmt.Fprintf(os.Stderr, "      --status           Show recording status\n")
		fmt.Fprintf(os.Stderr, "  -h, --help             Show this help message\n\n")

		fmt.Fprintf(os.Stderr, "ZIPLINE OPTIONS:\n")
		fmt.Fprintf(os.Stderr, "  -u, --url URL          Zipline server URL (both --url and --token are required for upload)\n")
		fmt.Fprintf(os.Stderr, "  -t, --token FILE       Zipline token file path (default: ~/.config/zipline/token)\n\n")

		fmt.Fprintf(os.Stderr, "EXAMPLES:\n")
		fmt.Fprintf(os.Stderr, "  %s -m image-area                    # Select area to screenshot\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "  %s -m image-screen -p ~/Pictures   # Full screenshot to ~/Pictures\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "  %s -m video-area -u https://your.zipline.com -t ~/.config/zipline/token  # Record and upload\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "  %s --status                         # Check recording status\n", os.Args[0])
	}

	flag.Parse()

	// Check which flags were actually set
	flag.Visit(func(f *flag.Flag) {
		if f.Name == "url" || f.Name == "u" {
			urlSet = true
		}
		if f.Name == "token" || f.Name == "t" {
			tokenSet = true
		}
	})

	if *help {
		flag.Usage()
		return
	}

	// Validate configuration
	if !cfg.Status && cfg.Mode == "" {
		fmt.Fprintf(os.Stderr, "Error: mode is required when not using status\n\n")
		flag.Usage()
		os.Exit(1)
	}

	// Check if both URL and token are provided when either is set
	if urlSet || tokenSet {
		if !urlSet {
			fmt.Fprintf(os.Stderr, "Error: Both --url and --token are required for Zipline upload. Missing --url\n\n")
			os.Exit(1)
		}
		if !tokenSet {
			fmt.Fprintf(os.Stderr, "Error: Both --url and --token are required for Zipline upload. Missing --token\n\n")
			os.Exit(1)
		}
		cfg.Zipline = true
	}

	run(cfg)
}

func run(cfg *config.Config) {
	// Handle status check
	if cfg.Status {
		isRecording, outputFile := getRecordingStatus()
		if isRecording {
			fmt.Printf("🔴 Recording in progress\n")
			if outputFile != "" {
				fmt.Printf("📁 Output: %s\n", outputFile)
			}
			fmt.Println("Run recshot again to stop")
		} else {
			fmt.Println("⏹️ No recording active")
		}
		return
	}

	// Validate configuration
	if err := cfg.Validate(); err != nil {
		log.Fatal(err)
	}

	// Check dependencies
	if err := checkDependencies(cfg.Dependencies); err != nil {
		log.Fatal(err)
	}

	// Create save directory
	if err := os.MkdirAll(cfg.SavePath, 0755); err != nil {
		log.Fatalf("Failed to create save directory: %v", err)
	}

	// Initialize components
	notifier := notify.New()
	procMgr := process.New()
	capturer := capture.New(cfg, notifier, procMgr)

	// Handle video recording check
	if cfg.IsVideoMode() {
		if outputFile, wasRecording, err := capturer.CheckRecording(); err != nil {
			log.Fatal(err)
		} else if wasRecording {
			if outputFile != "" {
				fmt.Printf("Recording saved to: %s\n", outputFile)
				if cfg.Zipline {
					handleUpload(cfg, notifier, outputFile)
				}
			}
			os.Exit(0)
		}
	}

	// Generate filename
	filename := filepath.Join(cfg.SavePath, time.Now().Format("2006-01-02_15-04-05")+cfg.GetFileExtension())

	// Capture based on mode
	if cfg.IsImageMode() {
		if err := capturer.TakeScreenshot(filename); err != nil {
			notifier.SendError("Failed to take screenshot")
			log.Fatal(err)
		}
		fmt.Printf("Screenshot saved: %s\n", filename)
		notifier.SendSuccess("Screenshot saved")

		if cfg.Zipline {
			handleUpload(cfg, notifier, filename)
		}
	} else if cfg.IsVideoMode() {
		if err := capturer.StartRecording(filename); err != nil {
			log.Fatal(err)
		}
	}
}

func handleUpload(cfg *config.Config, notifier *notify.Notifier, filename string) {
	uploader := upload.NewZiplineUploader(cfg, notifier)
	_, err := uploader.Upload(filename)
	if err != nil {
		log.Printf("Upload failed: %v", err)
		// Fallback to copying image to clipboard
		if err := copyImageToClipboard(filename); err != nil {
			log.Printf("Failed to copy image to clipboard: %v", err)
		} else {
			notifier.SendSuccess("Upload failed - image copied to clipboard")
		}
		return
	}
}

func checkDependencies(deps []string) error {
	for _, dep := range deps {
		if _, err := exec.LookPath(dep); err != nil {
			return fmt.Errorf("%s not found", dep)
		}
	}
	return nil
}

// getRecordingStatus returns recording status and output file
func getRecordingStatus() (bool, string) {
	procMgr := process.New()
	pids, err := procMgr.FindByName("wl-screenrec")
	if err != nil || len(pids) == 0 {
		return false, ""
	}

	// Find output file from first process
	for _, pid := range pids {
		if cmdline, err := procMgr.ReadCmdline(pid); err == nil {
			args := strings.Fields(cmdline)
			for i, arg := range args {
				if arg == "-f" && i+1 < len(args) {
					return true, args[i+1]
				}
			}
		}
	}

	return true, ""
}
